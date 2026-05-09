use axum::{
    extract::{State, Query},
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::AppState;
use crate::db;
use crate::types::{AuthUser, WikiProject};

#[derive(Deserialize)]
pub struct GetConfigQuery {
    key: String,
}

/// Get config value by key
pub async fn get_config(
    State(state): State<AppState>,
    Query(query): Query<GetConfigQuery>,
) -> Result<Json<Value>, String> {
    let value: Option<Value> = db::get_config::<Value>(&state.db, &query.key)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(value.unwrap_or(Value::Null)))
}

#[derive(Deserialize)]
pub struct SetConfigBody {
    key: String,
    value: Value,
}

/// Set config value (admin only)
pub async fn set_config(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(body): Json<SetConfigBody>,
) -> Result<(), String> {
    db::set_config(&state.db, &body.key, &body.value)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get recent projects from database (user-scoped)
pub async fn get_projects(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<WikiProject>>, String> {
    let projects = if auth_user.is_admin() {
        db::get_all_projects(&state.db).await
    } else {
        db::get_user_projects(&state.db, &auth_user.user_id).await
    }
    .map_err(|e| e.to_string())?;

    Ok(Json(projects))
}

#[derive(Deserialize)]
pub struct AddProjectBody {
    project: WikiProject,
}

/// Add project to recent list
pub async fn add_project(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<AddProjectBody>,
) -> Result<(), String> {
    db::add_to_recent_projects(&state.db, &body.project, Some(&auth_user.user_id))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Deserialize)]
pub struct RemoveProjectBody {
    path: String,
}

/// Remove project from recent list (admin only)
pub async fn remove_project(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<RemoveProjectBody>,
) -> Result<(), String> {
    if !auth_user.is_admin() {
        return Err("Admin privileges required".to_string());
    }
    db::remove_project(&state.db, &body.path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get server configuration (LLM, embedding, etc.) for frontend
pub async fn get_server_config(
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let config = &state.config;

    Ok(Json(serde_json::json!({
        "llm": config.llm_config_json(),
        "embedding": config.embedding_config_json(),
        "search": {
            "enabled": config.search.enabled,
            "provider": config.search.provider,
            "apiKey": config.search.api_key,
        },
    })))
}

/// Get last opened project (full WikiProject if exists in database)
pub async fn get_last_project(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Option<WikiProject>>, String> {
    if auth_user.is_admin() {
        let project = db::get_last_project(&state.db)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(Json(project));
    }

    let projects = db::get_user_projects(&state.db, &auth_user.user_id)
        .await
        .map_err(|e| e.to_string())?;

    let last = projects.into_iter().next();
    Ok(Json(last))
}

// ── Admin endpoints ──────────────────────────────────────────────

/// List all users (admin only)
pub async fn list_users(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<crate::types::User>>, String> {
    if !auth_user.is_admin() {
        return Err("Admin privileges required".to_string());
    }
    let users = db::get_all_users(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(users))
}

#[derive(Deserialize)]
pub struct AssignProjectBody {
    pub project_id: String,
    pub user_id: String,
}

/// Assign a project to a user and move its directory (admin only)
pub async fn assign_project_to_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<AssignProjectBody>,
) -> Result<(), String> {
    if !auth_user.is_admin() {
        return Err("Admin privileges required".to_string());
    }

    let target_user = db::get_user_by_id(&state.db, &body.user_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Target user not found".to_string())?;

    let project = db::get_project_by_id(&state.db, &body.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    let old_path = std::path::PathBuf::from(&project.path);
    let project_name = old_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let new_path = state.config.server.projects_dir
        .join(&target_user.id)
        .join(project_name);

    // Move directory on disk
    if old_path.exists() && old_path != new_path {
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create target directory: {}", e))?;
        }
        std::fs::rename(&old_path, &new_path)
            .map_err(|e| format!("Failed to move project directory: {}", e))?;
    }

    // Update database
    db::assign_project_user(&state.db, &new_path.to_string_lossy(), &target_user.id)
        .await
        .map_err(|e| e.to_string())?;

    let new_path_str = new_path.to_string_lossy().to_string();
    sqlx::query("UPDATE projects SET path = ? WHERE id = ?")
        .bind(&new_path_str)
        .bind(&project.id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Failed to update project path: {}", e))?;

    tracing::info!(
        "Admin '{}' assigned project '{}' to user '{}'",
        auth_user.username, project.name, target_user.username
    );

    Ok(())
}

// ── Admin user management ────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateUserBody {
    pub username: String,
    pub password: String,
    pub role: String,
}

/// Create a new user (admin only)
pub async fn create_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<CreateUserBody>,
) -> Result<Json<crate::types::User>, String> {
    if !auth_user.is_admin() {
        return Err("Admin privileges required".to_string());
    }
    if body.role != crate::types::ROLE_ADMIN && body.role != crate::types::ROLE_USER {
        return Err(format!("Invalid role '{}'. Must be 'admin' or 'user'.", body.role));
    }
    if db::get_user_by_username(&state.db, &body.username)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .is_some()
    {
        return Err(format!("User '{}' already exists", body.username));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = crate::auth_pass::hash_password(&body.password)
        .map_err(|e| format!("Failed to hash password: {}", e))?;
    let now = chrono::Utc::now().timestamp();

    let user = db::create_user(&state.db, &id, &body.username, &password_hash, &body.role, now)
        .await
        .map_err(|e| format!("Failed to create user: {}", e))?;

    tracing::info!("Admin '{}' created user '{}' (role: {})", auth_user.username, user.username, user.role);
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct UpdateUserBody {
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

/// Update a user's password and/or role (admin only)
pub async fn update_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<UpdateUserBody>,
) -> Result<Json<crate::types::User>, String> {
    if !auth_user.is_admin() {
        return Err("Admin privileges required".to_string());
    }

    let existing = db::get_user_by_username(&state.db, &body.username)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("User '{}' not found", body.username))?;

    // Validate role if provided
    if let Some(ref role) = body.role {
        if role != crate::types::ROLE_ADMIN && role != crate::types::ROLE_USER {
            return Err(format!("Invalid role '{}'. Must be 'admin' or 'user'.", role));
        }
        // Prevent demoting the last admin
        if existing.role == crate::types::ROLE_ADMIN && role == crate::types::ROLE_USER {
            let admin_count = db::count_users_by_role(&state.db, crate::types::ROLE_ADMIN)
                .await
                .map_err(|e| format!("DB error: {}", e))?;
            if admin_count <= 1 {
                return Err("Cannot remove the last admin".to_string());
            }
        }
    }

    // Hash password if provided
    let password_hash = if let Some(ref pw) = body.password {
        Some(crate::auth_pass::hash_password(pw).map_err(|e| format!("Failed to hash password: {}", e))?)
    } else {
        None
    };

    let new_role = body.role.as_deref().unwrap_or(&existing.role);

    db::update_user_fields(
        &state.db,
        &body.username,
        password_hash.as_deref(),
        body.role.as_deref(),
    )
    .await
    .map_err(|e| format!("Failed to update user: {}", e))?;

    let user = crate::types::User {
        role: new_role.to_string(),
        ..existing
    };

    tracing::info!("Admin '{}' updated user '{}'", auth_user.username, user.username);
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct DeleteUserBody {
    pub username: String,
}

/// Delete a user (admin only)
pub async fn delete_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<DeleteUserBody>,
) -> Result<(), String> {
    if !auth_user.is_admin() {
        return Err("Admin privileges required".to_string());
    }
    if auth_user.username == body.username {
        return Err("Cannot delete yourself".to_string());
    }

    let target = db::get_user_by_username(&state.db, &body.username)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("User '{}' not found", body.username))?;

    // Prevent deleting the last admin
    if target.role == crate::types::ROLE_ADMIN {
        let admin_count = db::count_users_by_role(&state.db, crate::types::ROLE_ADMIN)
            .await
            .map_err(|e| format!("DB error: {}", e))?;
        if admin_count <= 1 {
            return Err("Cannot delete the last admin".to_string());
        }
    }

    // Reassign projects to the current admin before deleting
    db::reassign_user_projects(&state.db, &target.id, &auth_user.user_id)
        .await
        .map_err(|e| format!("Failed to reassign projects: {}", e))?;

    db::delete_user(&state.db, &body.username)
        .await
        .map_err(|e| format!("Failed to delete user: {}", e))?;

    tracing::info!("Admin '{}' deleted user '{}'", auth_user.username, body.username);
    Ok(())
}