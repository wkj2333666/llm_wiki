use serde::{de::DeserializeOwned, Serialize};
use sqlx::SqlitePool;

use crate::types::{User, wiki::WikiProject, ROLE_ADMIN};

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

/// Get project by ID
pub async fn get_project_by_id(pool: &SqlitePool, project_id: &str) -> Result<Option<WikiProject>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, name, path FROM projects WHERE id = ?"
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, name, path)| WikiProject { id, name, path }))
}

/// Get projects for a specific user with last_opened_at timestamps
pub async fn get_user_projects_with_ts(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<(WikiProject, i64)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT id, name, path, last_opened_at FROM projects WHERE user_id = ? ORDER BY last_opened_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, path, ts)| (WikiProject { id, name, path }, ts))
        .collect())
}

/// Get projects for a specific user (ordered by last_opened_at DESC)
pub async fn get_user_projects(pool: &SqlitePool, user_id: &str) -> Result<Vec<WikiProject>, sqlx::Error> {
    Ok(get_user_projects_with_ts(pool, user_id).await?.into_iter().map(|(p, _)| p).collect())
}

/// Get all projects regardless of user (admin use)
pub async fn get_all_projects(pool: &SqlitePool) -> Result<Vec<WikiProject>, sqlx::Error> {
    Ok(get_all_projects_with_ts(pool).await?.into_iter().map(|(p, _)| p).collect())
}

/// Get all projects with last_opened_at timestamps (admin use)
pub async fn get_all_projects_with_ts(
    pool: &SqlitePool,
) -> Result<Vec<(WikiProject, i64)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT id, name, path, last_opened_at FROM projects ORDER BY last_opened_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, path, ts)| (WikiProject { id, name, path }, ts))
        .collect())
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

/// Add project to recent list (upserts project record and updates order)
pub async fn add_to_recent_projects(pool: &SqlitePool, project: &WikiProject, user_id: Option<&str>) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    // Upsert project record
    sqlx::query(
        "INSERT INTO projects (id, name, path, user_id, created_at, last_opened_at) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(path) DO UPDATE SET name = ?, last_opened_at = ?"
    )
    .bind(&project.id)
    .bind(&project.name)
    .bind(&project.path)
    .bind(user_id)
    .bind(now)  // created_at (only used on insert)
    .bind(now)  // last_opened_at
    .bind(&project.name)  // update: name
    .bind(now)  // update: last_opened_at
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

    Ok(())
}

// ── User operations ──────────────────────────────────────────────

/// Create a new user
pub async fn create_user(
    pool: &SqlitePool,
    id: &str,
    username: &str,
    password_hash: &str,
    role: &str,
    created_at: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get user by username
pub async fn get_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>, sqlx::Error> {
    Ok(get_user_with_hash(pool, username).await?.map(|(u, _)| u))
}

/// Get user by id
pub async fn get_user_by_id(pool: &SqlitePool, user_id: &str) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id, username, role, password_hash, created_at FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, username, role, _password_hash, created_at)| User {
        id,
        username,
        role,
        created_at,
    }))
}

/// Get user and password hash in a single query for login
pub async fn get_user_with_hash(pool: &SqlitePool, username: &str) -> Result<Option<(User, String)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id, username, role, password_hash, created_at FROM users WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, username, role, password_hash, created_at)| {
        (User { id, username, role, created_at }, password_hash)
    }))
}

/// Upsert a user: update password/role if exists, insert if not
pub async fn upsert_user(
    pool: &SqlitePool,
    id: &str,
    username: &str,
    password_hash: &str,
    role: &str,
    created_at: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(username) DO UPDATE SET
             password_hash = excluded.password_hash,
             role = excluded.role"
    )
    .bind(id)
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all users
pub async fn get_all_users(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id, username, role, password_hash, created_at FROM users ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, username, role, _password_hash, created_at)| User {
            id,
            username,
            role,
            created_at,
        })
        .collect())
}

/// Delete a user by username
pub async fn delete_user(pool: &SqlitePool, username: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE username = ?")
        .bind(username)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Get the first admin user (for legacy project migration)
pub async fn get_first_admin_user(pool: &SqlitePool) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id, username, role, password_hash, created_at FROM users WHERE role = ? ORDER BY created_at LIMIT 1"
    )
    .bind(ROLE_ADMIN)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, username, role, _password_hash, created_at)| User {
        id,
        username,
        role,
        created_at,
    }))
}

/// Get projects with NULL user_id (orphan legacy projects)
pub async fn get_orphan_projects(pool: &SqlitePool) -> Result<Vec<(String, String)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT id, path FROM projects WHERE user_id IS NULL"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Assign user_id to a project
pub async fn assign_project_user(pool: &SqlitePool, project_path: &str, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects SET user_id = ? WHERE path = ?")
        .bind(user_id)
        .bind(project_path)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Project operations ───────────────────────────────────────────

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