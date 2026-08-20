use crate::app_state::AppState;
use crate::domain::UserInfo;
use crate::repositories::{CreateKnowledgeSubmission, KnowledgeAssetRepository};
use crate::rls_context::AuthorizedPool;
use axum::{
    extract::{multipart::Field, Multipart},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tracing::{error, warn};
use uuid::Uuid;

pub const MAX_KNOWLEDGE_PDF_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_KNOWLEDGE_UPLOAD_BODY_BYTES: usize = MAX_KNOWLEDGE_PDF_BYTES + 512 * 1024;

const KNOWLEDGE_SOURCE_BUCKET: &str = "edutalent-knowledge-sources";
const MAX_TITLE_BYTES: usize = 255;
const MAX_DESCRIPTION_BYTES: usize = 8_000;
const MAX_SUBJECT_BYTES: usize = 255;
const MAX_GRADE_BYTES: usize = 64;
const MAX_FILENAME_BYTES: usize = 255;

type UploadRejection = (StatusCode, Json<Value>);

#[derive(Default)]
struct ParsedKnowledgeUpload {
    title: Option<String>,
    description: Option<String>,
    subject: Option<String>,
    grade: Option<String>,
    original_filename: Option<String>,
    pdf_bytes: Option<Vec<u8>>,
}

pub async fn knowledge_upload_handler(
    Extension(state): Extension<AppState>,
    Extension(user): Extension<UserInfo>,
    Extension(pool): Extension<Arc<AuthorizedPool>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), UploadRejection> {
    if user.role != "SchoolManager" {
        return Err(reject(StatusCode::FORBIDDEN, "Insufficient role"));
    }

    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| reject(StatusCode::UNAUTHORIZED, "Invalid active session"))?;
    let school_id =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT school_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&*pool)
            .await
            .map_err(|error| {
                error!(%error, user_id = %user_id, "knowledge upload school lookup failed");
                reject(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Unable to resolve school scope",
                )
            })?
            .flatten()
            .ok_or_else(|| reject(StatusCode::FORBIDDEN, "School manager has no school scope"))?;

    let upload = parse_upload(&mut multipart).await?;
    let title = required_text(upload.title, "Title is required")?;
    let original_filename = required_text(upload.original_filename, "A PDF file is required")?;
    let pdf_bytes = upload
        .pdf_bytes
        .ok_or_else(|| reject(StatusCode::BAD_REQUEST, "A PDF file is required"))?;

    validate_pdf(&original_filename, &pdf_bytes)?;

    let file_size_bytes = i64::try_from(pdf_bytes.len())
        .map_err(|_| reject(StatusCode::PAYLOAD_TOO_LARGE, "PDF is too large"))?;
    let sha256 = sha256_hex(&pdf_bytes);
    let object_key = format!("{school_id}/{}.pdf", Uuid::new_v4());
    let storage_reference = format!("storage://{KNOWLEDGE_SOURCE_BUCKET}/{object_key}");

    ensure_private_bucket(&state).await?;
    upload_storage_object(&state, &object_key, pdf_bytes).await?;

    let repository = KnowledgeAssetRepository::new(pool);
    let create_result = repository
        .create_submission(CreateKnowledgeSubmission {
            school_id,
            title,
            description: normalize_optional(upload.description),
            source_type: "pdf".to_string(),
            language: "fa".to_string(),
            subject: normalize_optional(upload.subject),
            grade: normalize_optional(upload.grade),
            template_type: None,
            tags: json!({}),
            created_by: user_id,
            original_file_url: Some(storage_reference),
            original_filename,
            mime_type: "application/pdf".to_string(),
            file_size_bytes: Some(file_size_bytes),
            sha256: Some(sha256),
            page_count: None,
            is_scanned_pdf: false,
        })
        .await;

    match create_result {
        Ok(asset) => Ok((
            StatusCode::CREATED,
            Json(json!({
                "id": asset.id,
                "status": asset.status.as_str(),
            })),
        )),
        Err(error) => {
            error!(%error, school_id = %school_id, object_key = %object_key, "knowledge upload database persistence failed");
            if let Err(cleanup_error) = delete_storage_object(&state, &object_key).await {
                error!(
                    %cleanup_error,
                    school_id = %school_id,
                    object_key = %object_key,
                    "knowledge upload compensation failed; object may be orphaned"
                );
            }
            Err(reject(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to register uploaded PDF",
            ))
        }
    }
}

async fn parse_upload(multipart: &mut Multipart) -> Result<ParsedKnowledgeUpload, UploadRejection> {
    let mut upload = ParsedKnowledgeUpload::default();

    while let Some(mut field) = multipart.next_field().await.map_err(|error| {
        warn!(%error, "invalid knowledge upload multipart body");
        reject(StatusCode::BAD_REQUEST, "Invalid upload form")
    })? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "title" => {
                set_once(
                    &mut upload.title,
                    read_text_field(&mut field, MAX_TITLE_BYTES).await?,
                    "Duplicate title field",
                )?;
            }
            "description" => {
                set_once(
                    &mut upload.description,
                    read_text_field(&mut field, MAX_DESCRIPTION_BYTES).await?,
                    "Duplicate description field",
                )?;
            }
            "subject" => {
                set_once(
                    &mut upload.subject,
                    read_text_field(&mut field, MAX_SUBJECT_BYTES).await?,
                    "Duplicate subject field",
                )?;
            }
            "grade" => {
                set_once(
                    &mut upload.grade,
                    read_text_field(&mut field, MAX_GRADE_BYTES).await?,
                    "Duplicate grade field",
                )?;
            }
            "file" => {
                if upload.pdf_bytes.is_some() || upload.original_filename.is_some() {
                    return Err(reject(
                        StatusCode::BAD_REQUEST,
                        "Only one PDF may be uploaded",
                    ));
                }
                let filename = field.file_name().unwrap_or_default().trim().to_string();
                if filename.is_empty() || filename.len() > MAX_FILENAME_BYTES {
                    return Err(reject(StatusCode::BAD_REQUEST, "PDF filename is invalid"));
                }
                let bytes = read_limited_field(&mut field, MAX_KNOWLEDGE_PDF_BYTES).await?;
                upload.original_filename = Some(filename);
                upload.pdf_bytes = Some(bytes);
            }
            "" => {
                return Err(reject(
                    StatusCode::BAD_REQUEST,
                    "Upload field is missing a name",
                ))
            }
            _ => {
                return Err(reject(
                    StatusCode::BAD_REQUEST,
                    "Upload form contains an unsupported field",
                ))
            }
        }
    }

    Ok(upload)
}

async fn read_text_field(field: &mut Field<'_>, limit: usize) -> Result<String, UploadRejection> {
    let bytes = read_limited_field(field, limit).await?;
    String::from_utf8(bytes)
        .map(|value| value.trim().to_string())
        .map_err(|_| {
            reject(
                StatusCode::BAD_REQUEST,
                "Upload metadata must be valid UTF-8",
            )
        })
}

async fn read_limited_field(
    field: &mut Field<'_>,
    limit: usize,
) -> Result<Vec<u8>, UploadRejection> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(|error| {
        warn!(%error, "unable to read knowledge upload field");
        reject(StatusCode::BAD_REQUEST, "Invalid upload body")
    })? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(reject(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Upload field is too large",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn set_once(
    slot: &mut Option<String>,
    value: String,
    duplicate_message: &'static str,
) -> Result<(), UploadRejection> {
    if slot.is_some() {
        return Err(reject(StatusCode::BAD_REQUEST, duplicate_message));
    }
    *slot = Some(value);
    Ok(())
}

fn required_text(value: Option<String>, message: &'static str) -> Result<String, UploadRejection> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| reject(StatusCode::BAD_REQUEST, message))
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn validate_pdf(filename: &str, bytes: &[u8]) -> Result<(), UploadRejection> {
    if !filename.to_ascii_lowercase().ends_with(".pdf") {
        return Err(reject(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Only PDF files are accepted",
        ));
    }
    if bytes.is_empty() || bytes.len() > MAX_KNOWLEDGE_PDF_BYTES {
        return Err(reject(
            StatusCode::PAYLOAD_TOO_LARGE,
            "PDF is empty or too large",
        ));
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(reject(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "File content is not a PDF",
        ));
    }
    let tail_start = bytes.len().saturating_sub(1024);
    if !bytes[tail_start..]
        .windows(5)
        .any(|window| window == b"%%EOF")
    {
        return Err(reject(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "PDF is incomplete",
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

async fn ensure_private_bucket(state: &AppState) -> Result<(), UploadRejection> {
    let bucket_url = format!(
        "{}/storage/v1/bucket/{KNOWLEDGE_SOURCE_BUCKET}",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = storage_auth(state.services.http_client.get(&bucket_url), state)
        .send()
        .await
        .map_err(|error| {
            error!(%error, "knowledge storage bucket lookup failed");
            reject(StatusCode::BAD_GATEWAY, "Knowledge storage is unavailable")
        })?;

    if response.status().is_success() {
        let body = response.json::<Value>().await.map_err(|error| {
            error!(%error, "knowledge storage bucket response was invalid");
            reject(StatusCode::BAD_GATEWAY, "Knowledge storage is unavailable")
        })?;
        if body.get("public").and_then(Value::as_bool) != Some(false) {
            error!(
                bucket = KNOWLEDGE_SOURCE_BUCKET,
                "knowledge source bucket is not private"
            );
            return Err(reject(
                StatusCode::SERVICE_UNAVAILABLE,
                "Knowledge storage is not safely configured",
            ));
        }
        return Ok(());
    }

    if response.status().as_u16() != 404 {
        error!(status = %response.status(), "knowledge storage bucket lookup failed");
        return Err(reject(
            StatusCode::BAD_GATEWAY,
            "Knowledge storage is unavailable",
        ));
    }

    let create_url = format!(
        "{}/storage/v1/bucket",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = storage_auth(state.services.http_client.post(&create_url), state)
        .json(&json!({
            "id": KNOWLEDGE_SOURCE_BUCKET,
            "name": KNOWLEDGE_SOURCE_BUCKET,
            "public": false,
            "fileSizeLimit": MAX_KNOWLEDGE_PDF_BYTES,
            "allowedMimeTypes": ["application/pdf"]
        }))
        .send()
        .await
        .map_err(|error| {
            error!(%error, "knowledge storage bucket creation failed");
            reject(StatusCode::BAD_GATEWAY, "Knowledge storage is unavailable")
        })?;

    if response.status().is_success() {
        return Ok(());
    }

    if response.status().as_u16() == 409 {
        return verify_bucket_private(state).await;
    }

    error!(status = %response.status(), "knowledge storage bucket creation failed");
    Err(reject(
        StatusCode::BAD_GATEWAY,
        "Knowledge storage is unavailable",
    ))
}

async fn verify_bucket_private(state: &AppState) -> Result<(), UploadRejection> {
    let url = format!(
        "{}/storage/v1/bucket/{KNOWLEDGE_SOURCE_BUCKET}",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = storage_auth(state.services.http_client.get(url), state)
        .send()
        .await
        .map_err(|error| {
            error!(%error, "knowledge storage bucket verification failed");
            reject(StatusCode::BAD_GATEWAY, "Knowledge storage is unavailable")
        })?;
    if !response.status().is_success() {
        return Err(reject(
            StatusCode::BAD_GATEWAY,
            "Knowledge storage is unavailable",
        ));
    }
    let body = response.json::<Value>().await.map_err(|error| {
        error!(%error, "knowledge storage bucket verification response was invalid");
        reject(StatusCode::BAD_GATEWAY, "Knowledge storage is unavailable")
    })?;
    if body.get("public").and_then(Value::as_bool) == Some(false) {
        Ok(())
    } else {
        Err(reject(
            StatusCode::SERVICE_UNAVAILABLE,
            "Knowledge storage is not safely configured",
        ))
    }
}

async fn upload_storage_object(
    state: &AppState,
    object_key: &str,
    bytes: Vec<u8>,
) -> Result<(), UploadRejection> {
    let url = format!(
        "{}/storage/v1/object/{KNOWLEDGE_SOURCE_BUCKET}/{object_key}",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = storage_auth(state.services.http_client.post(url), state)
        .header("Content-Type", "application/pdf")
        .header("x-upsert", "false")
        .timeout(Duration::from_secs(60))
        .body(bytes)
        .send()
        .await
        .map_err(|error| {
            error!(%error, object_key = %object_key, "knowledge source object upload failed");
            reject(StatusCode::BAD_GATEWAY, "Unable to store uploaded PDF")
        })?;

    if response.status().is_success() {
        Ok(())
    } else {
        error!(status = %response.status(), object_key = %object_key, "knowledge source object upload was rejected");
        Err(reject(
            StatusCode::BAD_GATEWAY,
            "Unable to store uploaded PDF",
        ))
    }
}

async fn delete_storage_object(state: &AppState, object_key: &str) -> Result<(), String> {
    let url = format!(
        "{}/storage/v1/object/{KNOWLEDGE_SOURCE_BUCKET}",
        state.supabase_config.url.trim_end_matches('/')
    );
    let response = storage_auth(state.services.http_client.delete(url), state)
        .json(&json!({ "prefixes": [object_key] }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("storage cleanup returned {}", response.status()))
    }
}

fn storage_auth(builder: reqwest::RequestBuilder, state: &AppState) -> reqwest::RequestBuilder {
    builder
        .bearer_auth(&state.supabase_config.secret_key)
        .header("apikey", &state.supabase_config.secret_key)
}

fn reject(status: StatusCode, message: &'static str) -> UploadRejection {
    (status, Json(json!({ "error": message })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_pdf() -> Vec<u8> {
        b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n".to_vec()
    }

    #[test]
    fn validates_pdf_extension_signature_and_eof() {
        assert!(validate_pdf("guide.pdf", &minimal_pdf()).is_ok());
        assert!(validate_pdf("guide.txt", &minimal_pdf()).is_err());
        assert!(validate_pdf("guide.pdf", b"not-a-pdf").is_err());
        assert!(validate_pdf("guide.pdf", b"%PDF-1.4\nmissing eof").is_err());
    }

    #[test]
    fn storage_reference_components_never_use_client_filename() {
        let school_id = Uuid::nil();
        let object_key = format!("{school_id}/{}.pdf", Uuid::nil());
        assert_eq!(
            object_key,
            "00000000-0000-0000-0000-000000000000/00000000-0000-0000-0000-000000000000.pdf"
        );
        assert!(!object_key.contains("../"));
    }

    #[test]
    fn sha256_is_stable_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
