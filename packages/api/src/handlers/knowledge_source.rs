use crate::app_state::AppState;
use crate::domain::UserInfo;
use crate::rls_context::AuthorizedPool;
use axum::{
    body::Body,
    extract::Query,
    http::{
        header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue, StatusCode,
    },
    response::{Json, Response},
    Extension,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{sync::Arc, time::Duration};
use tracing::error;
use uuid::Uuid;

const KNOWLEDGE_SOURCE_BUCKET: &str = "edutalent-knowledge-sources";
const MAX_KNOWLEDGE_PDF_BYTES: usize = 20 * 1024 * 1024;

type SourceRejection = (StatusCode, Json<Value>);

#[derive(Debug, Deserialize)]
pub struct KnowledgeSourceQuery {
    asset_id: String,
}

pub async fn knowledge_source_handler(
    Extension(state): Extension<AppState>,
    Extension(user): Extension<UserInfo>,
    Extension(pool): Extension<Arc<AuthorizedPool>>,
    Query(query): Query<KnowledgeSourceQuery>,
) -> Result<Response, SourceRejection> {
    if user.role != "PlatformAdmin" {
        return Err(reject(StatusCode::FORBIDDEN, "Insufficient role"));
    }

    let actor_id = Uuid::parse_str(&user.id)
        .map_err(|_| reject(StatusCode::UNAUTHORIZED, "Invalid active session"))?;
    let asset_id = Uuid::parse_str(query.asset_id.trim())
        .map_err(|_| reject(StatusCode::BAD_REQUEST, "Invalid asset ID"))?;

    let row = sqlx::query(
        r#"
        SELECT
            ka.school_id,
            source.original_file_url,
            source.original_filename,
            source.mime_type,
            source.file_size_bytes,
            source.sha256
        FROM knowledge_assets ka
        JOIN LATERAL (
            SELECT original_file_url, original_filename, mime_type, file_size_bytes, sha256
            FROM knowledge_source_files
            WHERE asset_id = ka.id
            ORDER BY created_at ASC
            LIMIT 1
        ) source ON TRUE
        WHERE ka.id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(&*pool)
    .await
    .map_err(|error| {
        error!(%error, %asset_id, "knowledge source lookup failed");
        reject(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to load source metadata",
        )
    })?
    .ok_or_else(|| reject(StatusCode::NOT_FOUND, "Source document is unavailable"))?;

    let school_id: Uuid = row.try_get("school_id").map_err(internal_source_error)?;
    let original_file_url: Option<String> = row
        .try_get("original_file_url")
        .map_err(internal_source_error)?;
    let mime_type: String = row.try_get("mime_type").map_err(internal_source_error)?;
    let stored_size: Option<i64> = row
        .try_get("file_size_bytes")
        .map_err(internal_source_error)?;
    let stored_sha256: Option<String> = row.try_get("sha256").map_err(internal_source_error)?;

    if mime_type != "application/pdf" {
        return Err(reject(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Only governed PDF sources can be reviewed",
        ));
    }

    let reference = original_file_url
        .as_deref()
        .ok_or_else(|| reject(StatusCode::NOT_FOUND, "Source document is unavailable"))?;
    let object_key = object_key_for_school(reference, school_id)
        .ok_or_else(|| reject(StatusCode::NOT_FOUND, "Source document is unavailable"))?;

    if stored_size.is_some_and(|size| size < 0 || size as u64 > MAX_KNOWLEDGE_PDF_BYTES as u64) {
        error!(%asset_id, %school_id, "knowledge source metadata exceeds review size limit");
        return Err(reject(
            StatusCode::BAD_GATEWAY,
            "Source document failed integrity checks",
        ));
    }

    let url = format!(
        "{}/storage/v1/object/{KNOWLEDGE_SOURCE_BUCKET}/{object_key}",
        state.supabase_config.url.trim_end_matches('/')
    );
    let mut storage_response = state
        .services
        .http_client
        .get(url)
        .bearer_auth(&state.supabase_config.secret_key)
        .header("apikey", &state.supabase_config.secret_key)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|error| {
            error!(%error, %asset_id, "knowledge source storage read failed");
            reject(StatusCode::BAD_GATEWAY, "Source document is unavailable")
        })?;

    if !storage_response.status().is_success() {
        error!(
            status = %storage_response.status(),
            %asset_id,
            "knowledge source storage rejected review read"
        );
        return Err(reject(
            StatusCode::BAD_GATEWAY,
            "Source document is unavailable",
        ));
    }

    if storage_response
        .content_length()
        .is_some_and(|size| size > MAX_KNOWLEDGE_PDF_BYTES as u64)
    {
        return Err(reject(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Source document is too large to review",
        ));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = storage_response.chunk().await.map_err(|error| {
        error!(%error, %asset_id, "knowledge source storage body read failed");
        reject(StatusCode::BAD_GATEWAY, "Source document is unavailable")
    })? {
        if bytes.len().saturating_add(chunk.len()) > MAX_KNOWLEDGE_PDF_BYTES {
            return Err(reject(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Source document is too large to review",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }

    if !bytes.starts_with(b"%PDF-") {
        error!(%asset_id, "knowledge source content no longer has a PDF signature");
        return Err(reject(
            StatusCode::BAD_GATEWAY,
            "Source document failed integrity checks",
        ));
    }
    if let Some(expected_size) = stored_size {
        if usize::try_from(expected_size).ok() != Some(bytes.len()) {
            error!(
                %asset_id,
                expected_size,
                actual_size = bytes.len(),
                "knowledge source size no longer matches stored metadata"
            );
            return Err(reject(
                StatusCode::BAD_GATEWAY,
                "Source document failed integrity checks",
            ));
        }
    }
    if let Some(expected_sha256) = stored_sha256.as_deref() {
        let actual_sha256 = sha256_hex(&bytes);
        if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
            error!(%asset_id, "knowledge source hash no longer matches stored metadata");
            return Err(reject(
                StatusCode::BAD_GATEWAY,
                "Source document failed integrity checks",
            ));
        }
    }

    sqlx::query(
        r#"
        INSERT INTO knowledge_audit_logs (
            actor_id, actor_role, action, target_type, target_id, school_id, details_json
        ) VALUES ($1, 'PlatformAdmin', 'knowledge_asset.source_reviewed', 'knowledge_asset', $2, $3, $4)
        "#,
    )
    .bind(actor_id)
    .bind(asset_id)
    .bind(school_id)
    .bind(json!({ "delivery": "inline_pdf", "byte_count": bytes.len() }))
    .execute(&*pool)
    .await
    .map_err(|error| {
        error!(%error, %asset_id, "knowledge source review audit write failed");
        reject(StatusCode::INTERNAL_SERVER_ERROR, "Unable to record source review")
    })?;

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/pdf"))
        .header(
            CONTENT_DISPOSITION,
            HeaderValue::from_static("inline; filename=source.pdf"),
        )
        .header(
            CACHE_CONTROL,
            HeaderValue::from_static("private, no-store, max-age=0"),
        )
        .header(
            "x-content-type-options",
            HeaderValue::from_static("nosniff"),
        )
        .body(Body::from(bytes))
        .map_err(|error| {
            error!(%error, %asset_id, "knowledge source response construction failed");
            reject(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to render source document",
            )
        })
}

fn object_key_for_school(reference: &str, school_id: Uuid) -> Option<&str> {
    let prefix = format!("storage://{KNOWLEDGE_SOURCE_BUCKET}/");
    let key = reference.strip_prefix(&prefix)?;
    let school_prefix = format!("{school_id}/");
    let filename = key.strip_prefix(&school_prefix)?;
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename == "."
        || filename == ".."
    {
        return None;
    }
    let stem = filename.strip_suffix(".pdf")?;
    Uuid::parse_str(stem).ok()?;
    Some(key)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn internal_source_error(error: sqlx::Error) -> SourceRejection {
    error!(%error, "knowledge source metadata decode failed");
    reject(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Unable to load source metadata",
    )
}

fn reject(status: StatusCode, message: &'static str) -> SourceRejection {
    (status, Json(json!({ "error": message })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_opaque_school_scoped_pdf_references() {
        let school_id = Uuid::nil();
        let valid = "storage://edutalent-knowledge-sources/00000000-0000-0000-0000-000000000000/11111111-1111-1111-1111-111111111111.pdf";
        assert_eq!(
            object_key_for_school(valid, school_id),
            Some("00000000-0000-0000-0000-000000000000/11111111-1111-1111-1111-111111111111.pdf")
        );
        assert!(object_key_for_school(
            "storage://edutalent-knowledge-sources/22222222-2222-2222-2222-222222222222/11111111-1111-1111-1111-111111111111.pdf",
            school_id
        )
        .is_none());
        assert!(object_key_for_school(
            "storage://edutalent-knowledge-sources/00000000-0000-0000-0000-000000000000/../secret.pdf",
            school_id
        )
        .is_none());
        assert!(object_key_for_school("https://example.invalid/source.pdf", school_id).is_none());
    }

    #[test]
    fn source_hash_is_stable() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
