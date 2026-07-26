use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

const LEGACY_TEACHER_MATERIAL_ENDPOINT: &str = "teacher/materials/create";

/// Reject the retired teacher material-ingestion endpoint before Dioxus decodes
/// the request body or any PDF extraction/vectorization work can begin.
pub async fn block_legacy_teacher_material_ingestion(request: Request, next: Next) -> Response {
    if is_legacy_teacher_material_path(request.uri().path()) {
        return (
            StatusCode::GONE,
            Json(json!({
                "error": "teacher_document_ingestion_retired",
                "message": "Submit source documents through the school-manager knowledge workflow and use them after platform publication."
            })),
        )
            .into_response();
    }

    next.run(request).await
}

fn is_legacy_teacher_material_path(path: &str) -> bool {
    path.trim_end_matches('/')
        .ends_with(LEGACY_TEACHER_MATERIAL_ENDPOINT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_the_retired_create_endpoint() {
        assert!(is_legacy_teacher_material_path(
            "/api/teacher/materials/create"
        ));
        assert!(is_legacy_teacher_material_path(
            "/api/teacher/materials/create/"
        ));
        assert!(!is_legacy_teacher_material_path(
            "/api/teacher/materials/list"
        ));
        assert!(!is_legacy_teacher_material_path(
            "/api/manager/knowledge-submissions"
        ));
    }
}
