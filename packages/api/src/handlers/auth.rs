use axum::{
    body::Bytes,
    http::header::{HeaderMap, CONTENT_TYPE},
    response::IntoResponse,
    Extension, Json,
};
use axum_extra::extract::cookie::CookieJar;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

use crate::app_state::AppState;
use crate::domain::UserInfo;
use crate::error::AppError;
use crate::session_security::{
    access_cookie, access_removal_cookie, login_rate_limit_key, refresh_cookie,
    refresh_removal_cookie, resolve_active_session, AuthRateLimiter, SessionValidationError,
};

static LOGIN_RATE_LIMITER: Lazy<AuthRateLimiter> = Lazy::new(AuthRateLimiter::default);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub school_id: Option<String>,
    pub exp: usize,
}

impl AuthenticatedUser {
    pub fn user_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub school_id: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Deserialize)]
struct PasswordGrantResponse {
    access_token: String,
    refresh_token: Option<String>,
}

/// Login handler that sets an HttpOnly cookie session after both Supabase token
/// validation and canonical active-account validation in PostgreSQL.
pub async fn login_handler(
    Extension(state): Extension<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    body: Bytes,
) -> Result<axum::response::Response, (axum::http::StatusCode, String)> {
    use axum::http::StatusCode;
    use axum::response::Redirect;

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let is_form_submission = content_type.contains("application/x-www-form-urlencoded");

    let request: LoginRequest = if is_form_submission {
        serde_urlencoded::from_bytes(&body)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid login request".to_string()))?
    } else {
        serde_json::from_slice(&body)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid login request".to_string()))?
    };

    let email = request.email.trim().to_ascii_lowercase();
    if email.is_empty() || request.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Email and password are required".to_string(),
        ));
    }

    // The supported appliance is a single trusted gateway node. Until a
    // verified proxy-address chain exists, do not trust client-supplied IP
    // headers; throttle by normalized account key in the unknown-address bucket.
    let rate_limit_scope = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    let rate_key = login_rate_limit_key(rate_limit_scope, &email);
    if let Err(limit) = LOGIN_RATE_LIMITER.check(&rate_key).await {
        tracing::warn!(
            retry_after_seconds = limit.retry_after_seconds,
            "Login rate limit exceeded"
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Too many login attempts. Try again later.".to_string(),
        ));
    }

    let config = &state.supabase_config;
    let url = format!(
        "{}/auth/v1/token?grant_type=password",
        config.url.trim_end_matches('/')
    );

    let response = state
        .services
        .http_client
        .post(&url)
        .header("apikey", &config.publishable_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({
            "email": email,
            "password": request.password,
        }))
        .send()
        .await
        .map_err(|error| {
            tracing::error!(%error, "Supabase login request failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Authentication service unavailable".to_string(),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        LOGIN_RATE_LIMITER.record_failure(rate_key).await;
        tracing::warn!(%status, "Supabase rejected login credentials");
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".to_string(),
        ));
    }

    let grant = response
        .json::<PasswordGrantResponse>()
        .await
        .map_err(|error| {
            tracing::error!(%error, "Supabase returned an invalid login response");
            (
                StatusCode::BAD_GATEWAY,
                "Authentication service unavailable".to_string(),
            )
        })?;

    let user_info = match verify_and_get_user(&state, &grant.access_token).await {
        Ok(user) => user,
        Err(SessionValidationError::AccountUnavailable) => {
            LOGIN_RATE_LIMITER.record_failure(rate_key).await;
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid email or password".to_string(),
            ));
        }
        Err(SessionValidationError::DependencyUnavailable) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Authentication service unavailable".to_string(),
            ));
        }
    };

    LOGIN_RATE_LIMITER.clear(&rate_key).await;

    let mut jar = jar.add(access_cookie(grant.access_token));
    if let Some(refresh_token_value) = grant.refresh_token.filter(|value| !value.is_empty()) {
        jar = jar.add(refresh_cookie(refresh_token_value));
    }

    tracing::info!(user_id = %user_info.id, role = %user_info.role, "User logged in");

    if is_form_submission {
        Ok((jar, Redirect::to("/dashboard")).into_response())
    } else {
        Ok((jar, Json(LoginResponse { user: user_info })).into_response())
    }
}

pub async fn logout_handler(jar: CookieJar) -> impl IntoResponse {
    let jar = jar
        .add(access_removal_cookie())
        .add(refresh_removal_cookie());
    (jar, Json(serde_json::json!({"message": "Logged out"})))
}

async fn verify_and_get_user(
    state: &AppState,
    token: &str,
) -> Result<UserInfo, SessionValidationError> {
    let claims = state
        .services
        .supabase_service
        .validate_jwt_token(token)
        .await
        .map_err(map_token_validation_error)?;
    let (user_id, _) = state
        .services
        .supabase_service
        .extract_user_from_token(&claims)
        .map_err(map_token_validation_error)?;
    resolve_active_session(state, &user_id)
        .await
        .map(|session| session.user)
}

fn map_token_validation_error(error: AppError) -> SessionValidationError {
    match error {
        AppError::Unauthorized(_) | AppError::Authentication(_) | AppError::Jwt(_) => {
            SessionValidationError::AccountUnavailable
        }
        _ => SessionValidationError::DependencyUnavailable,
    }
}
