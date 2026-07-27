use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{error, info, warn};

/// Request logging middleware
pub async fn request_logging_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // Get user info if available
    let user_info = request.extensions().get::<crate::handlers::auth::AuthenticatedUser>()
        .map(|u| format!("{} ({})", u.name, u.email))
        .unwrap_or_else(|| "anonymous".to_string());

    info!(
        method = %method,
        uri = %uri,
        user = %user_info,
        user_agent = %user_agent,
        "Request started"
    );

    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status();

    if status.is_server_error() {
        error!(
            method = %method,
            uri = %uri,
            user = %user_info,
            status = %status,
            duration_ms = %duration.as_millis(),
            "Request completed with server error"
        );
    } else if status.is_client_error() {
        warn!(
            method = %method,
            uri = %uri,
            user = %user_info,
            status = %status,
            duration_ms = %duration.as_millis(),
            "Request completed with client error"
        );
    } else {
        info!(
            method = %method,
            uri = %uri,
            user = %user_info,
            status = %status,
            duration_ms = %duration.as_millis(),
            "Request completed successfully"
        );
    }

    response
}

/// Audit logging middleware
pub async fn audit_logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Get user info if available
    let user = request.extensions().get::<crate::handlers::auth::AuthenticatedUser>().cloned();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let ip = extract_client_ip(&request);

    // Extract user_agent before moving request
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown").to_string();

    let response = next.run(request).await;
    let status = response.status();

    // Log audit information for sensitive operations
    if is_sensitive_operation(&method, &uri, status) {
        if let Some(user) = user {
            info!(
                user_id = %user.user_id(),
                user_email = %user.email,
                user_role = %user.role,
                method = %method,
                uri = %uri,
                status = %status,
                ip = %ip,
                user_agent = %user_agent,
                "Sensitive operation performed"
            );
        } else {
            warn!(
                method = %method,
                uri = %uri,
                status = %status,
                ip = %ip,
                user_agent = %user_agent,
                "Sensitive operation attempted without authentication"
            );
        }
    }

    response
}

/// Extract client IP from request
fn extract_client_ip(request: &Request) -> String {
    // Try to get IP from common headers
    let headers = request.headers();

    // Check X-Forwarded-For header (for reverse proxies)
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            // X-Forwarded-For can contain multiple IPs, take the first one
            return xff_str.split(',').next().unwrap_or("unknown").trim().to_string();
        }
    }

    // Check X-Real-IP header
    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(xri_str) = xri.to_str() {
            return xri_str.to_string();
        }
    }

    // Fall back to remote address
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Check if operation should be audited
fn is_sensitive_operation(method: &axum::http::Method, uri: &axum::http::Uri, status: StatusCode) -> bool {
    // Audit all write operations (POST, PUT, DELETE, PATCH)
    if method == axum::http::Method::POST
        || method == axum::http::Method::PUT
        || method == axum::http::Method::DELETE
        || method == axum::http::Method::PATCH
    {
        return true;
    }

    // Audit authentication-related operations
    if uri.path().contains("/auth/") {
        return true;
    }

    // Audit admin operations
    if uri.path().contains("/admin/") {
        return true;
    }

    // Audit profile operations
    if uri.path().contains("/profile/") {
        return true;
    }

    // Audit failed operations
    if status.is_client_error() || status.is_server_error() {
        return true;
    }

    false
}