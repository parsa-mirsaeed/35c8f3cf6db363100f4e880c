use crate::app_state::AppState;
use crate::domain::UserInfo;
use crate::repositories::RepositoryError;
use axum::http::{
    header::{InvalidHeaderValue, SET_COOKIE},
    HeaderMap, HeaderValue,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

pub const ACCESS_COOKIE_NAME: &str = "access_token";
pub const REFRESH_COOKIE_NAME: &str = "refresh_token";
pub const ACCESS_COOKIE_MAX_AGE: Duration = Duration::minutes(15);
pub const REFRESH_COOKIE_MAX_AGE: Duration = Duration::days(7);

const AUTH_FAILURE_LIMIT: usize = 5;
const AUTH_FAILURE_WINDOW: StdDuration = StdDuration::from_secs(5 * 60);
const MAX_RATE_LIMIT_KEYS: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SessionValidationError {
    #[error("account is unavailable")]
    AccountUnavailable,
    #[error("session dependency is unavailable")]
    DependencyUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitExceeded {
    pub retry_after_seconds: u64,
}

#[derive(Clone, Default)]
pub struct AuthRateLimiter {
    failures: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl AuthRateLimiter {
    pub async fn check(&self, key: &str) -> Result<(), RateLimitExceeded> {
        let now = Instant::now();
        let mut failures = self.failures.lock().await;
        prune_failures(&mut failures, now);

        let Some(attempts) = failures.get(key) else {
            return Ok(());
        };
        if attempts.len() < AUTH_FAILURE_LIMIT {
            return Ok(());
        }

        let retry_after_seconds = attempts
            .front()
            .map(|first| {
                AUTH_FAILURE_WINDOW
                    .saturating_sub(now.saturating_duration_since(*first))
                    .as_secs()
                    .max(1)
            })
            .unwrap_or(1);
        Err(RateLimitExceeded {
            retry_after_seconds,
        })
    }

    pub async fn record_failure(&self, key: String) {
        let now = Instant::now();
        let mut failures = self.failures.lock().await;
        prune_failures(&mut failures, now);
        if failures.len() >= MAX_RATE_LIMIT_KEYS && !failures.contains_key(&key) {
            if let Some(oldest_key) = failures
                .iter()
                .min_by_key(|(_, attempts)| attempts.front().copied().unwrap_or(now))
                .map(|(key, _)| key.clone())
            {
                failures.remove(&oldest_key);
            }
        }
        failures.entry(key).or_default().push_back(now);
    }

    pub async fn clear(&self, key: &str) {
        self.failures.lock().await.remove(key);
    }
}

fn prune_failures(failures: &mut HashMap<String, VecDeque<Instant>>, now: Instant) {
    failures.retain(|_, attempts| {
        while attempts
            .front()
            .is_some_and(|attempt| now.saturating_duration_since(*attempt) >= AUTH_FAILURE_WINDOW)
        {
            attempts.pop_front();
        }
        !attempts.is_empty()
    });
}

pub fn login_rate_limit_key(ip: IpAddr, email: &str) -> String {
    format!("login:{ip}:{}", email.trim().to_ascii_lowercase())
}

pub fn refresh_rate_limit_key(ip: Option<IpAddr>, refresh_token: &str) -> String {
    let token_digest = Sha256::digest(refresh_token.as_bytes());
    let token_fingerprint = token_digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "refresh:{}:{token_fingerprint}",
        ip.map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    )
}

pub async fn resolve_active_session(
    state: &AppState,
    user_id: &str,
) -> Result<UserInfo, SessionValidationError> {
    let user_uuid =
        Uuid::parse_str(user_id).map_err(|_| SessionValidationError::AccountUnavailable)?;
    let user = state
        .services
        .user
        .find_with_role_by_id(user_uuid.into())
        .await
        .map_err(map_repository_error)?;

    if !user.is_active {
        return Err(SessionValidationError::AccountUnavailable);
    }

    let school_id: Uuid = user.school_id.into();
    state
        .services
        .school
        .find_by_id(school_id)
        .await
        .map_err(map_repository_error)?;

    Ok(UserInfo {
        id: user.id.to_string(),
        email: user.email,
        role: user.role_name.to_string(),
    })
}

fn map_repository_error(error: RepositoryError) -> SessionValidationError {
    match error {
        RepositoryError::NotFound { .. } | RepositoryError::Unauthorized => {
            SessionValidationError::AccountUnavailable
        }
        _ => SessionValidationError::DependencyUnavailable,
    }
}

pub fn access_cookie(value: String) -> Cookie<'static> {
    session_cookie(ACCESS_COOKIE_NAME, value, ACCESS_COOKIE_MAX_AGE)
}

pub fn refresh_cookie(value: String) -> Cookie<'static> {
    session_cookie(REFRESH_COOKIE_NAME, value, REFRESH_COOKIE_MAX_AGE)
}

pub fn access_removal_cookie() -> Cookie<'static> {
    removal_cookie(ACCESS_COOKIE_NAME)
}

pub fn refresh_removal_cookie() -> Cookie<'static> {
    removal_cookie(REFRESH_COOKIE_NAME)
}

fn session_cookie(name: &'static str, value: String, max_age: Duration) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, value);
    apply_session_cookie_policy(&mut cookie);
    cookie.set_max_age(max_age);
    cookie
}

fn removal_cookie(name: &'static str) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, String::new());
    apply_session_cookie_policy(&mut cookie);
    cookie.set_max_age(Duration::ZERO);
    cookie
}

fn apply_session_cookie_policy(cookie: &mut Cookie<'static>) {
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(SameSite::Strict);
}

pub fn append_cookie(
    headers: &mut HeaderMap,
    cookie: &Cookie<'_>,
) -> Result<(), InvalidHeaderValue> {
    headers.append(SET_COOKIE, HeaderValue::from_str(&cookie.to_string())?);
    Ok(())
}

pub fn append_session_removals(headers: &mut HeaderMap) -> Result<(), InvalidHeaderValue> {
    append_cookie(headers, &access_removal_cookie())?;
    append_cookie(headers, &refresh_removal_cookie())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_and_removed_cookies_share_the_security_policy() {
        for cookie in [
            access_cookie("access".to_string()),
            refresh_cookie("refresh".to_string()),
            access_removal_cookie(),
            refresh_removal_cookie(),
        ] {
            assert_eq!(cookie.path(), Some("/"));
            assert_eq!(cookie.http_only(), Some(true));
            assert_eq!(cookie.secure(), Some(true));
            assert_eq!(cookie.same_site(), Some(SameSite::Strict));
        }
        assert_eq!(
            access_cookie("access".to_string()).max_age(),
            Some(ACCESS_COOKIE_MAX_AGE)
        );
        assert_eq!(
            refresh_cookie("refresh".to_string()).max_age(),
            Some(REFRESH_COOKIE_MAX_AGE)
        );
        assert!(ACCESS_COOKIE_MAX_AGE < REFRESH_COOKIE_MAX_AGE);
        assert_eq!(access_removal_cookie().max_age(), Some(Duration::ZERO));
        assert_eq!(refresh_removal_cookie().max_age(), Some(Duration::ZERO));
    }

    #[tokio::test]
    async fn rate_limiter_blocks_after_bounded_failures_and_clears_on_success() {
        let limiter = AuthRateLimiter::default();
        let key = "login:127.0.0.1:user@example.test";
        for _ in 0..AUTH_FAILURE_LIMIT {
            assert!(limiter.check(key).await.is_ok());
            limiter.record_failure(key.to_string()).await;
        }
        assert!(limiter.check(key).await.is_err());
        limiter.clear(key).await;
        assert!(limiter.check(key).await.is_ok());
    }

    #[test]
    fn refresh_rate_limit_keys_do_not_contain_tokens() {
        let token = "secret-refresh-token";
        let key = refresh_rate_limit_key(None, token);
        assert!(!key.contains(token));
        assert!(key.starts_with("refresh:unknown:"));
    }
}
