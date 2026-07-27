use crate::domain::{AuthCredentials, AuthError, AuthResult, SystemRole, User, UserSession};
use crate::infrastructure::auth_provider::AuthProvider;
use dioxus::prelude::*;

/// Application authentication service
pub struct AppAuthService;

impl AppAuthService {
    /// Authenticate user with credentials
    pub async fn login(credentials: AuthCredentials) -> AuthResult {
        // Validate credentials format first
        if let Err(error) = crate::domain::auth::AuthService::validate_credentials(&credentials) {
            return match error {
                AuthError::InvalidEmail => AuthResult::InvalidCredentials,
                AuthError::PasswordTooShort => AuthResult::InvalidCredentials,
                _ => AuthResult::InvalidCredentials,
            };
        }

        // Delegate to infrastructure layer for actual authentication
        AuthProvider::authenticate(credentials).await
    }

    /// Logout user and clear session
    pub async fn logout() -> Result<(), AuthError> {
        AuthProvider::logout().await
    }

    /// Get current authenticated user
    pub async fn get_current_user() -> Result<Option<User>, AuthError> {
        AuthProvider::get_current_user().await
    }

    /// Check if user is authenticated
    pub async fn is_authenticated() -> bool {
        AuthProvider::is_authenticated().await
    }

    /// Refresh user session
    pub async fn refresh_session() -> Result<UserSession, AuthError> {
        AuthProvider::refresh_session().await
    }

    /// Check if current user has required role
    pub async fn has_role(required_role: SystemRole) -> bool {
        if let Ok(Some(user)) = Self::get_current_user().await {
            user.role == required_role
        } else {
            false
        }
    }

    /// Check if current user has any of the required roles
    pub async fn has_any_role(required_roles: &[SystemRole]) -> bool {
        if let Ok(Some(user)) = Self::get_current_user().await {
            required_roles.contains(&user.role)
        } else {
            false
        }
    }

    /// Check if current user has specific permission
    pub async fn has_permission(permission: &str) -> bool {
        if let Ok(Some(user)) = Self::get_current_user().await {
            user.has_permission(permission)
        } else {
            false
        }
    }
}

/// Authentication hooks for Dioxus components
pub struct AuthHooks;

impl AuthHooks {
    /// Hook to get current user state
    pub fn use_current_user() -> Result<Option<User>, AuthError> {
        // Read directly from global state for performance
        let user = crate::infrastructure::auth_provider::CURRENT_USER_STATE
            .read()
            .clone();
        Ok(user)
    }

    /// Hook to check authentication status
    pub fn use_is_authenticated() -> bool {
        crate::infrastructure::auth_provider::CURRENT_USER_STATE
            .read()
            .is_some()
    }

    /// Hook to check if user has specific role
    pub fn use_has_role(required_role: SystemRole) -> bool {
        if let Some(user) = crate::infrastructure::auth_provider::CURRENT_USER_STATE
            .read()
            .as_ref()
        {
            user.role == required_role
        } else {
            false
        }
    }

    /// Hook to check if user has specific permission
    pub fn use_has_permission(permission: String) -> bool {
        if let Some(user) = crate::infrastructure::auth_provider::CURRENT_USER_STATE
            .read()
            .as_ref()
        {
            user.has_permission(&permission)
        } else {
            false
        }
    }

    /// Hook to get user-friendly error message
    pub fn use_auth_error() -> Option<String> {
        let error_state = use_signal(|| Option::<String>::None);

        // This would be set by other auth operations
        let result = error_state.read().clone();
        result
    }

    /// Set authentication error
    pub fn set_auth_error(error: String) {
        // This would update the global error state
        // Implementation depends on your state management approach
        // Placeholder implementation
        let _ = error;
    }

    /// Clear authentication error
    pub fn clear_auth_error() {
        // This would clear the global error state
    }
}

/// Role guard components
pub struct RoleGuard;

impl RoleGuard {
    /// Component that only renders if user has required role
    pub fn require_role(
        required_role: SystemRole,
        fallback: Option<Element>,
        children: Element,
    ) -> Element {
        let has_role = AuthHooks::use_has_role(required_role);

        let fallback_content = if let Some(fallback_element) = fallback {
            fallback_element
        } else {
            rsx! {
                div {
                    style: "padding: 2rem; text-align: center;",
                    h2 { "Access Denied" }
                    p { "You don't have permission to access this page." }
                }
            }
        };

        rsx! {
            if has_role {
                {children}
            } else {
                {fallback_content}
            }
        }
    }

    /// Component that only renders if user has any of the required roles
    pub fn require_any_role(
        required_roles: Vec<SystemRole>,
        fallback: Option<Element>,
        children: Element,
    ) -> Element {
        let mut has_role = use_signal(|| false);

        use_effect(move || {
            let roles = required_roles.clone();
            spawn(async move {
                let has_any = AppAuthService::has_any_role(&roles).await;
                has_role.set(has_any);
            });
        });

        let fallback_content = if let Some(fallback_element) = fallback {
            fallback_element
        } else {
            rsx! {
                div {
                    style: "padding: 2rem; text-align: center;",
                    h2 { "Access Denied" }
                    p { "You don't have permission to access this page." }
                }
            }
        };

        rsx! {
            if *has_role.read() {
                {children}
            } else {
                {fallback_content}
            }
        }
    }

    /// Component that only renders if user has required permission
    pub fn require_permission(
        required_permission: String,
        fallback: Option<Element>,
        children: Element,
    ) -> Element {
        let has_permission = AuthHooks::use_has_permission(required_permission);

        let fallback_content = if let Some(fallback_element) = fallback {
            fallback_element
        } else {
            rsx! {
                div {
                    style: "padding: 1rem; background: #fee; border: 1px solid #fcc; border-radius: 4px;",
                    "Permission denied"
                }
            }
        };

        rsx! {
            if has_permission {
                {children}
            } else {
                {fallback_content}
            }
        }
    }
}

/// Authentication utility functions
pub struct AuthUtils;

impl AuthUtils {
    /// Get redirect URL after successful login based on user role
    pub fn get_login_redirect(user: &User) -> String {
        crate::domain::auth::AuthService::get_redirect_path(&user.role).to_string()
    }

    /// Format user role for display
    pub fn format_role_display(role: &SystemRole) -> String {
        role.display_name().to_string()
    }

    /// Check if session is valid
    pub async fn validate_session() -> bool {
        match AppAuthService::refresh_session().await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Handle authentication errors and provide user feedback
    pub fn handle_auth_error(error: &AuthError) -> String {
        crate::domain::auth::AuthService::get_error_message(error).to_string()
    }
}
