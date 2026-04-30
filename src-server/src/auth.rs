use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};

use crate::AppState;

/// Auth middleware for API routes.
///
/// For single-user self-hosted scenario with Caddy basicauth:
/// - Caddy handles authentication at the reverse proxy level
/// - API server trusts all requests (no additional auth needed)
///
/// If you want API-level auth, set a non-empty token in server.toml
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = &state.config.server.token;

    // If no token configured, trust all requests
    // (authentication is handled by Caddy basicauth)
    if token.is_empty() {
        return Ok(next.run(request).await);
    }

    // If token is configured, require it
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