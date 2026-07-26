use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// Import server functions
use api::server_functions::auth_functions::whoami;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthState {
    pub is_authenticated: bool,
    pub user: Option<api::domain::UserInfo>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            is_authenticated: false,
            user: None,
        }
    }
}

/// Primary authentication hook
/// Automatically checks session on mount via server-side cookies
pub fn use_auth() -> Resource<Option<api::domain::UserInfo>> {
    use_resource(move || async move {
        // Call server whoami - cookies are sent automatically by browser
        // No token parameter needed - middleware handles auth via HttpOnly cookies
        match whoami().await {
            Ok(user) => Some(user),
            Err(_) => None,
        }
    })
}

/// Global logout action
/// Uses HTTP request to logout endpoint which clears HttpOnly cookies
pub fn trigger_logout() {
    spawn(async move {
        // Call logout endpoint via HTTP - cookies are cleared by server
        let _ = gloo_net::http::Request::post("/api/auth/logout")
            .send()
            .await;
        let nav = use_navigator();
        nav.replace("/");
    });
}