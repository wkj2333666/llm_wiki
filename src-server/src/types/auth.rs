use serde::{Deserialize, Serialize};

// Role constants
pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_USER: &str = "user";
const SERVICE_USER_ID: &str = "service";

/// User record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub role: String,
    pub created_at: i64,
}

/// Login request body
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Token response returned after successful login
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub user: User,
}

/// Authenticated user extracted from JWT or legacy token
///
/// Injected into request extensions by auth middleware.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

impl AuthUser {
    /// Construct the service admin identity used for legacy server tokens.
    pub fn service_admin() -> Self {
        Self {
            user_id: SERVICE_USER_ID.into(),
            username: SERVICE_USER_ID.into(),
            role: ROLE_ADMIN.into(),
        }
    }

    pub fn from_claims(user_id: String, username: String, role: String) -> Self {
        Self { user_id, username, role }
    }

    pub fn is_admin(&self) -> bool {
        self.role == ROLE_ADMIN || self.user_id == SERVICE_USER_ID
    }

    /// Check that `path` starts with `<projects_dir>/<self.user_id>/`.
    /// Admins bypass this check.
    pub fn validate_path(&self, path: &str, projects_dir: &std::path::Path) -> Result<(), String> {
        if self.is_admin() {
            return Ok(());
        }

        let user_prefix = projects_dir.join(&self.user_id);
        let user_prefix_str = user_prefix.to_string_lossy().to_string();
        // Append separator so "user1" doesn't match "user12" projects
        let prefix_with_sep = format!("{}{}", user_prefix_str, std::path::MAIN_SEPARATOR);

        let normalized = std::path::Path::new(path)
            .to_string_lossy()
            .to_string();

        if normalized != user_prefix_str && !normalized.starts_with(&prefix_with_sep) {
            return Err(format!(
                "Access denied: path '{}' is outside your project directory",
                path
            ));
        }

        Ok(())
    }
}
