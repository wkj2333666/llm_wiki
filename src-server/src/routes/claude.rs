use axum::{
    extract::State,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream as StreamTrait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::services::claude::{self, ClaudeMessage};

#[derive(Serialize)]
pub struct DetectResult {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub error: Option<String>,
}

/// Detect Claude CLI installation
pub async fn detect() -> Result<Json<DetectResult>, String> {
    let result = claude::detect().await;
    Ok(Json(DetectResult {
        installed: result.installed,
        version: result.version,
        path: result.path,
        error: result.error,
    }))
}

#[derive(Deserialize)]
pub struct SpawnBody {
    stream_id: String,
    model: String,
    messages: Vec<ClaudeMessageBody>,
}

#[derive(Deserialize)]
pub struct ClaudeMessageBody {
    role: String,
    content: String,
}

/// Spawn Claude CLI and stream output via SSE
pub async fn spawn(
    State(state): State<AppState>,
    Json(body): Json<SpawnBody>,
) -> Result<Sse<impl StreamTrait<Item = Result<Event, std::convert::Infallible>>>, String> {
    // Convert messages
    let messages: Vec<ClaudeMessage> = body.messages.into_iter().map(|m| ClaudeMessage {
        role: m.role,
        content: m.content,
    }).collect();

    let children = std::sync::Arc::clone(&state.claude_processes.children);

    let output_stream = claude::spawn(&body.model, &messages, children, body.stream_id).await?;

    // Convert string stream to SSE events
    let sse_stream = output_stream.map(|line| {
        Ok(match line {
            Ok(l) => Event::default().data(l),
            Err(_) => Event::default().data("{\"type\":\"error\"}"),
        })
    });

    Ok(Sse::new(sse_stream))
}

#[derive(Deserialize)]
pub struct KillBody {
    stream_id: String,
}

/// Kill running Claude CLI process
pub async fn kill(
    State(state): State<AppState>,
    Json(body): Json<KillBody>,
) -> Result<(), String> {
    claude::kill(std::sync::Arc::clone(&state.claude_processes.children), &body.stream_id).await?;
    Ok(())
}