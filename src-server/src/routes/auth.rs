use axum::{extract::State, Json, http::StatusCode};

use crate::auth_jwt;
use crate::auth_pass;
use crate::db;
use crate::types::{AuthUser, LoginRequest, TokenResponse};

/// Login route — verifies credentials and returns a JWT.
pub async fn login(
    State(state): State<crate::AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, (StatusCode, String)> {
    let (user, password_hash) = db::get_user_with_hash(&state.db, &body.username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()))?;

    let valid = auth_pass::verify_password(&body.password, &password_hash)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()));
    }

    let token = auth_jwt::create_token(&user.id, &user.username, &user.role, &state.jwt_secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    tracing::info!("User '{}' logged in", user.username);

    Ok(Json(TokenResponse { token, user }))
}

/// Get current user info from JWT.
pub async fn me(
    auth_user: AuthUser,
    State(state): State<crate::AppState>,
) -> Result<Json<crate::types::User>, (StatusCode, String)> {
    let user = db::get_user_by_id(&state.db, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    Ok(Json(user))
}
