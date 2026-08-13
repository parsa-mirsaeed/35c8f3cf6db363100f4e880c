use crate::domain::{AuthCredentials, AuthError, AuthResult, SystemRole, User, UserSession};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use api::server_functions::auth_functions::whoami;

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    user: api::domain::UserInfo,
}

fn system_role(role: &str) -> Result<SystemRole, AuthError> {
    match role {
        "PlatformAdmin" => Ok(SystemRole::PlatformAdmin),
        "SchoolManager" => Ok(SystemRole::SchoolManager),
        "Teacher" => Ok(SystemRole::Teacher),
        "Student" => Ok(SystemRole::Student),
        "Parent" => Ok(SystemRole::Parent),
        _ => Err(AuthError::Unauthorized),
    }
}

/// Production authentication provider implementation.
/// Authentication tokens remain in server-issued HttpOnly cookies.
pub struct AuthProvider;

impl AuthProvider {
    pub async fn authenticate(credentials: AuthCredentials) -> AuthResult {
        web_sys::console::log_1(&"AuthProvider::authenticate starting".into());

        let request = LoginRequest {
            email: credentials.email,
            password: credentials.password,
        };

        let response = match gloo_net::http::Request::post("/api/auth/login")
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request).unwrap())
            .expect("Failed to build request")
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                web_sys::console::error_1(
                    &format!("Network error during login: {error}").into(),
                );
                return AuthResult::ServerError(format!("Network error: {error}"));
            }
        };

        if !response.ok() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if status == 401 {
                web_sys::console::warn_1(&"Login rejected: invalid credentials".into());
                return AuthResult::InvalidCredentials;
            }

            web_sys::console::error_1(
                &format!("Login request failed with HTTP {status}").into(),
            );
            return AuthResult::ServerError(error_text);
        }

        let login_response: LoginResponse = match response.json().await {
            Ok(response) => response,
            Err(error) => {
                web_sys::console::error_1(
                    &format!("Failed to parse login response: {error}").into(),
                );
                return AuthResult::ServerError(format!("Failed to parse response: {error}"));
            }
        };

        let user_info = login_response.user;
        let role = match system_role(&user_info.role) {
            Ok(role) => role,
            Err(_) => {
                web_sys::console::error_1(&"Authenticated user has unsupported role".into());
                return AuthResult::ServerError(
                    "Authenticated user has unsupported role".to_string(),
                );
            }
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
        let session = UserSession::new(user.clone(), "cookie-session".to_string());
        Self::update_session(session.clone());
        AuthResult::Success(session)
    }

    pub async fn logout() -> Result<(), AuthError> {
        let _ = gloo_net::http::Request::post("/api/auth/logout")
            .send()
            .await;
        AuthState::clear_user();
        Ok(())
    }

    pub async fn get_current_user() -> Result<Option<User>, AuthError> {
        if let Some(user) = AuthState::get_current_user() {
            return Ok(Some(user));
        }

        match whoami().await {
            Ok(user_info) => {
                let role = system_role(&user_info.role)?;
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

    pub async fn is_authenticated() -> bool {
        Self::get_current_user().await.unwrap_or(None).is_some()
    }

    pub async fn refresh_session() -> Result<UserSession, AuthError> {
        let user = Self::get_current_user()
            .await?
            .ok_or(AuthError::SessionExpired)?;
        Ok(UserSession::new(user, "cookie-session".to_string()))
    }

    pub fn update_session(session: UserSession) {
        AuthState::update_user(session.user);
    }
}

pub static CURRENT_USER_STATE: GlobalSignal<Option<User>> = Signal::global(|| None);
pub static IS_INITIALIZING: GlobalSignal<bool> = Signal::global(|| true);

pub struct AuthState;

impl AuthState {
    pub async fn initialize() -> Result<(), AuthError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_canonical_role_and_rejects_unknown_roles() {
        assert_eq!(system_role("PlatformAdmin"), Ok(SystemRole::PlatformAdmin));
        assert_eq!(system_role("SchoolManager"), Ok(SystemRole::SchoolManager));
        assert_eq!(system_role("Teacher"), Ok(SystemRole::Teacher));
        assert_eq!(system_role("Student"), Ok(SystemRole::Student));
        assert_eq!(system_role("Parent"), Ok(SystemRole::Parent));
        assert_eq!(system_role("database_owner"), Err(AuthError::Unauthorized));
    }
}
