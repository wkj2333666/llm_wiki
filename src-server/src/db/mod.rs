mod schema;
mod operations;

pub use operations::*;
pub use schema::*;

use sqlx::SqlitePool;

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(SCHEMA)
        .execute(pool)
        .await?;

    // Add user_id column to projects if it doesn't exist (migration from pre-user version)
    sqlx::query(
        "ALTER TABLE projects ADD COLUMN user_id TEXT REFERENCES users(id)"
    )
    .execute(pool)
    .await
    .ok(); // Ignore error if column already exists

    Ok(())
}

/// Bootstrap users from config and migrate orphan projects.
///
/// Returns the admin user (newly created or existing) for project migration.
pub async fn bootstrap_users(
    pool: &SqlitePool,
    config_users: &[crate::config::UserConfig],
    projects_dir: &std::path::Path,
) -> Result<(), String> {
    // Import users from config
    for uc in config_users {
        let id = uuid::Uuid::new_v4().to_string();
        let hash = if !uc.password_hash.is_empty() {
            uc.password_hash.clone()
        } else if !uc.password.is_empty() {
            crate::auth_pass::hash_password(&uc.password)
                .map_err(|e| format!("Failed to hash password for '{}': {}", uc.username, e))?
        } else {
            tracing::warn!("User '{}' has no password or password_hash set, skipping", uc.username);
            continue;
        };
        let now = chrono::Utc::now().timestamp();

        upsert_user(pool, &id, &uc.username, &hash, &uc.role, now)
            .await
            .map_err(|e| format!("Failed to upsert user '{}': {}", uc.username, e))?;

        tracing::info!("User '{}' (role: {}) imported from config", uc.username, uc.role);
    }

    // Check if we still have no users — create default admin
    let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    // Prefer the first config admin, then the first DB admin, then create default
    let config_admin = config_users.iter().find(|u| u.role == crate::types::ROLE_ADMIN);

    let admin_user = if let Some(ca) = config_admin {
        // Find the DB record for this config admin (just upserted above)
        get_user_by_username(pool, &ca.username)
            .await
            .map_err(|e| format!("DB error: {}", e))?
    } else if user_count == 0 {
        let admin_id = uuid::Uuid::new_v4().to_string();
        let admin_password = uuid::Uuid::new_v4().to_string().split('-').next().unwrap().to_string();
        let hash = crate::auth_pass::hash_password(&admin_password).unwrap();
        let now = chrono::Utc::now().timestamp();

        create_user(pool, &admin_id, "admin", &hash, crate::types::ROLE_ADMIN, now)
            .await
            .map_err(|e| format!("Failed to create default admin: {}", e))?;

        tracing::warn!(
            "No users configured. Created default admin: username='admin', password='{}'. CHANGE THIS PASSWORD!",
            admin_password
        );

        Some(crate::types::User {
            id: admin_id,
            username: "admin".into(),
            role: crate::types::ROLE_ADMIN.into(),
            created_at: now,
        })
    } else {
        get_first_admin_user(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?
    };

    // Migrate orphan projects (projects without user_id)
    let orphans = get_orphan_projects(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    if !orphans.is_empty() {
        if let Some(ref admin) = admin_user {
            for (project_id, ref path) in &orphans {
                let old_path = std::path::PathBuf::from(path);
                let project_name = old_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                let new_path = projects_dir.join(&admin.id).join(project_name);

                // Move project directory on disk
                let mut moved = false;
                if old_path.exists() && old_path != new_path {
                    // Ensure target parent directory exists
                    if let Some(parent) = new_path.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| tracing::warn!("Failed to create user project dir {:?}: {}", parent, e))
                            .ok();
                    }

                    match std::fs::rename(&old_path, &new_path) {
                        Ok(_) => {
                            tracing::info!("Moved project '{}' from {:?} to {:?}", project_name, old_path, new_path);
                            moved = true;
                        }
                        Err(e) => tracing::warn!("Failed to move project {:?} to {:?}: {}. Project will remain at original path.", old_path, new_path, e),
                    }
                }

                // Update database: set user_id
                let actual_path = if moved { &new_path } else { &old_path };
                assign_project_user(pool, &actual_path.to_string_lossy(), &admin.id)
                    .await
                    .map_err(|e| tracing::warn!("Failed to assign project {} to admin: {}", project_id, e))
                    .ok();

                if moved {
                    // Update path in database to new location
                    sqlx::query("UPDATE projects SET path = ? WHERE id = ?")
                        .bind(new_path.to_string_lossy().to_string())
                        .bind(project_id)
                        .execute(pool)
                        .await
                        .map_err(|e| tracing::warn!("Failed to update project path: {}", e))
                        .ok();
                }
            }
            tracing::info!("Migrated {} orphan project(s) to admin user '{}'", orphans.len(), admin.username);
        } else {
            tracing::warn!("{} orphan project(s) found but no admin user to assign them to", orphans.len());
        }
    }

    Ok(())
}
