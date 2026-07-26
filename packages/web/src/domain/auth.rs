use super::roles::SystemRole;
use super::user::{User, UserSession};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthResult {
    Success(UserSession),
    InvalidCredentials,
    AccountInactive,
    AccountLocked,
    TemporaryPassword(String),
    EmailNotConfirmed,
    ServerError(String),
}

pub struct AuthService;

impl AuthService {
    pub fn validate_credentials(credentials: &AuthCredentials) -> Result<(), AuthError> {
        if credentials.email.is_empty() || !credentials.email.contains('@') {
            return Err(AuthError::InvalidEmail);
        }
        if credentials.password.len() < 8 {
            return Err(AuthError::PasswordTooShort);
        }
        Ok(())
    }

    pub fn get_redirect_path(role: &SystemRole) -> &'static str {
        match role {
            SystemRole::PlatformAdmin => "/dashboard/platform-admin",
            SystemRole::SchoolManager
            | SystemRole::Teacher
            | SystemRole::Student
            | SystemRole::Parent => "/dashboard",
        }
    }

    pub fn can_access_route(user: &User, route: &str) -> bool {
        if ["/", "/login", "/forgot-password"].contains(&route) {
            return true;
        }

        if route.starts_with("/dashboard/platform-admin") {
            return user.role == SystemRole::PlatformAdmin;
        }
        if route.starts_with("/dashboard/school-manager") {
            return user.role == SystemRole::SchoolManager;
        }
        if route.starts_with("/dashboard/teacher") {
            return user.role == SystemRole::Teacher;
        }
        if route.starts_with("/dashboard/student") {
            return user.role == SystemRole::Student;
        }
        if route.starts_with("/dashboard/parent") {
            return user.role == SystemRole::Parent;
        }
        if route.starts_with("/dashboard") {
            return true;
        }

        if route.starts_with("/admin") {
            return user.role == SystemRole::SchoolManager;
        }

        false
    }

    pub fn get_error_message(error: &AuthError) -> &'static str {
        match error {
            AuthError::InvalidCredentials => "Invalid email or password",
            AuthError::AccountInactive => "Your account has been deactivated",
            AuthError::AccountLocked => "Your account has been locked",
            AuthError::EmailNotConfirmed => "Please confirm your email address",
            AuthError::InvalidEmail => "Please enter a valid email address",
            AuthError::PasswordTooShort => "Password must be at least 8 characters",
            AuthError::SessionExpired => "Your session has expired, please login again",
            AuthError::Unauthorized => "You don't have permission to access this resource",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthError {
    InvalidCredentials,
    AccountInactive,
    AccountLocked,
    EmailNotConfirmed,
    InvalidEmail,
    PasswordTooShort,
    SessionExpired,
    Unauthorized,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AuthError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetConfirmation {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordChangeRequest {
    pub current_password: String,
    pub new_password: String,
}

pub struct AccessControl;

impl AccessControl {
    pub fn can_perform_action(user: &User, action: &str, resource: &str) -> bool {
        match (resource, action) {
            ("knowledge_asset", "review" | "embed" | "publish" | "archive") => {
                user.role.can_manage_platform_knowledge()
            }
            ("knowledge_audit", "read") => user.role.can_manage_platform_knowledge(),
            ("user", "create" | "update" | "delete") => user.has_permission("manage_users"),
            ("user", "read") => true,
            ("class", "create" | "update" | "delete") => user.has_permission("manage_classes"),
            ("class", "read") => user.has_permission("view_classes"),
            ("assignment", "create" | "grade") => user.has_permission("create_assignments"),
            ("assignment", "submit") => user.has_permission("submit_assignments"),
            ("assignment", "read") => true,
            ("report", "read") => user.has_permission("view_reports"),
            ("system", "manage") => user.has_permission("manage_system_settings"),
            _ => false,
        }
    }

    pub fn get_accessible_routes(user: &User) -> Vec<&'static str> {
        let mut routes = vec!["/dashboard", "/profile", "/settings"];

        match user.role {
            SystemRole::PlatformAdmin => {
                routes.extend_from_slice(&[
                    "/dashboard/platform-admin",
                    "/dashboard/platform-admin/knowledge-assets",
                    "/dashboard/platform-admin/knowledge-audit",
                ]);
            }
            SystemRole::SchoolManager => {
                routes.extend_from_slice(&[
                    "/admin/users",
                    "/admin/classes",
                    "/admin/reports",
                    "/admin/settings",
                ]);
            }
            SystemRole::Teacher | SystemRole::Student | SystemRole::Parent => {}
        }

        if user.role.can_manage_classes() {
            routes.push("/classes");
        }
        if user.role.can_view_reports() {
            routes.push("/reports");
        }
        routes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_credentials() {
        let valid = AuthCredentials {
            email: "test@example.com".to_string(),
            password: "securepassword123".to_string(),
        };
        assert!(AuthService::validate_credentials(&valid).is_ok());

        let invalid = AuthCredentials {
            email: "invalid-email".to_string(),
            password: "short".to_string(),
        };
        assert!(AuthService::validate_credentials(&invalid).is_err());
    }

    #[test]
    fn role_specific_dashboards_are_isolated() {
        let platform_admin = User::new(
            "1".to_string(),
            "platform@example.com".to_string(),
            SystemRole::PlatformAdmin,
            None,
        );
        let school_manager = User::new(
            "2".to_string(),
            "manager@example.com".to_string(),
            SystemRole::SchoolManager,
            None,
        );

        assert!(AuthService::can_access_route(
            &platform_admin,
            "/dashboard/platform-admin"
        ));
        assert!(!AuthService::can_access_route(
            &school_manager,
            "/dashboard/platform-admin"
        ));
        assert!(!AuthService::can_access_route(
            &platform_admin,
            "/dashboard/school-manager"
        ));
    }

    #[test]
    fn access_control_preserves_existing_school_workflows() {
        let school_manager = User::new(
            "1".to_string(),
            "admin@school.com".to_string(),
            SystemRole::SchoolManager,
            None,
        );
        let teacher = User::new(
            "2".to_string(),
            "teacher@school.com".to_string(),
            SystemRole::Teacher,
            None,
        );
        let student = User::new(
            "3".to_string(),
            "student@school.com".to_string(),
            SystemRole::Student,
            None,
        );

        assert!(AccessControl::can_perform_action(
            &school_manager,
            "create",
            "user"
        ));
        assert!(AccessControl::can_perform_action(
            &teacher,
            "create",
            "assignment"
        ));
        assert!(AccessControl::can_perform_action(
            &student,
            "submit",
            "assignment"
        ));
    }
}
