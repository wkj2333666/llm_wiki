//! Claude CLI subprocess management
//! Ported from src-tauri/src/commands/claude_cli.rs, adapted for SSE

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Serialize)]
pub struct DetectResult {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

/// Detect Claude CLI installation
pub async fn detect() -> DetectResult {
    let path = match which::which("claude") {
        Ok(p) => p,
        Err(_) => {
            return DetectResult {
                installed: false,
                version: None,
                path: None,
                error: Some("`claude` not found on PATH".to_string()),
            };
        }
    };

    let path_str = path.to_string_lossy().to_string();

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        Command::new(&path).arg("--version").output(),
    )
    .await;

    match output {
        Ok(Ok(out)) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            DetectResult {
                installed: true,
                version: Some(version),
                path: Some(path_str),
                error: None,
            }
        }
        Ok(Ok(out)) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let error = if stderr.contains("quarantine") || stderr.contains("damaged") {
                Some(format!("Binary quarantined — try: xattr -d com.apple.quarantine {}", path_str))
            } else if stderr.is_empty() {
                Some(format!("`claude --version` exited with {}", out.status))
            } else {
                Some(stderr)
            };
            DetectResult {
                installed: false,
                version: None,
                path: Some(path_str),
                error,
            }
        }
        Ok(Err(e)) => DetectResult {
            installed: false,
            version: None,
            path: Some(path_str),
            error: Some(format!("Failed to spawn `claude`: {e}")),
        },
        Err(_) => DetectResult {
            installed: false,
            version: None,
            path: Some(path_str),
            error: Some("`claude --version` timed out after 3s".to_string()),
        },
    }
}

/// Spawn Claude CLI and return a stream of output lines
/// Returns (stdout_lines_stream, stderr_collector, child_handle)
pub async fn spawn(
    model: &str,
    messages: &[ClaudeMessage],
    children: Arc<Mutex<HashMap<String, Child>>>,
    stream_id: String,
) -> Result<impl futures::Stream<Item = Result<String, std::convert::Infallible>>, String> {
    // Build the turn list: fold system messages into preamble on first user turn
    let system_preamble: String = messages
        .iter()
        .filter(|m| m.role == "system")
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n\n");

    let conversation: Vec<&ClaudeMessage> = messages
        .iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .collect();

    if conversation.is_empty() {
        return Err("No user/assistant messages to send to claude CLI".to_string());
    }

    let mut first_user_seen = false;
    let turns: Vec<(String, String)> = conversation
        .iter()
        .map(|m| {
            let role = m.role.clone();
            let mut content = m.content.clone();
            if !first_user_seen && role == "user" && !system_preamble.is_empty() {
                content = format!("{system_preamble}\n\n{content}");
                first_user_seen = true;
            }
            (role, content)
        })
        .collect();

    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--model")
        .arg(model);

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn claude: {e}"))?;

    let mut stdin = child.stdin.take().ok_or_else(|| "Missing stdin handle".to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "Missing stdout handle".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Missing stderr handle".to_string())?;

    // Write turns to stdin
    for (role, content) in &turns {
        let event = serde_json::json!({
            "type": role,
            "message": {
                "role": role,
                "content": [{ "type": "text", "text": content }],
            }
        });
        let line = format!("{}\n", event);
        stdin.write_all(line.as_bytes()).await.map_err(|e| format!("Failed to write to stdin: {e}"))?;
    }
    stdin.flush().await.map_err(|e| format!("Failed to flush stdin: {e}"))?;
    drop(stdin);

    // Register child
    children.lock().await.insert(stream_id.clone(), child);

    // Create stream from stdout
    let stream_id_for_cleanup = stream_id.clone();
    let children_for_cleanup = Arc::clone(&children);
    let stream = async_stream::stream! {
        let mut reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        // Collect stderr in background
        let stderr_task = tokio::spawn(async move {
            let mut collected = String::new();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                tracing::warn!("[claude stderr] {}", line);
                collected.push_str(&line);
                collected.push('\n');
            }
            collected
        });

        // Emit stdout lines
        while let Ok(Some(line)) = reader.next_line().await {
            yield Ok(line);
        }

        // Wait for child to exit and emit final done message
        let child_opt = children_for_cleanup.lock().await.remove(&stream_id_for_cleanup);
        let exit_code = if let Some(mut c) = child_opt {
            c.wait().await.ok().and_then(|s| s.code())
        } else {
            None
        };

        let stderr_text = stderr_task.await.unwrap_or_default();
        let done_msg = serde_json::json!({
            "type": "done",
            "code": exit_code,
            "stderr": stderr_text,
        });
        yield Ok(format!("__done__:{}", serde_json::to_string(&done_msg).unwrap()));
    };

    Ok(stream)
}

/// Kill a running Claude process
pub async fn kill(
    children: Arc<Mutex<HashMap<String, Child>>>,
    stream_id: &str,
) -> Result<(), String> {
    if let Some(mut child) = children.lock().await.remove(stream_id) {
        let _ = child.start_kill();
    }
    Ok(())
}