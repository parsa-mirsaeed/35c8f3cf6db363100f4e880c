use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Application result type
pub type AppResult<T> = Result<T, AppError>;

/// Application error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Supabase error: {0}")]
    SupabaseError(String),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Repository error: {0}")]
    Repository(#[from] crate::repositories::base::RepositoryError),
}

impl AppError {
    pub fn unprocessable_entity(msg: String) -> Self {
        AppError::Validation(msg)
    }

    pub fn bad_request(msg: &str) -> Self {
        AppError::BadRequest(msg.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Authorization(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::SupabaseError(msg) => {
                tracing::error!("Supabase error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "External service error".to_string(),
                )
            }
            AppError::Jwt(err) => {
                tracing::error!("JWT error: {:?}", err);
                (
                    StatusCode::UNAUTHORIZED,
                    "Invalid authentication token".to_string(),
                )
            }
            AppError::Repository(err) => {
                tracing::error!("Repository error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

impl From<AppError> for dioxus::prelude::ServerFnError {
    fn from(err: AppError) -> Self {
        dioxus::prelude::ServerFnError::new(err.to_string())
    }
}
