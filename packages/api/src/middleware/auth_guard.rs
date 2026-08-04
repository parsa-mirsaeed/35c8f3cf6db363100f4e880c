use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Extension,
};
use axum_extra::extract::cookie::CookieJar;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::net::SocketAddr;
use tracing::{debug, error, warn};

use crate::app_state::AppState;
use crate::error::AppError;
use crate::session_security::{
    access_cookie, append_cookie, append_session_removals, refresh_cookie,
    refresh_rate_limit_key, resolve_active_session, AuthRateLimiter, SessionValidationError,
    ACCESS_COOKIE_NAME, REFRESH_COOKIE_NAME,
};

static REFRESH_RATE_LIMITER: Lazy<AuthRateLimiter> = Lazy::new(AuthRateLimiter::default);

#[derive(Debug, Deserialize)]
struct RefreshGrantResponse {
    access_token: String,
    refresh_token: Option<String>,
}

/// Validate the HttpOnly cookie session and inject only a canonical, active
/// database identity. A valid token is insufficient when the user, role, or
/// school relationship no longer exists.
pub async fn auth_middleware(
    state: Extension<AppState>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if request.uri().path() == "/api/auth/logout" {
        return Ok(next.run(request).await);
    }

    let mut access_cookie_was_invalid = false;
    if let Some(access_token) = jar.get(ACCESS_COOKIE_NAME) {
        match validate_token_session(&state, access_token.value()).await {
            Ok(user) => {
                request.extensions_mut().insert(user);
                return Ok(next.run(request).await);
            }
            Err(SessionValidationError::AccountUnavailable) => {
                warn!("Valid access session no longer maps to an active account");
                return run_without_session(request, next, true).await;
            }
            Err(SessionValidationError::DependencyUnavailable) => {
                error!("Session validation dependency unavailable");
                return run_without_session(request, next, true).await;
            }
        }
    } else {
        access_cookie_was_invalid = true;
    }

    if let Some(refresh_token) = jar.get(REFRESH_COOKIE_NAME) {
        let remote_ip = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip());
        let rate_key = refresh_rate_limit_key(remote_ip, refresh_token.value());
        if let Err(limit) = REFRESH_RATE_LIMITER.check(&rate_key).await {
            warn!(
                retry_after_seconds = limit.retry_after_seconds,
                "Refresh rate limit exceeded"
            );
            return run_without_session(request, next, true).await;
        }

        let config = &state.supabase_config;
        let url = format!(
            "{}/auth/v1/token?grant_type=refresh_token",
            config.url.trim_end_matches('/')
        );
        let response = state
            .services
            .http_client
            .post(&url)
            .header("apikey", &config.publishable_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({ "refresh_token": refresh_token.value() }))
            .send()
            .await;

        let response = match response {
            Ok(response) => response,
            Err(error) => {
                REFRESH_RATE_LIMITER.record_failure(rate_key).await;
                error!(%error, "Refresh request failed");
                return run_without_session(request, next, true).await;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            REFRESH_RATE_LIMITER.record_failure(rate_key).await;
            warn!(%status, "Refresh token was rejected");
            return run_without_session(request, next, true).await;
        }

        let grant = match response.json::<RefreshGrantResponse>().await {
            Ok(grant) => grant,
            Err(error) => {
                REFRESH_RATE_LIMITER.record_failure(rate_key).await;
                error!(%error, "Refresh provider returned an invalid response");
                return run_without_session(request, next, true).await;
            }
        };
        let Some(new_refresh_token) = grant.refresh_token.filter(|value| !value.is_empty()) else {
            REFRESH_RATE_LIMITER.record_failure(rate_key).await;
            error!("Refresh provider omitted rotated refresh token");
            return run_without_session(request, next, true).await;
        };

        match validate_token_session(&state, &grant.access_token).await {
            Ok(user) => {
                REFRESH_RATE_LIMITER.clear(&rate_key).await;
                request.extensions_mut().insert(user);
                let mut response = next.run(request).await;
                append_cookie(response.headers_mut(), &access_cookie(grant.access_token))
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                append_cookie(
                    response.headers_mut(),
                    &refresh_cookie(new_refresh_token),
                )
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(response);
            }
            Err(SessionValidationError::AccountUnavailable) => {
                REFRESH_RATE_LIMITER.record_failure(rate_key).await;
                warn!("Refreshed token no longer maps to an active account");
                return run_without_session(request, next, true).await;
            }
            Err(SessionValidationError::DependencyUnavailable) => {
                error!("Refreshed session validation dependency unavailable");
                return run_without_session(request, next, true).await;
            }
        }
    }

    debug!("No valid authenticated session found");
    run_without_session(request, next, access_cookie_was_invalid).await
}

async fn validate_token_session(
    state: &AppState,
    token: &str,
) -> Result<crate::domain::UserInfo, SessionValidationError> {
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
    resolve_active_session(state, &user_id).await
}

fn map_token_validation_error(error: AppError) -> SessionValidationError {
    match error {
        AppError::Unauthorized(_) | AppError::Authentication(_) | AppError::Jwt(_) => {
            SessionValidationError::AccountUnavailable
        }
        _ => SessionValidationError::DependencyUnavailable,
    }
}

async fn run_without_session(
    request: Request,
    next: Next,
    clear_cookies: bool,
) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;
    if clear_cookies {
        append_session_removals(response.headers_mut())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(response)
}
