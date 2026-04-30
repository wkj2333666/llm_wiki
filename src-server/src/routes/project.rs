use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::types::WikiProject;

#[derive(Deserialize)]
pub struct CreateProjectBody {
    name: String,
    #[serde(default)]
    path: Option<String>,  // Optional: if empty, use server's projects_dir
}

/// Create a new wiki project
/// If path is empty, uses server's configured projects_dir
pub async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<WikiProject>, String> {
    // Use server's projects_dir if no path specified
    let base_path = body.path
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|| state.config.server.projects_dir.clone());

    let root = base_path.join(&body.name);

    if root.exists() {
        return Err(format!("Directory already exists: '{}'", root.display()));
    }

    // Create directory structure
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    // Create wiki subdirectories
    std::fs::create_dir_all(root.join("wiki"))
        .map_err(|e| format!("Failed to create wiki directory: {}", e))?;
    std::fs::create_dir_all(root.join("raw/sources"))
        .map_err(|e| format!("Failed to create sources directory: {}", e))?;

    let id = uuid::Uuid::new_v4().to_string();
    let project = WikiProject {
        id,
        name: body.name,
        path: root.to_string_lossy().to_string(),
    };

    tracing::info!("Created project '{}' at '{}'", project.name, project.path);

    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct OpenProjectBody {
    name: String,  // Project name, will look in projects_dir
}

/// Open an existing wiki project by name
pub async fn open_project(
    State(state): State<AppState>,
    Json(body): Json<OpenProjectBody>,
) -> Result<Json<WikiProject>, String> {
    let root = state.config.server.projects_dir.join(&body.name);

    if !root.exists() {
        return Err(format!("Project '{}' does not exist", body.name));
    }
    if !root.is_dir() {
        return Err(format!("'{}' is not a directory", body.name));
    }

    // Validate that this looks like a wiki project
    if !root.join("wiki").exists() {
        return Err(format!(
            "'{}' is not a valid wiki project (missing wiki folder)",
            body.name
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let project = WikiProject {
        id,
        name: body.name,
        path: root.to_string_lossy().to_string(),
    };

    tracing::info!("Opened project '{}' at '{}'", project.name, project.path);

    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct OpenProjectByPathBody {
    path: String,
}

/// Open an existing wiki project by full path (for projects outside projects_dir)
pub async fn open_project_by_path(
    Json(body): Json<OpenProjectByPathBody>,
) -> Result<Json<WikiProject>, String> {
    let root = std::path::PathBuf::from(&body.path);

    if !root.exists() {
        return Err(format!("Path does not exist: '{}'", body.path));
    }
    if !root.is_dir() {
        return Err(format!("Path is not a directory: '{}'", body.path));
    }

    // Validate that this looks like a wiki project
    if !root.join("wiki").exists() {
        return Err(format!(
            "Not a valid wiki project (missing wiki folder): '{}'",
            body.path
        ));
    }

    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let project = WikiProject {
        id,
        name,
        path: root.to_string_lossy().to_string(),
    };

    tracing::info!("Opened project '{}' at '{}'", project.name, project.path);

    Ok(Json(project))
}

#[derive(Serialize)]
pub struct ProjectInfo {
    name: String,
    path: String,
    has_wiki: bool,
}

/// List all projects in the configured projects_dir
pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProjectInfo>>, String> {
    let projects_dir = &state.config.server.projects_dir;

    if !projects_dir.exists() {
        // Create the directory if it doesn't exist
        std::fs::create_dir_all(projects_dir)
            .map_err(|e| format!("Failed to create projects directory: {}", e))?;
        return Ok(Json(Vec::new()));
    }

    let mut projects = Vec::new();

    let entries = std::fs::read_dir(projects_dir)
        .map_err(|e| format!("Failed to read projects directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            // Check if it's a valid wiki project (has wiki folder)
            let has_wiki = path.join("wiki").exists();

            projects.push(ProjectInfo {
                name,
                path: path.to_string_lossy().to_string(),
                has_wiki,
            });
        }
    }

    // Sort by name
    projects.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(projects))
}