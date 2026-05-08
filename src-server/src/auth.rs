use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};

use crate::auth_jwt;
use crate::types::AuthUser;
use crate::AppState;

/// Auth middleware for API routes.
///
/// Two authentication methods are supported:
/// 1. Legacy server token — matches `server.token`; runs as service admin
/// 2. JWT bearer token — validates and extracts user identity
///
/// If neither succeeds, 401 is returned.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = &state.config.server.token;

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        // Legacy server token: service admin
        Some(t) if !token.is_empty() && t == token => {
            tracing::info!("Auth: legacy server token accepted");
            request.extensions_mut().insert(AuthUser::service_admin());
            Ok(next.run(request).await)
        }
        // JWT token
        Some(t) => {
            let short = if t.len() > 16 { format!("{}...", &t[..16]) } else { t.to_string() };
            match auth_jwt::validate_token(t, &state.jwt_secret) {
                Ok(claims) => {
                    tracing::info!("Auth: JWT valid for user '{}' (role: {})", claims.username, claims.role);
                    request.extensions_mut().insert(AuthUser::from_claims(
                        claims.sub,
                        claims.username,
                        claims.role,
                    ));
                    Ok(next.run(request).await)
                }
                Err(e) => {
                    tracing::warn!("Auth: JWT validation failed (token prefix: {}): {}", short, e);
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        }
        None => {
            let method = request.method().to_string();
            let uri = request.uri().to_string();
            tracing::warn!("Auth: no Authorization header on {} {}", method, uri);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
