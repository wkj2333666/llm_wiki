use axum::Json;
use serde::Deserialize;

use crate::types::WikiProject;

#[derive(Deserialize)]
pub struct CreateProjectBody {
    name: String,
    path: String,
}

/// Create a new wiki project
pub async fn create_project(
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<WikiProject>, String> {
    // TODO: implement using services::project::create_project
    let root = std::path::Path::new(&body.path).join(&body.name);

    if root.exists() {
        return Err(format!("Directory already exists: '{}'", root.display()));
    }

    // Create directory structure
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let id = uuid::Uuid::new_v4().to_string();
    let project = WikiProject {
        id,
        name: body.name,
        path: root.to_string_lossy().to_string(),
    };

    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct OpenProjectBody {
    path: String,
}

/// Open an existing wiki project
pub async fn open_project(
    Json(body): Json<OpenProjectBody>,
) -> Result<Json<WikiProject>, String> {
    // TODO: implement using services::project::open_project
    let root = std::path::Path::new(&body.path);

    if !root.exists() {
        return Err(format!("Path does not exist: '{}'", body.path));
    }
    if !root.is_dir() {
        return Err(format!("Path is not a directory: '{}'", body.path));
    }

    // Validate that this looks like a wiki project
    if !root.join("schema.md").exists() {
        return Err(format!(
            "Not a valid wiki project (missing schema.md): '{}'",
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
        path: body.path.replace('\\', "/"),
    };

    Ok(Json(project))
}