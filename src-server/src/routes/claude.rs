use axum::{
    response::sse::{Event, Sse},
    Json,
};
use async_stream::stream;
use futures::stream::Stream as StreamTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct DetectResult {
    installed: bool,
    version: Option<String>,
    path: Option<String>,
    error: Option<String>,
}

/// Detect Claude CLI installation
pub async fn detect() -> Result<Json<DetectResult>, String> {
    // TODO: implement using services::claude::detect
    let path = which::which("claude").ok();
    match path {
        Some(p) => {
            Ok(Json(DetectResult {
                installed: true,
                version: Some("unknown".to_string()),
                path: Some(p.to_string_lossy().to_string()),
                error: None,
            }))
        }
        None => {
            Ok(Json(DetectResult {
                installed: false,
                version: None,
                path: None,
                error: Some("`claude` not found on PATH".to_string()),
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct SpawnBody {
    stream_id: String,
    model: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Deserialize)]
pub struct ClaudeMessage {
    role: String,
    content: String,
}

/// Spawn Claude CLI and stream output via SSE
pub async fn spawn(
    Json(_body): Json<SpawnBody>,
) -> Sse<impl StreamTrait<Item = Result<Event, std::convert::Infallible>>> {
    // TODO: implement using services::claude::spawn
    // Placeholder: just return an empty stream
    let stream = stream! {
        yield Ok(Event::default().data("{\"type\":\"error\",\"message\":\"Not implemented yet\"}"));
    };
    Sse::new(stream)
}

#[derive(Deserialize)]
pub struct KillBody {
    stream_id: String,
}

/// Kill running Claude CLI process
pub async fn kill(
    Json(_body): Json<KillBody>,
) -> Result<(), String> {
    // TODO: implement using services::claude::kill
    Ok(())
}