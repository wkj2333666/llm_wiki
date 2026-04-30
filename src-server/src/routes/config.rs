use axum::{
    extract::{State, Query},
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::AppState;
use crate::types::WikiProject;

#[derive(Deserialize)]
pub struct GetConfigQuery {
    key: String,
}

/// Get config value by key (uses query parameter, not JSON body)
pub async fn get_config(
    Query(query): Query<GetConfigQuery>,
) -> Result<Json<Value>, String> {
    // TODO: implement using db::get_config
    Ok(Json(Value::Null))
}

#[derive(Deserialize)]
pub struct SetConfigBody {
    key: String,
    value: Value,
}

/// Set config value
pub async fn set_config(
    Json(_body): Json<SetConfigBody>,
) -> Result<(), String> {
    // TODO: implement using db::set_config
    Ok(())
}

/// Get recent projects
pub async fn get_projects() -> Result<Json<Vec<WikiProject>>, String> {
    // TODO: implement using db::get_projects
    Ok(Json(Vec::new()))
}

#[derive(Deserialize)]
pub struct AddProjectBody {
    project: WikiProject,
}

/// Add project to recent list
pub async fn add_project(
    Json(_body): Json<AddProjectBody>,
) -> Result<(), String> {
    // TODO: implement using db::add_to_recent_projects
    Ok(())
}

#[derive(Deserialize)]
pub struct RemoveProjectBody {
    path: String,
}

/// Remove project from recent list
pub async fn remove_project(
    Json(_body): Json<RemoveProjectBody>,
) -> Result<(), String> {
    // TODO: implement using db::remove_project
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