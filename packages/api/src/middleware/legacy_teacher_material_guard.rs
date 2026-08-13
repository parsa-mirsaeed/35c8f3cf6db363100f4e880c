use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::LazyLock};

const LEGACY_TEACHER_MATERIAL_ENDPOINT: &str = "teacher/materials/create";
const CONTENT_SECURITY_POLICY_PREFIX: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' 'wasm-unsafe-eval'";
const CONTENT_SECURITY_POLICY_SUFFIX: &str = "; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; frame-src 'none'; worker-src 'self' blob:; media-src 'self'; manifest-src 'self'";
const REFERRER_POLICY: &str = "strict-origin-when-cross-origin";
const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), payment=(), usb=(), accelerometer=(), gyroscope=(), magnetometer=()";
const MAX_HTML_SECURITY_BODY_BYTES: usize = 8 * 1024 * 1024;

static INLINE_SCRIPT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<script\b(?P<attrs>[^>]*)>(?P<body>.*?)</script\s*>")
        .expect("inline script regex is valid")
});
static SCRIPT_SRC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|\s)src\s*=").expect("script src regex is valid")
});

fn content_security_policy(inline_script_hashes: &[String]) -> String {
    let mut policy = String::with_capacity(
        CONTENT_SECURITY_POLICY_PREFIX.len()
            + CONTENT_SECURITY_POLICY_SUFFIX.len()
            + inline_script_hashes.iter().map(|hash| hash.len() + 1).sum::<usize>(),
    );
    policy.push_str(CONTENT_SECURITY_POLICY_PREFIX);
    for hash in inline_script_hashes {
        policy.push(' ');
        policy.push_str(hash);
    }
    policy.push_str(CONTENT_SECURITY_POLICY_SUFFIX);
    policy
}

fn inline_script_hashes(html: &[u8]) -> Result<Vec<String>, std::str::Utf8Error> {
    let html = std::str::from_utf8(html)?;
    let mut hashes = BTreeSet::new();

    for captures in INLINE_SCRIPT_RE.captures_iter(html) {
        let attrs = captures.name("attrs").map(|value| value.as_str()).unwrap_or("");
        if SCRIPT_SRC_RE.is_match(attrs) {
            continue;
        }

        let body = captures.name("body").map(|value| value.as_str()).unwrap_or("");
        let digest = Sha256::digest(body.as_bytes());
        hashes.insert(format!("'sha256-{}'", STANDARD.encode(digest)));
    }

    Ok(hashes.into_iter().collect())
}

fn insert_common_security_headers(response: &mut Response, csp: &str) -> Result<(), HeaderValue> {
    let csp = HeaderValue::from_str(csp).map_err(|_| HeaderValue::from_static("invalid-csp"))?;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_SECURITY_POLICY, csp);
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
    Ok(())
}

async fn apply_security_headers(response: Response) -> Response {
    let is_html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase().starts_with("text/html"))
        .unwrap_or(false);

    if !is_html {
        let mut response = response;
        let csp = content_security_policy(&[]);
        if insert_common_security_headers(&mut response, &csp).is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let body = match to_bytes(body, MAX_HTML_SECURITY_BODY_BYTES).await {
        Ok(body) => body,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let hashes = match inline_script_hashes(&body) {
        Ok(hashes) => hashes,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let csp = content_security_policy(&hashes);

    // The original Dioxus response is streamed. Once bounded HTML collection is
    // complete, discard any stale content length before rebuilding the body.
    parts.headers.remove(header::CONTENT_LENGTH);
    let mut response = Response::from_parts(parts, Body::from(body));
    if insert_common_security_headers(&mut response, &csp).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    response
}

pub async fn block_legacy_teacher_material_ingestion(request: Request, next: Next) -> Response {
    let response = if is_legacy_teacher_material_path(request.uri().path()) {
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
    apply_security_headers(response).await
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
        let policy = content_security_policy(&[]);
        assert!(policy.contains("default-src 'self'"));
        assert!(policy.contains("connect-src 'self'"));
        assert!(policy.contains("script-src 'self' 'wasm-unsafe-eval'"));
        assert!(!policy.contains("'unsafe-eval'"));
        assert!(!policy.contains("'unsafe-inline'"));
        assert!(!policy.contains("http://"));
        assert!(!policy.contains("https://"));
    }

    #[test]
    fn csp_hashes_each_inline_script_and_ignores_external_scripts() {
        let html = br#"<!doctype html><html><body>
            <script>window.dx_hydrate = () => {};</script>
            <script src="/assets/app.js"></script>
            <script>window.initial_dioxus_hydration_data="dynamic";</script>
        </body></html>"#;

        let hashes = inline_script_hashes(html).expect("fixture is utf-8");
        assert_eq!(hashes.len(), 2);

        let first = format!(
            "'sha256-{}'",
            STANDARD.encode(Sha256::digest(b"window.dx_hydrate = () => {};"))
        );
        let second = format!(
            "'sha256-{}'",
            STANDARD.encode(Sha256::digest(
                b"window.initial_dioxus_hydration_data=\"dynamic\";"
            ))
        );
        assert!(hashes.contains(&first));
        assert!(hashes.contains(&second));

        let policy = content_security_policy(&hashes);
        assert!(policy.contains(&first));
        assert!(policy.contains(&second));
        assert!(!policy.contains("'unsafe-inline'"));
        assert!(!policy.contains("'unsafe-eval'"));
    }

    #[test]
    fn duplicate_inline_scripts_emit_one_hash() {
        let html = b"<script>same()</script><script>same()</script>";
        let hashes = inline_script_hashes(html).expect("fixture is utf-8");
        assert_eq!(hashes.len(), 1);
    }
}
