use crate::domain::{AuthError, User, UserSession};
use crate::infrastructure::{AuthProvider, AuthState};
use dioxus::prelude::*;

/// Session management service
/// Relies on server-side HttpOnly cookies for authentication
pub struct SessionManager;

impl SessionManager {
    /// Initialize user session after successful authentication
    pub async fn create_session(session: UserSession) -> Result<(), AuthError> {
        // Update global state only - no LocalStorage
        AuthProvider::update_session(session.clone());
        AuthState::update_user(session.user.clone());
        Ok(())
    }

    /// Load and validate existing session from server
    pub async fn load_session() -> Result<Option<UserSession>, AuthError> {
        // Check with the server if we are authenticated (via cookies)
        if let Ok(Some(user)) = AuthProvider::get_current_user().await {
            let session = UserSession::new(user, "cookie-session".to_string());
            return Ok(Some(session));
        }

        Ok(None)
    }

    /// Clear current session and logout user
    pub async fn clear_session() -> Result<(), AuthError> {
        // Clear server-side cookies and global state
        AuthProvider::logout().await?;
        AuthState::clear_user();
        Ok(())
    }

    /// Refresh current session if it's still valid
    pub async fn refresh_session() -> Result<UserSession, AuthError> {
        // Server middleware handles refresh automatically
        AuthProvider::refresh_session().await
    }

    /// Check if session is valid
    pub async fn is_session_valid() -> bool {
        Self::load_session().await.ok().flatten().is_some()
    }

    /// Get current user from session
    pub async fn get_current_user() -> Option<User> {
        AuthState::get_current_user()
    }

    /// Get session timeout in milliseconds
    pub fn get_session_timeout() -> u64 {
        // 15 minutes (access token lifetime)
        15 * 60 * 1000
    }

    /// Check if session needs refresh
    /// Note: Server middleware handles this automatically
    pub async fn needs_refresh() -> bool {
        // Server middleware handles refresh automatically
        // This is kept for compatibility but always returns false
        false
    }
}

/// Session hooks for Dioxus components
pub struct SessionHooks;

impl SessionHooks {
    /// Hook to get current session state
    pub fn use_current_session() -> Option<UserSession> {
        let mut session_state = use_signal(|| Option::<UserSession>::None);

        use_effect(move || {
            spawn(async move {
                if let Ok(Some(session)) = SessionManager::load_session().await {
                    session_state.set(Some(session));
                }
            });
        });

        let result = session_state.read().clone();
        result
    }

    /// Hook to check if session is valid
    pub fn use_is_session_valid() -> bool {
        let mut validity_state = use_signal(|| false);

        use_effect(move || {
            spawn(async move {
                let is_valid = SessionManager::is_session_valid().await;
                validity_state.set(is_valid);
            });
        });

        let result = *validity_state.read();
        result
    }

    /// Hook to get session expiry time
    pub fn use_session_expiry() -> Option<String> {
        let session = Self::use_current_session();
        session.and_then(|s| s.expires_at)
    }

    /// Hook to check if session needs refresh
    /// Note: Server middleware handles this automatically
    pub fn use_needs_refresh() -> bool {
        // Server middleware handles refresh automatically
        false
    }
}

/// Session utility functions
pub struct SessionUtils;

impl SessionUtils {
    /// Format session expiry time for display
    pub fn format_expiry_time(expiry_str: &str) -> Option<String> {
        chrono::DateTime::parse_from_rfc3339(expiry_str)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .ok()
    }

    /// Get time remaining until session expires
    pub fn get_time_remaining(expiry_str: &str) -> Option<String> {
        if let Ok(expiry_time) = chrono::DateTime::parse_from_rfc3339(expiry_str) {
            let now = chrono::Utc::now();
            let duration = expiry_time.signed_duration_since(now);

            if duration.num_seconds() > 0 {
                let hours = duration.num_hours();
                let minutes = duration.num_minutes() % 60;

                if hours > 0 {
                    Some(format!("{}h {}m", hours, minutes))
                } else {
                    Some(format!("{}m", minutes))
                }
            } else {
                Some("Expired".to_string())
            }
        } else {
            None
        }
    }

    /// Check if session is about to expire (within 5 minutes)
    pub fn is_session_expiring_soon(expiry_str: &str) -> bool {
        if let Ok(expiry_time) = chrono::DateTime::parse_from_rfc3339(expiry_str) {
            let now = chrono::Utc::now();
            let duration = expiry_time.signed_duration_since(now);
            duration.num_minutes() <= 5 && duration.num_seconds() > 0
        } else {
            true // Assume expiring if can't parse
        }
    }

    /// Initialize session on app startup
    pub async fn initialize_session() -> Result<Option<User>, AuthError> {
        // Initialize auth state (checks server for existing session via cookies)
        crate::infrastructure::AuthState::initialize().await?;

        // Load session from server
        if let Ok(Some(session)) = SessionManager::load_session().await {
            if SessionManager::is_session_valid().await {
                return Ok(Some(session.user));
            } else {
                // Clear invalid session
                SessionManager::clear_session().await?;
            }
        }

        Ok(None)
    }
}
