use crate::server_functions::knowledge_functions::KnowledgeAssetDto;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::repositories::KnowledgeAssetRepository;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "server")]
use uuid::Uuid;

const KNOWLEDGE_SOURCE_BUCKET: &str = "edutalent-knowledge-sources";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminKnowledgeReviewAssetDto {
    pub asset: KnowledgeAssetDto,
    pub source_review_available: bool,
    pub original_filename: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub has_verified_ocr: bool,
}

#[cfg(feature = "server")]
#[derive(Debug)]
struct SourceReviewMetadata {
    original_file_url: Option<String>,
    original_filename: String,
    mime_type: String,
    file_size_bytes: Option<i64>,
}

#[server(endpoint = "admin/knowledge-assets/review-list")]
pub async fn list_admin_knowledge_assets_for_review(
) -> Result<Vec<AdminKnowledgeReviewAssetDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "PlatformAdmin" {
            return Err(ServerFnError::new("Forbidden: insufficient role"));
        }

        let assets = KnowledgeAssetRepository::new(pool.clone())
            .list_for_admin()
            .await
            .map_err(|error| {
                tracing::error!(%error, "platform knowledge review list failed");
                ServerFnError::new("Unable to load governed knowledge assets")
            })?;
        if assets.is_empty() {
            return Ok(Vec::new());
        }

        let asset_ids = assets.iter().map(|asset| asset.id).collect::<Vec<_>>();
        let source_rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (asset_id)
                asset_id,
                original_file_url,
                original_filename,
                mime_type,
                file_size_bytes
            FROM knowledge_source_files
            WHERE asset_id = ANY($1)
            ORDER BY asset_id, created_at ASC
            "#,
        )
        .bind(&asset_ids)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "platform knowledge source metadata list failed");
            ServerFnError::new("Unable to load governed source metadata")
        })?;

        let ocr_rows = sqlx::query_scalar::<_, Uuid>(
            "SELECT asset_id FROM knowledge_ocr_texts WHERE asset_id = ANY($1)",
        )
        .bind(&asset_ids)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "platform knowledge OCR readiness list failed");
            ServerFnError::new("Unable to load governed OCR readiness")
        })?;
        let ocr_asset_ids = ocr_rows.into_iter().collect::<HashSet<_>>();

        let mut source_by_asset = HashMap::<Uuid, SourceReviewMetadata>::new();
        for row in source_rows {
            let asset_id: Uuid = row.try_get("asset_id").map_err(|error| {
                tracing::error!(%error, "platform knowledge source metadata decode failed");
                ServerFnError::new("Unable to load governed source metadata")
            })?;
            source_by_asset.insert(
                asset_id,
                SourceReviewMetadata {
                    original_file_url: row.try_get("original_file_url").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source URL decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                    original_filename: row.try_get("original_filename").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source filename decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                    mime_type: row.try_get("mime_type").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source MIME decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                    file_size_bytes: row.try_get("file_size_bytes").map_err(|error| {
                        tracing::error!(%error, "platform knowledge source size decode failed");
                        ServerFnError::new("Unable to load governed source metadata")
                    })?,
                },
            );
        }

        Ok(assets
            .into_iter()
            .map(|asset| {
                let source = source_by_asset.remove(&asset.id);
                let source_review_available = source.as_ref().is_some_and(|source| {
                    source.mime_type == "application/pdf"
                        && source
                            .original_file_url
                            .as_deref()
                            .is_some_and(|reference| {
                                let expected_prefix = format!(
                                    "storage://{KNOWLEDGE_SOURCE_BUCKET}/{}/",
                                    asset.school_id
                                );
                                reference.starts_with(&expected_prefix)
                            })
                });
                let has_verified_ocr = ocr_asset_ids.contains(&asset.id);
                AdminKnowledgeReviewAssetDto {
                    asset: asset.into(),
                    source_review_available,
                    original_filename: source
                        .as_ref()
                        .map(|source| source.original_filename.clone()),
                    file_size_bytes: source.and_then(|source| source.file_size_bytes),
                    has_verified_ocr,
                }
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_bucket_name_is_fixed() {
        assert_eq!(KNOWLEDGE_SOURCE_BUCKET, "edutalent-knowledge-sources");
    }
}
