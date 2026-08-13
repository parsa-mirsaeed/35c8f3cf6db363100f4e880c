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
use std::{net::SocketAddr, sync::Arc};
use tracing::{debug, error, warn};

use crate::app_state::AppState;
use crate::error::AppError;
use crate::rls_context::{AuthorizedActor, AuthorizedTx};
use crate::session_security::{
    access_cookie, append_cookie, append_session_removals, refresh_cookie, refresh_rate_limit_key,
    resolve_active_session, AuthRateLimiter, SessionValidationError, ACCESS_COOKIE_NAME,
    REFRESH_COOKIE_NAME,
};

static REFRESH_RATE_LIMITER: Lazy<AuthRateLimiter> = Lazy::new(AuthRateLimiter::default);

const LOGIN_PATH: &str = "/api/auth/login";
const LOGOUT_PATH: &str = "/api/auth/logout";

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
    if bypass_session_middleware(request.uri().path()) {
        return Ok(next.run(request).await);
    }

    let mut clear_invalid_cookies = false;
    if let Some(access_token) = jar.get(ACCESS_COOKIE_NAME) {
        match token_user_id(&state, access_token.value()).await {
            Ok(user_id) => match resolve_active_session(&state, &user_id).await {
                Ok(session) => {
                    request.extensions_mut().insert(session.user.clone());
                    return run_with_session(&state, request, next, session).await;
                }
                Err(SessionValidationError::AccountUnavailable) => {
                    warn!("Access token belongs to a disabled, deleted, or invalid account");
                    return run_without_session(request, next, true).await;
                }
                Err(SessionValidationError::DependencyUnavailable) => {
                    error!("Session validation dependency unavailable");
                    return run_without_session(request, next, false).await;
                }
            },
            Err(SessionValidationError::AccountUnavailable) => {
                debug!("Access token is invalid or expired; attempting refresh");
                clear_invalid_cookies = true;
            }
            Err(SessionValidationError::DependencyUnavailable) => {
                error!("Access-token validation dependency unavailable");
                return run_without_session(request, next, false).await;
            }
        }
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
            return run_without_session(request, next, false).await;
        }

        let config = &state.supabase_config;
        let url = format!(
            "{}/auth/v1/token?grant_type=refresh_token",
            config.url.trim_end_matches('/')
        );
        let provider_response = state
            .services
            .http_client
            .post(&url)
            .header("apikey", &config.publishable_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({ "refresh_token": refresh_token.value() }))
            .send()
            .await;

        let provider_response = match provider_response {
            Ok(response) => response,
            Err(error) => {
                error!(%error, "Refresh request dependency unavailable");
                return run_without_session(request, next, false).await;
            }
        };

        if !provider_response.status().is_success() {
            let status = provider_response.status();
            let status_code = status.as_u16();
            let invalidates_session = refresh_rejection_invalidates_session(status_code);
            if refresh_response_records_failure(status_code) {
                REFRESH_RATE_LIMITER.record_failure(rate_key).await;
            }
            if invalidates_session {
                warn!(%status, "Refresh token was rejected");
                return run_without_session(request, next, true).await;
            }

            if status_code == 429 {
                warn!(%status, "Refresh provider rate limited the session");
            } else {
                error!(%status, "Refresh provider dependency unavailable");
            }
            return run_without_session(request, next, false).await;
        }

        let grant = match provider_response.json::<RefreshGrantResponse>().await {
            Ok(grant) => grant,
            Err(error) => {
                error!(%error, "Refresh provider returned an invalid response");
                return run_without_session(request, next, false).await;
            }
        };
        let Some(new_refresh_token) = grant.refresh_token.filter(|value| !value.is_empty()) else {
            error!("Refresh provider omitted rotated refresh token");
            return run_without_session(request, next, false).await;
        };

        let user_id = match token_user_id(&state, &grant.access_token).await {
            Ok(user_id) => user_id,
            Err(SessionValidationError::AccountUnavailable) => {
                error!("Refresh provider returned an invalid access token");
                return run_without_session(request, next, false).await;
            }
            Err(SessionValidationError::DependencyUnavailable) => {
                error!("Refreshed-token validation dependency unavailable");
                return run_without_session(request, next, false).await;
            }
        };

        match resolve_active_session(&state, &user_id).await {
            Ok(session) => {
                REFRESH_RATE_LIMITER.clear(&rate_key).await;
                request.extensions_mut().insert(session.user.clone());
                let mut response = run_with_session(&state, request, next, session).await?;
                append_cookie(response.headers_mut(), &access_cookie(grant.access_token))
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                append_cookie(response.headers_mut(), &refresh_cookie(new_refresh_token))
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(response);
            }
            Err(SessionValidationError::AccountUnavailable) => {
                REFRESH_RATE_LIMITER.record_failure(rate_key).await;
                warn!("Refresh token belongs to a disabled, deleted, or invalid account");
                return run_without_session(request, next, true).await;
            }
            Err(SessionValidationError::DependencyUnavailable) => {
                error!("Refreshed session validation dependency unavailable");
                return run_without_session(request, next, false).await;
            }
        }
    }

    debug!("No valid authenticated session found");
    run_without_session(request, next, clear_invalid_cookies).await
}

fn bypass_session_middleware(path: &str) -> bool {
    matches!(path, LOGIN_PATH | LOGOUT_PATH)
}

fn refresh_rejection_invalidates_session(status: u16) -> bool {
    matches!(status, 400 | 401 | 403 | 422)
}

fn refresh_response_records_failure(status: u16) -> bool {
    refresh_rejection_invalidates_session(status) || status == 429
}

async fn run_with_session(
    state: &AppState,
    mut request: Request,
    next: Next,
    session: crate::session_security::ActiveSession,
) -> Result<Response, StatusCode> {
    let user_id = uuid::Uuid::parse_str(&session.user.id).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let actor = AuthorizedActor::new(user_id, session.user.role.clone(), Some(session.school_id))
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let tx = AuthorizedTx::begin(&state.services.raw_pool, actor)
        .await
        .map_err(|error| {
            error!(%error, "Failed to begin authorized request transaction");
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    request
        .extensions_mut()
        .insert(Arc::new(tx.authorized_pool()));

    tx.scope(next.run(request), |response| {
        !response.status().is_client_error() && !response.status().is_server_error()
    })
    .await
    .map_err(|error| {
        error!(%error, "Failed to finalize authorized request transaction");
        StatusCode::SERVICE_UNAVAILABLE
    })
}

async fn token_user_id(state: &AppState, token: &str) -> Result<String, SessionValidationError> {
    let claims = state
        .services
        .supabase_service
        .validate_jwt_token(token)
        .await
        .map_err(map_token_validation_error)?;
    state
        .services
        .supabase_service
        .extract_user_from_token(&claims)
        .map(|(user_id, _)| user_id)
        .map_err(map_token_validation_error)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_and_logout_bypass_session_cookie_cleanup() {
        assert!(bypass_session_middleware(LOGIN_PATH));
        assert!(bypass_session_middleware(LOGOUT_PATH));
        assert!(!bypass_session_middleware("/api/auth/whoami"));
        assert!(!bypass_session_middleware("/protected"));
    }

    #[test]
    fn only_explicit_refresh_rejections_invalidate_session_cookies() {
        for status in [
            StatusCode::BAD_REQUEST,
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::UNPROCESSABLE_ENTITY,
        ] {
            assert!(
                refresh_rejection_invalidates_session(status.as_u16()),
                "{status}"
            );
        }

        for status in [
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::GATEWAY_TIMEOUT,
        ] {
            assert!(
                !refresh_rejection_invalidates_session(status.as_u16()),
                "{status}"
            );
        }
    }

    #[test]
    fn provider_throttling_feeds_local_backoff_without_invalidating_session() {
        assert!(refresh_response_records_failure(
            StatusCode::TOO_MANY_REQUESTS.as_u16()
        ));
        assert!(!refresh_rejection_invalidates_session(
            StatusCode::TOO_MANY_REQUESTS.as_u16()
        ));
        assert!(!refresh_response_records_failure(
            StatusCode::SERVICE_UNAVAILABLE.as_u16()
        ));
    }
}
