use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;
use crate::db;
use crate::types::{AuthUser, WikiProject};

#[derive(Deserialize)]
pub struct CreateProjectBody {
    name: String,
    #[serde(default)]
    path: Option<String>,
}

/// Create a new wiki project under the current user's directory.
pub async fn create_project(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<WikiProject>, String> {
    let base_path = body.path
        .filter(|p| !p.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| state.config.server.projects_dir.join(&auth_user.user_id));

    let root = base_path.join(&body.name);

    if root.exists() {
        return Err(format!("Directory already exists: '{}'", root.display()));
    }

    std::fs::create_dir_all(&root)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

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

    db::add_to_recent_projects(&state.db, &project, Some(&auth_user.user_id))
        .await
        .map_err(|e| format!("Failed to save project: {}", e))?;

    tracing::info!("User '{}' created project '{}' at '{}'", auth_user.username, project.name, project.path);

    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct OpenProjectBody {
    name: String,
}

/// Open an existing wiki project by name under the current user's directory.
/// Admin users can also open projects from other users' directories.
pub async fn open_project(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<OpenProjectBody>,
) -> Result<Json<WikiProject>, String> {
    if auth_user.is_admin() {
        // Search across all user directories for the named project
        let projects_dir = &state.config.server.projects_dir;
        if let Ok(entries) = std::fs::read_dir(projects_dir) {
            for entry in entries.flatten() {
                let user_dir = entry.path();
                if !user_dir.is_dir() { continue; }
                let candidate = user_dir.join(&body.name);
                if candidate.is_dir() && candidate.join("wiki").exists() {
                    let path_str = candidate.to_string_lossy().to_string();
                    let existing = db::get_project_by_path(&state.db, &path_str)
                        .await
                        .map_err(|e| format!("Database error: {}", e))?;
                    let project = existing.unwrap_or_else(|| {
                        let id = uuid::Uuid::new_v4().to_string();
                        WikiProject { id, name: body.name.clone(), path: path_str }
                    });
                    db::add_to_recent_projects(&state.db, &project, Some(&auth_user.user_id))
                        .await
                        .map_err(|e| format!("Failed to update project: {}", e))?;
                    tracing::info!("Admin '{}' opened project '{}'", auth_user.username, project.name);
                    return Ok(Json(project));
                }
            }
        }
        return Err(format!("Project '{}' not found", body.name));
    }

    let root = state.config.server.projects_dir
        .join(&auth_user.user_id)
        .join(&body.name);

    if !root.exists() {
        return Err(format!("Project '{}' does not exist", body.name));
    }
    if !root.is_dir() {
        return Err(format!("'{}' is not a directory", body.name));
    }
    if !root.join("wiki").exists() {
        return Err(format!(
            "'{}' is not a valid wiki project (missing wiki folder)",
            body.name
        ));
    }

    let path_str = root.to_string_lossy().to_string();
    let existing = db::get_project_by_path(&state.db, &path_str)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let project = existing.unwrap_or_else(|| {
        let id = uuid::Uuid::new_v4().to_string();
        WikiProject { id, name: body.name, path: path_str }
    });

    db::add_to_recent_projects(&state.db, &project, Some(&auth_user.user_id))
        .await
        .map_err(|e| format!("Failed to update project: {}", e))?;

    tracing::info!("User '{}' opened project '{}'", auth_user.username, project.name);

    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct OpenProjectByPathBody {
    path: String,
}

/// Open a wiki project by full path.
/// Validates that the user owns the path (or is admin).
pub async fn open_project_by_path(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<OpenProjectByPathBody>,
) -> Result<Json<WikiProject>, String> {
    let root = std::path::PathBuf::from(&body.path);

    if !root.exists() {
        return Err(format!("Path does not exist: '{}'", body.path));
    }
    if !root.is_dir() {
        return Err(format!("Path is not a directory: '{}'", body.path));
    }
    if !root.join("wiki").exists() {
        return Err(format!(
            "Not a valid wiki project (missing wiki folder): '{}'",
            body.path
        ));
    }

    // Validate ownership
    validate_user_path(&auth_user, &body.path, &state.config.server.projects_dir)?;

    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let existing = db::get_project_by_path(&state.db, &body.path)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let project = existing.unwrap_or_else(|| {
        let id = uuid::Uuid::new_v4().to_string();
        WikiProject { id, name, path: body.path }
    });

    db::add_to_recent_projects(&state.db, &project, Some(&auth_user.user_id))
        .await
        .map_err(|e| format!("Failed to update project: {}", e))?;

    tracing::info!("User '{}' opened project '{}' by path", auth_user.username, project.name);

    Ok(Json(project))
}

#[derive(Serialize)]
pub struct ProjectInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    name: String,
    path: String,
    has_wiki: bool,
    last_opened_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
}

/// List projects: admin sees all, regular users see only their own.
pub async fn list_projects(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<ProjectInfo>>, String> {
    let projects_dir = &state.config.server.projects_dir;

    if !projects_dir.exists() {
        std::fs::create_dir_all(projects_dir)
            .map_err(|e| format!("Failed to create projects directory: {}", e))?;
    }

    let mut projects = Vec::new();

    if auth_user.is_admin() {
        let db_projects_with_owner = db::get_all_projects_with_owner(&state.db)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        // Index by path for O(1) lookup
        let db_map: HashMap<&str, &(WikiProject, i64, Option<String>)> = db_projects_with_owner
            .iter()
            .map(|item| (item.0.path.as_str(), item))
            .collect();

        let mut seen = std::collections::HashSet::new();

        // Scan all user directories under projects_dir
        if let Ok(entries) = std::fs::read_dir(projects_dir) {
            for entry in entries.flatten() {
                let user_dir = entry.path();
                if !user_dir.is_dir() {
                    continue;
                }
                if let Ok(proj_entries) = std::fs::read_dir(&user_dir) {
                    for proj_entry in proj_entries.flatten() {
                        let path = proj_entry.path();
                        if path.is_dir() {
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            let has_wiki = path.join("wiki").exists();
                            let path_str = path.to_string_lossy().to_string();

                            seen.insert(path_str.clone());
                            let db_match = db_map.get(path_str.as_str());

                            projects.push(ProjectInfo {
                                id: db_match.map(|(p, _, _)| p.id.clone()),
                                name,
                                path: path_str,
                                has_wiki,
                                last_opened_at: db_match.map(|(_, ts, _)| *ts),
                                owner: db_match.and_then(|(_, _, o)| o.clone()),
                            });
                        }
                    }
                }
            }
        }

        // Include DB-only projects not found on disk
        for (db_proj, ts, owner) in &db_projects_with_owner {
            if !seen.contains(db_proj.path.as_str()) {
                let path = std::path::PathBuf::from(&db_proj.path);
                projects.push(ProjectInfo {
                    id: Some(db_proj.id.clone()),
                    name: db_proj.name.clone(),
                    path: db_proj.path.clone(),
                    has_wiki: path.join("wiki").exists(),
                    last_opened_at: Some(*ts),
                    owner: owner.clone(),
                });
            }
        }
    } else {
        let db_projects_with_owner = db::get_user_projects_with_owner(&state.db, &auth_user.user_id)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        let db_map: HashMap<&str, &(WikiProject, i64, Option<String>)> = db_projects_with_owner
            .iter()
            .map(|item| (item.0.path.as_str(), item))
            .collect();

        let user_project_dir = projects_dir.join(&auth_user.user_id);
        if user_project_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&user_project_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let has_wiki = path.join("wiki").exists();
                        let path_str = path.to_string_lossy().to_string();

                        let db_match = db_map.get(path_str.as_str());

                        projects.push(ProjectInfo {
                            id: db_match.map(|(p, _, _)| p.id.clone()),
                            name,
                            path: path_str,
                            has_wiki,
                            last_opened_at: db_match.map(|(_, ts, _)| *ts),
                            owner: db_match.and_then(|(_, _, o)| o.clone()),
                        });
                    }
                }
            }
        }
    }

    // Sort: most recent first, then by name
    projects.sort_by(|a, b| {
        match (a.last_opened_at, b.last_opened_at) {
            (Some(a_time), Some(b_time)) => b_time.cmp(&a_time),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.cmp(&b.name),
        }
    });

    Ok(Json(projects))
}

// ── Path validation helper ───────────────────────────────────────

/// Check that a path belongs to the authenticated user.
/// Delegates to `AuthUser::validate_path`.
pub fn validate_user_path(
    auth_user: &AuthUser,
    path: &str,
    projects_dir: &std::path::Path,
) -> Result<(), String> {
    auth_user.validate_path(path, projects_dir)
}
