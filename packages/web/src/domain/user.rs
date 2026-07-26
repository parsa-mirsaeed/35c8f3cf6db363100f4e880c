use serde::{Deserialize, Serialize};
use super::roles::SystemRole;

/// User domain model with all necessary user information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub full_name: Option<String>,
    pub role: SystemRole,
    pub school_id: Option<String>,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub last_login: Option<String>,
}

impl User {
    /// Create a new user instance
    pub fn new(
        id: String,
        email: String,
        role: SystemRole,
        school_id: Option<String>,
    ) -> Self {
        Self {
            id,
            email,
            full_name: None,
            role,
            school_id,
            is_active: true,
            created_at: None,
            last_login: None,
        }
    }

    /// Get user's display name
    pub fn display_name(&self) -> String {
        self.full_name
            .clone()
            .unwrap_or_else(|| self.email.split('@').next().unwrap_or("User").to_string())
    }

    /// Get user's initials for avatar
    pub fn initials(&self) -> String {
        let name = self.display_name();
        let parts: Vec<&str> = name.split_whitespace().collect();
        match parts.len() {
            0 => "U".to_string(),
            1 => parts[0].chars().next().unwrap_or('U').to_uppercase().to_string(),
            _ => format!(
                "{}{}",
                parts[0].chars().next().unwrap_or('U').to_uppercase(),
                parts[1].chars().next().unwrap_or(' ').to_uppercase()
            ),
        }
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &SystemRole) -> bool {
        &self.role == role
    }

    /// Check if user has administrative privileges
    pub fn is_administrative(&self) -> bool {
        self.role.is_administrative()
    }

    /// Get user's permissions based on role
    pub fn get_permissions(&self) -> Vec<&'static str> {
        super::roles::RolePermissions::get_permissions(&self.role)
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        super::roles::RolePermissions::has_permission(&self.role, permission)
    }
}

/// User session information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSession {
    pub user: User,
    pub auth_token: String,
    pub expires_at: Option<String>,
    pub device_info: Option<String>,
}

impl UserSession {
    /// Create a new user session
    pub fn new(user: User, auth_token: String) -> Self {
        Self {
            user,
            auth_token,
            expires_at: None,
            device_info: None,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = &self.expires_at {
            // Simple timestamp comparison - in production, use proper datetime parsing
            chrono::DateTime::parse_from_rfc3339(expires_at)
                .map(|expiry| expiry < chrono::Utc::now())
                .unwrap_or(false)
        } else {
            false
        }
    }
}

/// User profile update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserProfile {
    pub full_name: Option<String>,
    pub email: Option<String>,
}

/// Minimal user information for display purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub full_name: Option<String>,
    pub role: SystemRole,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            role: user.role,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "123".to_string(),
            "test@example.com".to_string(),
            SystemRole::Teacher,
            Some("school_1".to_string()),
        );

        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, SystemRole::Teacher);
        assert!(user.is_active);
    }

    #[test]
    fn test_user_display_name() {
        let mut user = User::new(
            "123".to_string(),
            "john.doe@example.com".to_string(),
            SystemRole::Student,
            None,
        );

        // Test without full name
        assert_eq!(user.display_name(), "john.doe");

        // Test with full name
        user.full_name = Some("John Doe".to_string());
        assert_eq!(user.display_name(), "John Doe");
    }

    #[test]
    fn test_user_initials() {
        let mut user = User::new(
            "123".to_string(),
            "test@example.com".to_string(),
            SystemRole::Student,
            None,
        );

        // Test email-based initials
        assert_eq!(user.initials(), "T");

        // Test full name initials
        user.full_name = Some("John Doe".to_string());
        assert_eq!(user.initials(), "JD");
    }
}