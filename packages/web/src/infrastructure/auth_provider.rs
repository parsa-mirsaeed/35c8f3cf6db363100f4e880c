use crate::domain::{AuthCredentials, AuthError, AuthResult, SystemRole, User, UserSession};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// Server function for checking current user (doesn't need Set-Cookie)
use api::server_functions::auth_functions::whoami;

/// Login request payload (matches server-side LoginRequest)
#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

/// Login response payload (matches server-side LoginResponse)
#[derive(Debug, Deserialize)]
struct LoginResponse {
    user: api::domain::UserInfo,
}

/// Production authentication provider implementation
/// Uses HttpOnly cookies for secure token management
pub struct AuthProvider;

impl AuthProvider {
    /// Authenticate user with credentials
    ///
    /// **IMPORTANT**: This uses gloo_net::http::Request instead of a Dioxus
    /// server function because we need the browser to receive and store
    /// the Set-Cookie headers from the response. Server function calls
    /// may not properly propagate these headers to the browser.
    pub async fn authenticate(credentials: AuthCredentials) -> AuthResult {
        web_sys::console::log_1(&"AuthProvider::authenticate starting".into());

        // Use gloo_net to make HTTP request - this ensures cookies are properly received
        let request = LoginRequest {
            email: credentials.email.clone(),
            password: credentials.password.clone(),
        };

        let response = match gloo_net::http::Request::post("/api/auth/login")
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request).unwrap())
            .expect("Failed to build request")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                web_sys::console::error_1(&format!("Network error during login: {}", e).into());
                return AuthResult::ServerError(format!("Network error: {}", e));
            }
        };

        if !response.ok() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            web_sys::console::error_1(&format!("Login failed: {} - {}", status, error_text).into());

            return if status == 401 {
                AuthResult::InvalidCredentials
            } else {
                AuthResult::ServerError(error_text)
            };
        }

        // Parse response - cookies are automatically stored by browser from Set-Cookie headers
        let login_response: LoginResponse = match response.json().await {
            Ok(resp) => resp,
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to parse login response: {}", e).into());
                return AuthResult::ServerError(format!("Failed to parse response: {}", e));
            }
        };

        let user_info = login_response.user;
        web_sys::console::log_1(&format!("User authenticated: {:?}", user_info).into());

        // Map Response to Domain Model
        let role = match user_info.role.as_str() {
            "SchoolManager" => SystemRole::SchoolManager,
            "Teacher" => SystemRole::Teacher,
            "Student" => SystemRole::Student,
            "Parent" => SystemRole::Parent,
            _ => SystemRole::Student,
        };

        let user = User {
            id: user_info.id,
            email: user_info.email,
            full_name: None,
            role,
            school_id: None,
            is_active: true,
            created_at: None,
            last_login: None,
        };

        // Token is handled by HttpOnly cookies, we don't need to store it client-side
        let session = UserSession::new(user.clone(), "cookie-session".to_string());

        // Update global state
        Self::update_session(session.clone());

        AuthResult::Success(session)
    }

    /// Logout user and clear session
    /// Uses gloo_net to call logout endpoint which clears cookies
    pub async fn logout() -> Result<(), AuthError> {
        // Call server logout to clear HttpOnly cookies
        let _ = gloo_net::http::Request::post("/api/auth/logout")
            .send()
            .await;

        // Clear global auth state
        AuthState::clear_user();

        Ok(())
    }

    /// Get current authenticated user
    pub async fn get_current_user() -> Result<Option<User>, AuthError> {
        // Check global auth state first (fastest)
        if let Some(user) = AuthState::get_current_user() {
            return Ok(Some(user));
        }

        // Check server (via whoami) - cookies sent automatically
        match whoami().await {
            Ok(user_info) => {
                let role = match user_info.role.as_str() {
                    "SchoolManager" => SystemRole::SchoolManager,
                    "Teacher" => SystemRole::Teacher,
                    "Student" => SystemRole::Student,
                    "Parent" => SystemRole::Parent,
                    _ => SystemRole::Student,
                };

                let user = User {
                    id: user_info.id,
                    email: user_info.email,
                    full_name: None,
                    role,
                    school_id: None,
                    is_active: true,
                    created_at: None,
                    last_login: None,
                };

                AuthState::update_user(user.clone());
                Ok(Some(user))
            }
            Err(_) => Ok(None),
        }
    }

    /// Check if user is authenticated
    pub async fn is_authenticated() -> bool {
        Self::get_current_user().await.unwrap_or(None).is_some()
    }

    /// Refresh the current session
    pub async fn refresh_session() -> Result<UserSession, AuthError> {
        // Server middleware handles refresh automatically via cookies
        let user = Self::get_current_user()
            .await?
            .ok_or(AuthError::SessionExpired)?;

        let session = UserSession::new(user, "cookie-session".to_string());
        Ok(session)
    }

    pub fn update_session(session: UserSession) {
        AuthState::update_user(session.user);
    }
}

/// Global auth state management
pub static CURRENT_USER_STATE: GlobalSignal<Option<User>> = Signal::global(|| None);
pub static IS_INITIALIZING: GlobalSignal<bool> = Signal::global(|| true);

pub struct AuthState;

impl AuthState {
    /// Initialize auth state from server
    pub async fn initialize() -> Result<(), AuthError> {
        // Check server for existing session via cookies
        let _ = AuthProvider::get_current_user().await;
        *IS_INITIALIZING.write() = false;
        Ok(())
    }

    pub fn update_user(user: User) {
        *CURRENT_USER_STATE.write() = Some(user);
    }

    pub fn clear_user() {
        *CURRENT_USER_STATE.write() = None;
    }

    pub fn get_current_user() -> Option<User> {
        CURRENT_USER_STATE.read().clone()
    }

    pub fn is_authenticated() -> bool {
        CURRENT_USER_STATE.read().is_some()
    }
}
