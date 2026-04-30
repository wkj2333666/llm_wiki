use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::db;
use crate::types::WikiProject;

#[derive(Deserialize)]
pub struct CreateProjectBody {
    name: String,
    #[serde(default)]
    path: Option<String>,  // Optional: if empty, use server's projects_dir
}

/// Create a new wiki project
/// If path is empty or whitespace, uses server's configured projects_dir
pub async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<WikiProject>, String> {
    // Use server's projects_dir if no path specified or path is empty/whitespace
    let base_path = body.path
        .filter(|p| !p.trim().is_empty())
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

    // Save to database
    db::add_to_recent_projects(&state.db, &project)
        .await
        .map_err(|e| format!("Failed to save project: {}", e))?;

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

    // Check if project exists in database, get existing ID
    let existing = db::get_project_by_path(&state.db, &root.to_string_lossy())
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let project = match existing {
        Some(p) => p,
        None => {
            // New entry for project not yet in database
            let id = uuid::Uuid::new_v4().to_string();
            WikiProject {
                id,
                name: body.name,
                path: root.to_string_lossy().to_string(),
            }
        }
    };

    // Update database (refreshes last_opened_at and recent list order)
    db::add_to_recent_projects(&state.db, &project)
        .await
        .map_err(|e| format!("Failed to update project: {}", e))?;

    tracing::info!("Opened project '{}' at '{}'", project.name, project.path);

    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct OpenProjectByPathBody {
    path: String,
}

/// Open an existing wiki project by full path (for projects outside projects_dir)
pub async fn open_project_by_path(
    State(state): State<AppState>,
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

    // Check if project exists in database
    let existing = db::get_project_by_path(&state.db, &body.path)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let project = match existing {
        Some(p) => p,
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            WikiProject {
                id,
                name,
                path: root.to_string_lossy().to_string(),
            }
        }
    };

    // Update database
    db::add_to_recent_projects(&state.db, &project)
        .await
        .map_err(|e| format!("Failed to update project: {}", e))?;

    tracing::info!("Opened project '{}' at '{}'", project.name, project.path);

    Ok(Json(project))
}

#[derive(Serialize)]
pub struct ProjectInfo {
    name: String,
    path: String,
    has_wiki: bool,
    last_opened_at: Option<i64>,  // Unix timestamp, from database
}

/// List all projects in the configured projects_dir
/// Also includes recent projects from database that may be outside projects_dir
pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProjectInfo>>, String> {
    let projects_dir = &state.config.server.projects_dir;

    // Create the directory if it doesn't exist
    if !projects_dir.exists() {
        std::fs::create_dir_all(projects_dir)
            .map_err(|e| format!("Failed to create projects directory: {}", e))?;
    }

    // Get recent projects from database for ordering and timestamps
    let recent_projects = db::get_projects(&state.db)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Fetch all last_opened_at timestamps in one batch
    let mut last_opened_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for p in &recent_projects {
        if let Some(ts) = db::get_last_opened_at(&state.db, &p.id).await.ok().flatten() {
            last_opened_map.insert(p.id.clone(), ts);
        }
    }

    // Build path -> project lookup
    let recent_path_map: std::collections::HashMap<String, WikiProject> = recent_projects
        .iter()
        .map(|p| (p.path.clone(), p.clone()))
        .collect();

    // Scan projects_dir
    let mut projects = Vec::new();

    if projects_dir.exists() {
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

                let has_wiki = path.join("wiki").exists();
                let path_str = path.to_string_lossy().to_string();

                // Get last_opened_at from pre-loaded map
                let last_opened_at = recent_path_map.get(&path_str)
                    .and_then(|p| last_opened_map.get(&p.id))
                    .copied();

                projects.push(ProjectInfo {
                    name,
                    path: path_str,
                    has_wiki,
                    last_opened_at,
                });
            }
        }
    }

    // Also include recent projects that are outside projects_dir
    for recent in recent_projects.iter() {
        let recent_path = std::path::PathBuf::from(&recent.path);
        if !recent_path.starts_with(projects_dir) && recent_path.exists() {
            let has_wiki = recent_path.join("wiki").exists();
            let last_opened_at = last_opened_map.get(&recent.id).copied();

            projects.push(ProjectInfo {
                name: recent.name.clone(),
                path: recent.path.clone(),
                has_wiki,
                last_opened_at,
            });
        }
    }

    // Sort: first by last_opened_at (most recent first), then by name
    projects.sort_by(|a, b| {
        match (a.last_opened_at, b.last_opened_at) {
            (Some(a_time), Some(b_time)) => b_time.cmp(&a_time),  // Most recent first
            (Some(_), None) => std::cmp::Ordering::Less,  // Has timestamp comes first
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.cmp(&b.name),
        }
    });

    Ok(Json(projects))
}