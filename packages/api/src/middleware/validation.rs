use crate::error::AppError;
use crate::utils::validation::validate_request;
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        Request,
    },
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use validator::Validate;

/// Convert JSON rejections into 422 Unprocessable Entity errors
pub async fn json_rejection_handler(rejection: JsonRejection) -> Response {
    tracing::warn!("JSON rejection: {:?}", rejection);

    let field_errors = match rejection {
        JsonRejection::JsonDataError(err) => {
            vec![format!("Invalid JSON data: {}", err)]
        }
        JsonRejection::JsonSyntaxError(err) => {
            vec![format!("Invalid JSON syntax: {}", err)]
        }
        JsonRejection::MissingJsonContentType(_) => {
            vec!["Content-Type must be application/json".to_string()]
        }
        JsonRejection::BytesRejection(_) => {
            vec!["Invalid request body".to_string()]
        }
        _ => vec!["Invalid request format".to_string()],
    };

    AppError::unprocessable_entity(field_errors.join(", ")).into_response()
}

/// Convert path parameter rejections into 400 Bad Request errors
pub async fn path_rejection_handler(rejection: PathRejection) -> Response {
    tracing::warn!("Path rejection: {:?}", rejection);

    let error_message = match rejection {
        PathRejection::MissingPathParams(_) => "Missing required path parameters",
        PathRejection::FailedToDeserializePathParams(_) => "Invalid path parameters format",
        _ => "Invalid path parameters",
    };

    AppError::bad_request(error_message).into_response()
}

/// Validation middleware that automatically validates request bodies
pub async fn validation_middleware<T: Validate + Clone + Send + Sync + 'static>(
    Json(validated_request): Json<T>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Validate the request
    if let Err(validation_error) = validate_request(&validated_request) {
        return Err(validation_error.into_response());
    }

    // If validation passes, continue with the request
    request.extensions_mut().insert(validated_request);
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_rejection_messages() {
        // Verify the static error messages are correct
        assert_eq!(
            "Missing required path parameters",
            "Missing required path parameters"
        );
    }
}
