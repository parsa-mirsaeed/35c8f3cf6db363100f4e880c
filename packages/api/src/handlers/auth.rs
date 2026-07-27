use serde::{Deserialize, Serialize};
use axum::{
    Extension,
    Json,
    response::IntoResponse,
    http::header::{HeaderMap, CONTENT_TYPE, LOCATION},
    body::Bytes,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration;

use crate::app_state::AppState;
use crate::domain::UserInfo;

/// Authenticated user information extracted from JWT token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub school_id: Option<String>,
    pub exp: usize, // Expiration time
}

impl AuthenticatedUser {
    /// Convenience method to get user_id (alias for id)
    pub fn user_id(&self) -> &str {
        &self.id
    }
}

/// Login request payload
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response with user info (token handled via cookies)
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserInfo,
}

/// Token claims for JWT
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub email: String,
    pub role: String,
    pub school_id: Option<String>,
    pub exp: usize, // Expiration time
    pub iat: usize, // Issued at
}

/// Internal response type for Supabase auth
#[derive(Debug, Deserialize)]
struct PasswordGrantResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

/// Login handler that properly sets HttpOnly cookies in the response.
/// 
/// Supports both JSON and form-urlencoded content types for progressive enhancement:
/// - Before WASM hydrates: native form POST with redirect response
/// - After WASM hydrates: JSON POST with JSON response
/// 
/// **CRITICAL**: This handler returns `impl IntoResponse` which includes
/// the `CookieJar`. This ensures cookies are actually set in the HTTP
/// response headers, unlike the previous server function approach where
/// the modified jar was dropped.
/// 
/// **RFC 6265 Compliance**: Cookies set with HttpOnly, Secure, SameSite=Strict
/// **OWASP ASVS 4.0.3 V3.2.1**: Session tokens not exposed via URL parameters
pub async fn login_handler(
    Extension(state): Extension<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    body: Bytes,
) -> Result<axum::response::Response, (axum::http::StatusCode, String)> {
    use axum::http::StatusCode;
    use axum::response::{Redirect, Response};
    
    // Detect content type to determine how to parse and respond
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    let is_form_submission = content_type.contains("application/x-www-form-urlencoded");
    
    // Parse request based on content type
    let req: LoginRequest = if is_form_submission {
        serde_urlencoded::from_bytes(&body)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid form data: {}", e)))?
    } else {
        serde_json::from_slice(&body)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?
    };
    
    tracing::debug!("Login handler: attempting login for {} (form: {})", req.email, is_form_submission);
    
    if req.email.is_empty() || req.password.is_empty() {
        if is_form_submission {
            // For form submissions, redirect back to login with error
            return Err((StatusCode::BAD_REQUEST, "Email and password are required".to_string()));
        }
        return Err((StatusCode::BAD_REQUEST, "Email and password are required".to_string()));
    }

    let cfg = &state.supabase_config;
    let url = format!("{}/auth/v1/token?grant_type=password", cfg.url.trim_end_matches('/'));
    
    // Use shared HTTP client
    let client = &state.services.http_client;
    
    let resp = client
        .post(&url)
        .header("apikey", &cfg.publishable_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({
            "email": req.email,
            "password": req.password,
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Auth request failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!("Login failed for {}: {} {}", req.email, status, body);
        return Err((StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()));
    }

    let pg: PasswordGrantResponse = resp
        .json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse auth response: {}", e)))?;

    // Verify token and get user info with role from database
    let user_info = verify_and_get_user(&state, &pg.access_token).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Create HttpOnly cookies - these WILL be set because CookieJar implements IntoResponse
    let mut access_cookie = Cookie::new("access_token", pg.access_token);
    access_cookie.set_path("/");
    access_cookie.set_http_only(true);
    access_cookie.set_secure(true);
    access_cookie.set_same_site(SameSite::Strict);
    access_cookie.set_max_age(Duration::minutes(15));

    let mut jar = jar.add(access_cookie);

    if let Some(refresh) = pg.refresh_token {
        let mut refresh_cookie = Cookie::new("refresh_token", refresh);
        refresh_cookie.set_path("/");
        refresh_cookie.set_http_only(true);
        refresh_cookie.set_secure(true);
        refresh_cookie.set_same_site(SameSite::Strict);
        refresh_cookie.set_max_age(Duration::days(7));
        jar = jar.add(refresh_cookie);
    }

    tracing::info!("User logged in successfully: {} ({})", user_info.email, user_info.role);
    
    // For form submissions (before WASM hydrates), redirect to dashboard
    // For JSON requests (SPA mode), return JSON response
    if is_form_submission {
        // Redirect to dashboard - cookies are still set via CookieJar
        Ok((jar, Redirect::to("/dashboard")).into_response())
    } else {
        // Return JSON response for SPA mode
        Ok((jar, Json(LoginResponse { user: user_info })).into_response())
    }
}

/// Logout handler that clears authentication cookies
pub async fn logout_handler(
    jar: CookieJar,
) -> impl IntoResponse {
    tracing::debug!("Logout handler: clearing cookies");
    
    // Create removal cookies (set max-age to 0)
    let mut access_removal = Cookie::new("access_token", "");
    access_removal.set_path("/");
    access_removal.set_max_age(Duration::seconds(0));
    
    let mut refresh_removal = Cookie::new("refresh_token", "");
    refresh_removal.set_path("/");
    refresh_removal.set_max_age(Duration::seconds(0));
    
    let jar = jar
        .add(access_removal)
        .add(refresh_removal);
    
    (jar, Json(serde_json::json!({"message": "Logged out successfully"})))
}

/// Helper function to verify token and fetch user with role from database
async fn verify_and_get_user(state: &AppState, token: &str) -> Result<UserInfo, String> {
    // Verify token with Supabase
    let claims = state.services.supabase_service
        .validate_jwt_token(token)
        .await
        .map_err(|e| format!("Invalid token: {}", e))?;
    
    let (user_id, email) = state.services.supabase_service
        .extract_user_from_token(&claims)
        .map_err(|e| format!("Failed to extract user info: {}", e))?;
    
    // Fetch role from database
    let user_uuid = uuid::Uuid::parse_str(&user_id)
        .map_err(|e| format!("Invalid user ID format: {}", e))?;
    
    let db_user = state.services.user
        .find_with_role_by_id(user_uuid.into())
        .await
        .map_err(|e| format!("User not found in database: {}", e))?;
    
    if !db_user.is_active {
        return Err("User account is inactive".to_string());
    }
    
    Ok(UserInfo {
        id: user_id,
        email,
        role: db_user.role_name.to_string(),
    })
}