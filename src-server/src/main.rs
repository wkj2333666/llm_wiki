mod auth;
mod config;
mod db;
mod panic_guard;
mod routes;
mod services;
mod types;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post, delete},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: Arc<AppConfig>,
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

    // Build state
    let state = AppState {
        db,
        config: Arc::new(config.clone()),
    };

    // Build router with auth layer
    let api_routes = Router::new()
        // File system
        .route("/fs/read", get(routes::fs::read_file))
        .route("/fs/write", post(routes::fs::write_file))
        .route("/fs/list", get(routes::fs::list_directory))
        .route("/fs/copy", post(routes::fs::copy_file))
        .route("/fs/copy-dir", post(routes::fs::copy_directory))
        .route("/fs/preprocess", post(routes::fs::preprocess_file))
        .route("/fs/delete", delete(routes::fs::delete_file))
        .route("/fs/create-dir", post(routes::fs::create_directory))
        .route("/fs/exists", get(routes::fs::file_exists))
        .route("/fs/read-base64", get(routes::fs::read_file_as_base64))
        .route("/fs/file", get(routes::fs::serve_file))
        .route("/fs/extract-images", post(routes::fs::extract_images))
        // Project
        .route("/project/list", get(routes::project::list_projects))
        .route("/project/create", post(routes::project::create_project))
        .route("/project/open", post(routes::project::open_project))
        .route("/project/open-path", post(routes::project::open_project_by_path))
        // Vector store
        .route("/vector/upsert", post(routes::vector::upsert_chunks))
        .route("/vector/search", post(routes::vector::search_chunks))
        .route("/vector/delete", post(routes::vector::delete_page))
        .route("/vector/count", get(routes::vector::count_chunks))
        // Claude CLI
        .route("/claude/detect", get(routes::claude::detect))
        .route("/claude/spawn", post(routes::claude::spawn))
        .route("/claude/kill", post(routes::claude::kill))
        // Config
        .route("/config/get", get(routes::config::get_config))
        .route("/config/set", post(routes::config::set_config))
        .route("/config/projects", get(routes::config::get_projects))
        .route("/config/projects/add", post(routes::config::add_project))
        .route("/config/projects/remove", delete(routes::config::remove_project))
        // Server config (for frontend)
        .route("/config/server", get(routes::config::get_server_config))
        // Wiki related pages
        .route("/wiki/related", get(routes::wiki::find_related_pages))
        // Clip server status
        .route("/clip/status", get(routes::clip::status))
        // Web search
        .route("/search/web", post(routes::search::web_search))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth::auth_middleware));

    let app = Router::new()
        .nest("/api", api_routes)
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