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
    auth_user: AuthUser,
    Json(body): Json<SetConfigBody>,
) -> Result<(), String> {
    if !auth_user.is_admin() {
        return Err("Admin privileges required".to_string());
    }
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