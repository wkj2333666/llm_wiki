use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};

use crate::AppState;

/// Auth middleware for API routes.
/// Requires Bearer token matching the token from config.
/// If no token is configured, all requests are allowed (development mode).
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = &state.config.server.token;

    // If no token configured, allow all requests (development mode)
    if token.is_empty() {
        return Ok(next.run(request).await);
    }

    // Also allow token passed as query param (for file serving in browser)
    let query_token = request.uri().query()
        .and_then(|q| {
            q.split('&')
                .find(|p| p.starts_with("token="))
                .map(|p| p.strip_prefix("token=").unwrap_or(""))
        });

    if query_token == Some(token.as_str()) {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        Some(t) if t == token => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}