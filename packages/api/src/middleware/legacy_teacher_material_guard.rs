use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

const LEGACY_TEACHER_MATERIAL_ENDPOINT: &str = "teacher/materials/create";
const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; frame-src 'none'; worker-src 'self' blob:; media-src 'self'; manifest-src 'self'";
const REFERRER_POLICY: &str = "strict-origin-when-cross-origin";
const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), payment=(), usb=(), accelerometer=(), gyroscope=(), magnetometer=()";

fn apply_security_headers(response: &mut Response) {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CONTENT_SECURITY_POLICY),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static(REFERRER_POLICY),
    );
    headers.insert(
        axum::http::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(PERMISSIONS_POLICY),
    );
    if std::env::var("EDUTALENT_ENFORCE_HSTS")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
}

pub async fn block_legacy_teacher_material_ingestion(request: Request, next: Next) -> Response {
    let mut response = if is_legacy_teacher_material_path(request.uri().path()) {
        (
            StatusCode::GONE,
            Json(json!({
                "error": "teacher_document_ingestion_retired",
                "message": "Submit source documents through the school-manager knowledge workflow and use them after platform publication."
            })),
        )
            .into_response()
    } else {
        next.run(request).await
    };
    apply_security_headers(&mut response);
    response
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

    #[test]
    fn production_csp_is_self_contained_and_has_no_remote_origins() {
        assert!(CONTENT_SECURITY_POLICY.contains("default-src 'self'"));
        assert!(CONTENT_SECURITY_POLICY.contains("connect-src 'self'"));
        assert!(CONTENT_SECURITY_POLICY.contains("script-src 'self'"));
        assert!(!CONTENT_SECURITY_POLICY.contains("'unsafe-eval'"));
        assert!(!CONTENT_SECURITY_POLICY.contains("http://"));
        assert!(!CONTENT_SECURITY_POLICY.contains("https://"));
    }
}
