mod auth;
mod auth_jwt;
mod auth_pass;
mod config;
mod db;
mod panic_guard;
mod routes;
mod services;
mod types;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post, delete},
    Router,
};
use tokio::process::Child;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::AppConfig;

/// Shared state holding running Claude child processes keyed by stream_id
#[derive(Default, Clone)]
pub struct ClaudeProcessState {
    children: Arc<Mutex<HashMap<String, Child>>>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: Arc<AppConfig>,
    pub jwt_secret: Arc<String>,
    pub claude_processes: ClaudeProcessState,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config from file + env vars
    let config = AppConfig::load();
    tracing::info!("Server port: {}", config.server.port);
    tracing::info!("Data directory: {:?}", config.server.data_dir);
    tracing::info!("Static directory: {:?}", config.server.static_dir);
    tracing::info!("LLM provider: {} ({})", config.llm.provider, config.llm.model);

    // Ensure data directory exists
    std::fs::create_dir_all(&config.server.data_dir).expect("Failed to create data directory");

    // Ensure projects directory exists
    std::fs::create_dir_all(&config.server.projects_dir).expect("Failed to create projects directory");

    // Initialize database
    let db_path = config.server.data_dir.join("app.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    tracing::info!("Database URL: {}", db_url);

    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    db::run_migrations(&db)
        .await
        .expect("Failed to run migrations");

    // Resolve JWT secret: config > persisted in DB > random generated
    let jwt_secret = resolve_jwt_secret(&db, &config).await;

    // Bootstrap users from config
    db::bootstrap_users(&db, &config.server.users, &config.server.projects_dir)
        .await
        .expect("Failed to bootstrap users");

    // Build state
    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        jwt_secret: Arc::new(jwt_secret),
        claude_processes: ClaudeProcessState::default(),
    };

    // Auth routes (no authentication required)
    let auth_routes = Router::new()
        .route("/api/auth/login", post(routes::auth::login));

    // Protected API routes (authentication required)
    let api_routes = Router::new()
        // Auth (authenticated only)
        .route("/api/auth/me", get(routes::auth::me))
        // File system
        .route("/api/fs/read", get(routes::fs::read_file))
        .route("/api/fs/write", post(routes::fs::write_file))
        .route("/api/fs/list", get(routes::fs::list_directory))
        .route("/api/fs/copy", post(routes::fs::copy_file))
        .route("/api/fs/copy-dir", post(routes::fs::copy_directory))
        .route("/api/fs/preprocess", post(routes::fs::preprocess_file))
        .route("/api/fs/delete", delete(routes::fs::delete_file))
        .route("/api/fs/create-dir", post(routes::fs::create_directory))
        .route("/api/fs/exists", get(routes::fs::file_exists))
        .route("/api/fs/read-base64", get(routes::fs::read_file_as_base64))
        .route("/api/fs/file", get(routes::fs::serve_file))
        .route("/api/fs/extract-images", post(routes::fs::extract_images))
        .route("/api/fs/upload", post(routes::fs::upload_files))
        // Project
        .route("/api/project/list", get(routes::project::list_projects))
        .route("/api/project/create", post(routes::project::create_project))
        .route("/api/project/open", post(routes::project::open_project))
        .route("/api/project/open-path", post(routes::project::open_project_by_path))
        // Vector store
        .route("/api/vector/upsert", post(routes::vector::upsert_chunks))
        .route("/api/vector/search", post(routes::vector::search_chunks))
        .route("/api/vector/delete", post(routes::vector::delete_page))
        .route("/api/vector/count", get(routes::vector::count_chunks))
        // Claude CLI
        .route("/api/claude/detect", get(routes::claude::detect))
        .route("/api/claude/spawn", post(routes::claude::spawn))
        .route("/api/claude/kill", post(routes::claude::kill))
        // Config
        .route("/api/config/get", get(routes::config::get_config))
        .route("/api/config/set", post(routes::config::set_config))
        .route("/api/config/projects", get(routes::config::get_projects))
        .route("/api/config/projects/add", post(routes::config::add_project))
        .route("/api/config/projects/remove", delete(routes::config::remove_project))
        .route("/api/config/last-project", get(routes::config::get_last_project))
        // Admin
        .route("/api/admin/users", get(routes::config::list_users))
        .route("/api/admin/projects/assign", post(routes::config::assign_project_to_user))
        // Server config (for frontend)
        .route("/api/config/server", get(routes::config::get_server_config))
        // Wiki related pages
        .route("/api/wiki/related", get(routes::wiki::find_related_pages))
        // Clip server status
        .route("/api/clip/status", get(routes::clip::status))
        // Web search
        .route("/api/search/web", post(routes::search::web_search))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth::auth_middleware));

    let app = Router::new()
        .merge(auth_routes)
        .merge(api_routes)
        .fallback_service(ServeDir::new(&config.server.static_dir).fallback(ServeDir::new(&config.server.static_dir).append_index_html_on_directories(true)))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Resolve JWT secret:
/// 1. Use explicit config value if set
/// 2. Try to load persisted secret from app_config table
/// 3. Generate a random one and persist it for next startup
async fn resolve_jwt_secret(db: &sqlx::SqlitePool, config: &AppConfig) -> String {
    // Explicit config takes priority
    if !config.server.jwt_secret.is_empty() {
        tracing::info!("Using JWT secret from config");
        return config.server.jwt_secret.clone();
    }

    // Try to load from persisted storage
    let persisted: Option<String> = db::get_config::<String>(db, "jwt_secret")
        .await
        .ok()
        .flatten();

    if let Some(ref secret) = persisted {
        if !secret.is_empty() {
            tracing::info!("Using persisted JWT secret");
            return secret.clone();
        }
    }

    // Generate new secret and persist it
    let raw: [u8; 32] = rand::random();
    let secret: String = raw.iter().map(|b| format!("{:02x}", b)).collect();
    tracing::warn!("Generated new JWT secret — all existing tokens are now invalid");

    if let Err(e) = db::set_config(db, "jwt_secret", &secret).await {
        tracing::warn!("Failed to persist JWT secret: {}. Tokens will be invalid on restart.", e);
    }

    secret
}
