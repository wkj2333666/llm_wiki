use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct ClipStatus {
    status: String,
}

/// Get clip server status
pub async fn status() -> Json<ClipStatus> {
    // TODO: integrate with clip_server module
    // The clip server runs independently on port 19827
    Json(ClipStatus {
        status: "running".to_string(),
    })
}