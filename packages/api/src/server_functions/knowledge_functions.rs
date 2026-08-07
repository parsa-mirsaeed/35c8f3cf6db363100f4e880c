// PR-03: protected database access is transaction-scoped through AuthorizedPool.
//! Role-scoped server functions for the governed knowledge workflow.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use crate::repositories::{
    CreateKnowledgeSubmission, KnowledgeAsset, KnowledgeAssetRepository,
    KnowledgeAssetWithSelection, KnowledgeIngestionJobRepository,
};
#[cfg(feature = "server")]
use crate::rls_context::AuthorizedPool;
#[cfg(feature = "server")]
use crate::services::{KnowledgeAssetService, KnowledgeSearchResult};
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use uuid::Uuid;

const MAX_CONTEXT_KEY_BYTES: usize = 255;
const MAX_SEARCH_QUERY_BYTES: usize = 8_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeAssetDto {
    pub id: String,
    pub school_id: String,
    pub title: String,
    pub description: Option<String>,
    pub source_type: String,
    pub status: String,
    pub language: String,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub template_type: Option<String>,
    pub tags: Value,
    pub created_by: String,
    pub reviewed_by: Option<String>,
    pub published_at: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeAssetSelectionDto {
    pub asset: KnowledgeAssetDto,
    pub enabled: bool,
    pub context_scope: String,
    pub context_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeSearchResultDto {
    pub asset_id: String,
    pub asset_title: String,
    pub chunk_index: usize,
    pub chunk_text: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerKnowledgeSubmissionRequest {
    pub title: String,
    pub description: Option<String>,
    pub source_type: String,
    pub language: String,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub template_type: Option<String>,
    pub tags: Value,
    /// URL in the controlled source-document store or manager-only channel.
    pub original_file_url: Option<String>,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size_bytes: Option<i64>,
    pub sha256: Option<String>,
    pub page_count: Option<i32>,
    pub is_scanned_pdf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachOcrTextRequest {
    pub asset_id: String,
    pub raw_text: String,
    pub ocr_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleKnowledgeAssetRequest {
    pub asset_id: String,
    pub enabled: bool,
    pub context_scope: String,
    pub context_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSearchRequest {
    pub query: String,
    pub asset_ids: Vec<String>,
    pub context_scope: String,
    pub context_key: String,
    pub top_k: usize,
}

#[cfg(feature = "server")]
impl From<KnowledgeAsset> for KnowledgeAssetDto {
    fn from(asset: KnowledgeAsset) -> Self {
        Self {
            id: asset.id.to_string(),
            school_id: asset.school_id.to_string(),
            title: asset.title,
            description: asset.description,
            source_type: asset.source_type,
            status: asset.status.as_str().to_string(),
            language: asset.language,
            subject: asset.subject,
            grade: asset.grade,
            template_type: asset.template_type,
            tags: asset.tags,
            created_by: asset.created_by.to_string(),
            reviewed_by: asset.reviewed_by.map(|id| id.to_string()),
            published_at: asset.published_at.map(|value| value.to_rfc3339()),
            failure_reason: asset.failure_reason,
            created_at: asset.created_at.to_rfc3339(),
            updated_at: asset.updated_at.to_rfc3339(),
        }
    }
}

#[cfg(feature = "server")]
impl From<KnowledgeAssetWithSelection> for KnowledgeAssetSelectionDto {
    fn from(selection: KnowledgeAssetWithSelection) -> Self {
        Self {
            asset: selection.asset.into(),
            enabled: selection.enabled,
            context_scope: selection.context_scope,
            context_key: selection.context_key,
        }
    }
}

#[cfg(feature = "server")]
impl From<KnowledgeSearchResult> for KnowledgeSearchResultDto {
    fn from(result: KnowledgeSearchResult) -> Self {
        Self {
            asset_id: result.asset_id,
            asset_title: result.asset_title,
            chunk_index: result.chunk_index,
            chunk_text: result.chunk_text,
            score: result.score,
        }
    }
}

#[cfg(feature = "server")]
struct AuthorizedActor {
    user_id: Uuid,
    school_id: Option<Uuid>,
    pool: Arc<AuthorizedPool>,
}

#[cfg(feature = "server")]
async fn authorize(allowed_roles: &[&str]) -> Result<AuthorizedActor, ServerFnError> {
    let (user, pool) = crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;

    if !allowed_roles
        .iter()
        .any(|allowed_role| *allowed_role == user.role.as_str())
    {
        return Err(ServerFnError::new("Forbidden: insufficient role"));
    }

    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerFnError::new("Invalid authenticated user ID"))?;
    let school_id =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT school_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&*pool)
            .await
            .map_err(|error| ServerFnError::new(format!("Failed to resolve school: {error}")))?
            .flatten();

    Ok(AuthorizedActor {
        user_id,
        school_id,
        pool,
    })
}

#[server(endpoint = "manager/knowledge-submissions")]
pub async fn create_manager_knowledge_submission(
    request: ManagerKnowledgeSubmissionRequest,
) -> Result<KnowledgeAssetDto, ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_submission(&request)?;
        let actor = authorize(&["SchoolManager"]).await?;
        let school_id = actor
            .school_id
            .ok_or_else(|| ServerFnError::new("School manager has no school scope"))?;
        let repository = KnowledgeAssetRepository::new(actor.pool);
        let asset = repository
            .create_submission(CreateKnowledgeSubmission {
                school_id,
                title: request.title,
                description: request.description,
                source_type: request.source_type,
                language: request.language,
                subject: request.subject,
                grade: request.grade,
                template_type: request.template_type,
                tags: request.tags,
                created_by: actor.user_id,
                original_file_url: request.original_file_url,
                original_filename: request.original_filename,
                mime_type: request.mime_type,
                file_size_bytes: request.file_size_bytes,
                sha256: request.sha256,
                page_count: request.page_count,
                is_scanned_pdf: request.is_scanned_pdf,
            })
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(asset.into())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

#[server(endpoint = "manager/knowledge-submissions/list")]
pub async fn list_manager_knowledge_submissions() -> Result<Vec<KnowledgeAssetDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let actor = authorize(&["SchoolManager"]).await?;
        let school_id = actor
            .school_id
            .ok_or_else(|| ServerFnError::new("School manager has no school scope"))?;
        let assets = KnowledgeAssetRepository::new(actor.pool)
            .list_for_school(school_id)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(assets.into_iter().map(Into::into).collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "admin/knowledge-assets")]
pub async fn list_admin_knowledge_assets() -> Result<Vec<KnowledgeAssetDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let actor = authorize(&["PlatformAdmin"]).await?;
        let assets = KnowledgeAssetRepository::new(actor.pool)
            .list_for_admin()
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(assets.into_iter().map(Into::into).collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "admin/knowledge-assets/ocr-text")]
pub async fn attach_admin_ocr_text(request: AttachOcrTextRequest) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if request.raw_text.trim().is_empty() || request.ocr_provider.trim().is_empty() {
            return Err(ServerFnError::new(
                "Verified OCR text and provider are required",
            ));
        }
        let actor = authorize(&["PlatformAdmin"]).await?;
        let asset_id = parse_asset_id(&request.asset_id)?;
        KnowledgeAssetService::new(actor.pool)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .attach_verified_ocr(
                asset_id,
                &request.raw_text,
                &request.ocr_provider,
                actor.user_id,
            )
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(true)
    }
    #[cfg(not(feature = "server"))]
    Ok(false)
}

#[server(endpoint = "admin/knowledge-assets/embed")]
pub async fn embed_admin_knowledge_asset(asset_id: String) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let actor = authorize(&["PlatformAdmin"]).await?;
        let asset_id = parse_asset_id(&asset_id)?;
        let job_id = KnowledgeIngestionJobRepository::new(actor.pool)
            .enqueue_embedding(asset_id, actor.user_id)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(job_id.to_string())
    }
    #[cfg(not(feature = "server"))]
    Ok(String::new())
}

#[server(endpoint = "admin/knowledge-assets/publish")]
pub async fn publish_admin_knowledge_asset(asset_id: String) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let actor = authorize(&["PlatformAdmin"]).await?;
        let asset_id = parse_asset_id(&asset_id)?;
        KnowledgeAssetService::new(actor.pool)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .publish_asset(asset_id, actor.user_id)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(true)
    }
    #[cfg(not(feature = "server"))]
    Ok(false)
}

#[server(endpoint = "admin/knowledge-assets/archive")]
pub async fn archive_admin_knowledge_asset(asset_id: String) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let actor = authorize(&["PlatformAdmin"]).await?;
        let asset_id = parse_asset_id(&asset_id)?;
        KnowledgeAssetService::new(actor.pool)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .archive_asset(asset_id, actor.user_id)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(true)
    }
    #[cfg(not(feature = "server"))]
    Ok(false)
}

#[server(endpoint = "teacher/knowledge-assets/available")]
pub async fn list_teacher_available_knowledge_assets(
    context_scope: String,
    context_key: String,
) -> Result<Vec<KnowledgeAssetSelectionDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_context(&context_scope, &context_key)?;
        let actor = authorize(&["Teacher"]).await?;
        let assets = KnowledgeAssetRepository::new(actor.pool)
            .list_available_for_teacher(actor.user_id, &context_scope, &context_key)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(assets.into_iter().map(Into::into).collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "teacher/knowledge-assets/toggle")]
pub async fn toggle_teacher_knowledge_asset(
    request: ToggleKnowledgeAssetRequest,
) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_context(&request.context_scope, &request.context_key)?;
        let actor = authorize(&["Teacher"]).await?;
        let asset_id = parse_asset_id(&request.asset_id)?;
        KnowledgeAssetRepository::new(actor.pool)
            .set_teacher_selection(
                actor.user_id,
                asset_id,
                request.enabled,
                &request.context_scope,
                &request.context_key,
            )
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(true)
    }
    #[cfg(not(feature = "server"))]
    Ok(false)
}

#[server(endpoint = "teacher/knowledge-assets/search")]
pub async fn search_teacher_knowledge_assets(
    request: KnowledgeSearchRequest,
) -> Result<Vec<KnowledgeSearchResultDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_context(&request.context_scope, &request.context_key)?;
        let query = request.query.trim();
        if query.is_empty() || query.len() > MAX_SEARCH_QUERY_BYTES {
            return Err(ServerFnError::new("Search query is invalid"));
        }
        let actor = authorize(&["Teacher"]).await?;
        let asset_ids = request
            .asset_ids
            .iter()
            .map(|value| parse_asset_id(value))
            .collect::<Result<Vec<_>, _>>()?;
        let results = KnowledgeAssetService::new(actor.pool)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .search_for_teacher(
                actor.user_id,
                query,
                &asset_ids,
                &request.context_scope,
                &request.context_key,
                request.top_k,
            )
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(results.into_iter().map(Into::into).collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[cfg(feature = "server")]
fn parse_asset_id(value: &str) -> Result<Uuid, ServerFnError> {
    Uuid::parse_str(value).map_err(|_| ServerFnError::new("Invalid asset ID"))
}

fn validate_context(scope: &str, key: &str) -> Result<(), ServerFnError> {
    if !matches!(
        scope,
        "global" | "workflow" | "class" | "generation_session"
    ) {
        return Err(ServerFnError::new("Invalid knowledge context scope"));
    }
    if key.len() > MAX_CONTEXT_KEY_BYTES {
        return Err(ServerFnError::new("Knowledge context key is too long"));
    }
    if scope != "global" && key.trim().is_empty() {
        return Err(ServerFnError::new(
            "A context key is required for non-global selections",
        ));
    }
    Ok(())
}

fn validate_submission(request: &ManagerKnowledgeSubmissionRequest) -> Result<(), ServerFnError> {
    if request.title.trim().is_empty() || request.title.len() > 255 {
        return Err(ServerFnError::new("Knowledge asset title is invalid"));
    }
    if request.original_filename.trim().is_empty() || request.mime_type.trim().is_empty() {
        return Err(ServerFnError::new(
            "Original filename and MIME type are required",
        ));
    }
    if request.file_size_bytes.is_some_and(|size| size < 0)
        || request.page_count.is_some_and(|pages| pages < 0)
    {
        return Err(ServerFnError::new("File metadata cannot be negative"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_context_scopes() {
        for scope in ["global", "workflow", "class", "generation_session"] {
            let key = if scope == "global" { "" } else { "context-id" };
            assert!(validate_context(scope, key).is_ok());
        }
    }

    #[test]
    fn rejects_unknown_or_unkeyed_contexts() {
        assert!(validate_context("tenant", "key").is_err());
        assert!(validate_context("class", "").is_err());
    }
}
