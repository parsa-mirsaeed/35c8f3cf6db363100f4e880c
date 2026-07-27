use crate::app_state::AppState;
use crate::domain::UserInfo;
use crate::services::supabase_auth::SupabaseAdminService;
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response, Extension};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration;
use tracing::{debug, error, warn};
use uuid::Uuid;

/// The Gatekeeper: Intercepts all requests to validate authentication.
///
/// **Flow (OWASP ASVS 4.0.3 Compliant):**
/// 1. Check for valid `access_token` cookie.
/// 2. If valid, validate JWT and fetch user role from database.
/// 3. If invalid/expired, check for `refresh_token` cookie.
/// 4. If refresh token exists, attempt to refresh via Supabase.
/// 5. If refresh succeeds, rotate cookies and inject UserInfo.
/// 6. If all fails, proceed without user (public routes) or return 401 (protected routes handle this).
///
/// **Security Properties:**
/// - HttpOnly cookies prevent XSS token theft
/// - Automatic token rotation on refresh
/// - Database-sourced roles (not JWT claims)
/// - Fail-safe: unauthenticated requests proceed to handlers which enforce authorization
pub async fn auth_middleware(
    state: Extension<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_service = &state.services.supabase_service;

    // 0. Skip auth check for logout specific route to allow signing out even if DB is full
    if req.uri().path() == "/api/auth/logout" {
        return Ok(next.run(req).await);
    }

    // 1. Try Access Token
    if let Some(access_token) = jar.get("access_token") {
        debug!("Found access_token cookie, validating...");

        if let Ok(claims) = auth_service.validate_jwt_token(access_token.value()).await {
            debug!("Access token valid, extracting user info...");

            if let Ok((user_id, email)) = auth_service.extract_user_from_token(&claims) {
                // Fetch role from DB to ensure consistency (CRITICAL: Don't trust JWT claims for roles)
                if let Ok(user_uuid) = Uuid::parse_str(&user_id) {
                    match state
                        .services
                        .user
                        .find_with_role_by_id(user_uuid.into())
                        .await
                    {
                        Ok(db_user) => {
                            let user_info = UserInfo {
                                id: user_id.clone(),
                                email: email.clone(),
                                role: db_user.role_name.to_string(),
                            };
                            debug!("User authenticated: {} ({})", email, db_user.role_name);
                            req.extensions_mut().insert(user_info);
                            return Ok(next.run(req).await);
                        }
                        Err(e) => {
                            warn!("User {} found in JWT but not in database: {}", user_id, e);
                            // Token valid but user doesn't exist in DB - proceed without clearing
                            // (cookies will be cleared on next login attempt)
                        }
                    }
                }
            }
        } else {
            debug!("Access token validation failed, will try refresh token");
        }
    }

    // 2. Access Token Failed? Try Refresh Token
    if let Some(refresh_token) = jar.get("refresh_token") {
        debug!("Attempting token refresh...");

        let cfg = &state.supabase_config;
        let client = &state.services.http_client;

        let url = format!(
            "{}/auth/v1/token?grant_type=refresh_token",
            cfg.url.trim_end_matches('/')
        );

        match client
            .post(&url)
            .header("apikey", &cfg.publishable_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "refresh_token": refresh_token.value() }))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let (Some(new_access), Some(new_refresh)) = (
                        json.get("access_token").and_then(|v| v.as_str()),
                        json.get("refresh_token").and_then(|v| v.as_str()),
                    ) {
                        debug!("Token refresh successful, rotating cookies...");

                        // 3. Create new cookies
                        let mut access_cookie = Cookie::new("access_token", new_access.to_string());
                        access_cookie.set_path("/");
                        access_cookie.set_http_only(true);
                        access_cookie.set_secure(true);
                        access_cookie.set_same_site(SameSite::Strict);
                        access_cookie.set_max_age(Duration::minutes(15));

                        let mut refresh_cookie =
                            Cookie::new("refresh_token", new_refresh.to_string());
                        refresh_cookie.set_path("/");
                        refresh_cookie.set_http_only(true);
                        refresh_cookie.set_secure(true);
                        refresh_cookie.set_same_site(SameSite::Strict);
                        refresh_cookie.set_max_age(Duration::days(7));

                        // 4. Inject User Info from New Token
                        if let Ok(claims) = auth_service.validate_jwt_token(new_access).await {
                            if let Ok((user_id, email)) =
                                auth_service.extract_user_from_token(&claims)
                            {
                                if let Ok(user_uuid) = Uuid::parse_str(&user_id) {
                                    if let Ok(db_user) = state
                                        .services
                                        .user
                                        .find_with_role_by_id(user_uuid.into())
                                        .await
                                    {
                                        let user_info = UserInfo {
                                            id: user_id,
                                            email,
                                            role: db_user.role_name.to_string(),
                                        };
                                        debug!(
                                            "User re-authenticated via refresh: {}",
                                            user_info.email
                                        );
                                        req.extensions_mut().insert(user_info);

                                        // Run the request and append cookies to response
                                        let mut response = next.run(req).await;
                                        response.headers_mut().append(
                                            axum::http::header::SET_COOKIE,
                                            access_cookie.to_string().parse().unwrap(),
                                        );
                                        response.headers_mut().append(
                                            axum::http::header::SET_COOKIE,
                                            refresh_cookie.to_string().parse().unwrap(),
                                        );
                                        return Ok(response);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(resp) => {
                warn!("Token refresh failed with status: {}", resp.status());
            }
            Err(e) => {
                error!("Token refresh request failed: {}", e);
            }
        }
    }

    // 3. No Valid Auth - Proceed Without User
    // Protected routes will check for UserInfo extension and return 401 if missing
    // This allows public routes to work without authentication
    debug!("No valid authentication found, proceeding without user context");
    Ok(next.run(req).await)
}
