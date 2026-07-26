use axum::http::Method;
use tower_http::cors::{Any, CorsLayer};

/// Create CORS middleware configuration
pub fn create_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any) // In production, specify allowed origins
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_credentials(false) // Changed for Bearer auth flow
}