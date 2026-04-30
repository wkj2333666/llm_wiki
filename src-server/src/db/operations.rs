use serde::{de::DeserializeOwned, Serialize};
use sqlx::SqlitePool;

use crate::types::wiki::WikiProject;

/// Get a config value by key
pub async fn get_config<T: DeserializeOwned + Default>(
    pool: &SqlitePool,
    key: &str,
) -> Result<Option<T>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM app_config WHERE key = ?"
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((value,)) => {
            let parsed: T = serde_json::from_str(&value).unwrap_or_default();
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

/// Set a config value by key
pub async fn set_config<T: Serialize>(
    pool: &SqlitePool,
    key: &str,
    value: &T,
) -> Result<(), sqlx::Error> {
    let json = serde_json::to_string(value).unwrap_or_default();
    sqlx::query(
        "INSERT OR REPLACE INTO app_config (key, value) VALUES (?, ?)"
    )
    .bind(key)
    .bind(&json)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get project by path
pub async fn get_project_by_path(pool: &SqlitePool, path: &str) -> Result<Option<WikiProject>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, name, path FROM projects WHERE path = ?"
    )
    .bind(path)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((id, name, path)) => Ok(Some(WikiProject { id, name, path })),
        None => Ok(None),
    }
}

/// Get last opened timestamp for a project by ID
pub async fn get_last_opened_at(pool: &SqlitePool, project_id: &str) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT last_opened_at FROM projects WHERE id = ?"
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.0))
}

/// Get all projects (ordered by recent_projects list)
pub async fn get_projects(pool: &SqlitePool) -> Result<Vec<WikiProject>, sqlx::Error> {
    let recent: Vec<String> = get_config::<Vec<String>>(pool, "recent_projects")
        .await?
        .unwrap_or_default();

    let mut projects = Vec::new();
    for path in recent {
        if let Some(project) = get_project_by_path(pool, &path).await? {
            projects.push(project);
        }
    }
    Ok(projects)
}

/// Get last opened project
pub async fn get_last_project(pool: &SqlitePool) -> Result<Option<WikiProject>, sqlx::Error> {
    let path: Option<String> = get_config::<Option<String>>(pool, "last_project")
        .await?
        .unwrap_or(None);

    if let Some(path) = path {
        return get_project_by_path(pool, &path).await;
    }
    Ok(None)
}

/// Save last opened project
pub async fn save_last_project(pool: &SqlitePool, project: &WikiProject) -> Result<(), sqlx::Error> {
    set_config(pool, "last_project", &Some(project.path.clone())).await?;
    add_to_recent_projects(pool, project).await?;
    Ok(())
}

/// Add project to recent list (upserts project record and updates order)
pub async fn add_to_recent_projects(pool: &SqlitePool, project: &WikiProject) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    // Upsert project record
    sqlx::query(
        "INSERT INTO projects (id, name, path, created_at, last_opened_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(path) DO UPDATE SET name = ?, last_opened_at = ?, id = ?"
    )
    .bind(&project.id)
    .bind(&project.name)
    .bind(&project.path)
    .bind(now)  // created_at (only used on insert)
    .bind(now)  // last_opened_at
    .bind(&project.name)  // update: name
    .bind(now)  // update: last_opened_at
    .bind(&project.id)  // update: id (keep consistent)
    .execute(pool)
    .await?;

    // Update recent_projects order: move this project to front
    let mut recent: Vec<String> = get_config::<Vec<String>>(pool, "recent_projects")
        .await?
        .unwrap_or_default();

    recent.retain(|p| p != &project.path);
    recent.insert(0, project.path.clone());
    recent.truncate(20); // Keep only 20 recent projects

    set_config(pool, "recent_projects", &recent).await?;

    // Also update last_project
    set_config(pool, "last_project", &Some(project.path.clone())).await?;

    Ok(())
}

/// Remove project from recent list
pub async fn remove_project(pool: &SqlitePool, path: &str) -> Result<(), sqlx::Error> {
    // Remove from recent_projects list
    let mut recent: Vec<String> = get_config::<Vec<String>>(pool, "recent_projects")
        .await?
        .unwrap_or_default();

    recent.retain(|p| p != path);
    set_config(pool, "recent_projects", &recent).await?;

    // Clear last_project if it matches
    let last: Option<String> = get_config::<Option<String>>(pool, "last_project")
        .await?
        .unwrap_or(None);
    if last.as_ref() == Some(&path.to_string()) {
        set_config(pool, "last_project", &None::<String>).await?;
    }

    // Delete from projects table
    sqlx::query("DELETE FROM projects WHERE path = ?")
        .bind(path)
        .execute(pool)
        .await?;

    Ok(())
}