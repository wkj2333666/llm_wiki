use axum::{
    extract::{State, Query},
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::AppState;
use crate::db;
use crate::types::WikiProject;

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

/// Set config value
pub async fn set_config(
    State(state): State<AppState>,
    Json(body): Json<SetConfigBody>,
) -> Result<(), String> {
    db::set_config(&state.db, &body.key, &body.value)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get recent projects from database
pub async fn get_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<WikiProject>>, String> {
    let projects = db::get_projects(&state.db)
        .await
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
    Json(body): Json<AddProjectBody>,
) -> Result<(), String> {
    db::add_to_recent_projects(&state.db, &body.project)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Deserialize)]
pub struct RemoveProjectBody {
    path: String,
}

/// Remove project from recent list
pub async fn remove_project(
    State(state): State<AppState>,
    Json(body): Json<RemoveProjectBody>,
) -> Result<(), String> {
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
) -> Result<Json<Option<WikiProject>>, String> {
    let project = db::get_last_project(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(project))
}