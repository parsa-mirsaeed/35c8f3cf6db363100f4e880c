use dioxus::fullstack::extract;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// NOTE: Ensure these types are available in your API crate
// If you have a shared crate, import from there. 
// If they are in this crate, keep the usage.
use crate::domain::UserInfo; 

#[cfg(feature = "server")]
use tracing::error as tracing_error;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginError {
    pub error: String,
}

// FIXED: Added explicit endpoint to prevent 405 errors
#[server(endpoint = "auth/verify")]
pub async fn verify_token(token: String) -> Result<LoginResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        // Ensure verifier is initialized
        let verifier = match crate::supabase_auth::get_supabase_verifier().await {
            Ok(v) => v,
            Err(_) => {
                let cfg = crate::config::Config::from_env().map_err(|e| ServerFnError::new(format!("config error: {}", e)))?;
                crate::supabase_auth::init_supabase_verifier(&cfg.supabase.project_ref, &cfg.supabase.audience).await;
                crate::supabase_auth::get_supabase_verifier()
                    .await
                    .map_err(|e| ServerFnError::new(format!("Failed to init Supabase verifier: {}", e)))?
            }
        };

        // First verify the token with Supabase to get basic auth info
        let auth_user_info = verifier
            .verify(&token)
            .await
            .map_err(|e| ServerFnError::new(format!("Invalid token: {}", e)))?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("Failed to access app state: {}", e)))?;
        
        let user_repo = crate::repositories::user_repository::UserRepository::new(state.services.pool.clone());
        
        // Parse the Supabase user ID as UserId
        let user_id: crate::domain::UserId = uuid::Uuid::parse_str(&auth_user_info.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID format: {}", e)))?
            .into();
        
        // Fetch user with role from database
        let user_with_role = match user_repo.find_with_role_by_id(user_id).await {
            Ok(user) => user,
            Err(crate::repositories::base::RepositoryError::NotFound { .. }) => {
                tracing::error!("User not found in database: {} (email: {})", auth_user_info.id, auth_user_info.email);
                return Err(ServerFnError::new("User not found in database. Please ensure the user is registered."));
            }
            Err(crate::repositories::base::RepositoryError::Database(e)) => {
                tracing::error!("Database error fetching user: {}", e);
                return Err(ServerFnError::new(format!("Database error: {}", e)));
            }
            Err(e) => {
                tracing::error!("Unexpected error fetching user: {}", e);
                return Err(ServerFnError::new(format!("Unexpected error: {}", e)));
            }
        };
        
        // Check if user is active
        if !user_with_role.is_active {
            return Err(ServerFnError::new("User account is inactive"));
        }
        
        // Build the response using the role from database
        let user_info = crate::domain::UserInfo {
            id: auth_user_info.id,
            email: auth_user_info.email,
            role: user_with_role.role_name.to_string(), // Role from database
        };

        Ok(LoginResponse { token, user: user_info })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}

#[cfg_attr(not(feature = "server"), allow(unused))]
#[derive(Debug, serde::Deserialize)]
struct PasswordGrantResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
}

// NOTE: login_user server function has been REPLACED by an explicit Axum handler
// at /api/auth/login in handlers/auth.rs. This is because Dioxus server functions
// cannot properly propagate Set-Cookie headers to the browser. The Axum handler
// returns (CookieJar, Json<T>) which correctly sets HttpOnly cookies.
//
// See: packages/api/src/handlers/auth.rs::login_handler


/// Refresh tokens (internal use - not exposed to client)
#[server(endpoint = "auth/refresh")]
pub async fn refresh(refresh_token: String) -> Result<AuthResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if refresh_token.is_empty() {
            return Err(ServerFnError::new("refresh_token is required"));
        }
        
        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("Failed to access app state: {}", e)))?;
            
        let cfg = &state.supabase_config;
        let url = format!("{}/auth/v1/token?grant_type=refresh_token", cfg.url.trim_end_matches('/'));
        
        // Use shared client
        let client = &state.services.http_client;
        
        let resp = client
            .post(&url)
            .header("apikey", &cfg.publishable_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({ "refresh_token": refresh_token }))
            .send()
            .await
            .map_err(|e| ServerFnError::new(format!("Refresh request failed: {}", e)))?;
            
        if !resp.status().is_success() {
            return Err(ServerFnError::new(format!("Refresh failed: {}", resp.status())));
        }
        let pg: PasswordGrantResponse = resp
            .json()
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to parse refresh response: {}", e)))?;
        
        Ok(AuthResponse {
            access_token: pg.access_token,
            refresh_token: pg.refresh_token,
            expires_in: pg.expires_in,
        })
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Refresh must be called on server"))
    }
}

// NOTE: logout server function has been REPLACED by an explicit Axum handler
// at /api/auth/logout in handlers/auth.rs. This is because Dioxus server functions
// cannot properly remove cookies via Set-Cookie headers.
//
// See: packages/api/src/handlers/auth.rs::logout_handler


/// Check authentication status based on cookies
/// UPDATED: No longer requires token parameter - uses cookies automatically
#[server(endpoint = "auth/whoami")]
pub async fn whoami() -> Result<crate::domain::UserInfo, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;
        
        // The middleware has already validated tokens and injected UserInfo
        let Extension(user): Extension<crate::domain::UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
            
        Ok(user)
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("whoami must be called on server"))
    }
}