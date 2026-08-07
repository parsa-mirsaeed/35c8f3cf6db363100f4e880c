// PR-03: protected database access is transaction-scoped through AuthorizedPool.
//! Persistence boundary for governed knowledge assets.

use crate::repositories::{BaseRepository, Repository, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgConnection, Row};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAssetStatus {
    Submitted,
    OcrPending,
    OcrReady,
    EmbeddingPending,
    Embedded,
    Published,
    Archived,
    Failed,
}

impl KnowledgeAssetStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::OcrPending => "ocr_pending",
            Self::OcrReady => "ocr_ready",
            Self::EmbeddingPending => "embedding_pending",
            Self::Embedded => "embedded",
            Self::Published => "published",
            Self::Archived => "archived",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> RepositoryResult<Self> {
        match value {
            "submitted" => Ok(Self::Submitted),
            "ocr_pending" => Ok(Self::OcrPending),
            "ocr_ready" => Ok(Self::OcrReady),
            "embedding_pending" => Ok(Self::EmbeddingPending),
            "embedded" => Ok(Self::Embedded),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            "failed" => Ok(Self::Failed),
            other => Err(RepositoryError::Validation(format!(
                "Unknown knowledge asset status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeAsset {
    pub id: Uuid,
    pub school_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub source_type: String,
    pub status: KnowledgeAssetStatus,
    pub language: String,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub template_type: Option<String>,
    pub tags: Value,
    pub created_by: Uuid,
    pub reviewed_by: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeAssetWithSelection {
    pub asset: KnowledgeAsset,
    pub enabled: bool,
    pub context_scope: String,
    pub context_key: String,
}

#[derive(Debug, Clone)]
pub struct CreateKnowledgeSubmission {
    pub school_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub source_type: String,
    pub language: String,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub template_type: Option<String>,
    pub tags: Value,
    pub created_by: Uuid,
    pub original_file_url: Option<String>,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size_bytes: Option<i64>,
    pub sha256: Option<String>,
    pub page_count: Option<i32>,
    pub is_scanned_pdf: bool,
}

#[derive(Debug, Clone)]
pub struct AssetForEmbedding {
    pub asset: KnowledgeAsset,
    pub clean_text: String,
}

#[derive(Debug, Clone)]
pub struct PersistedChunk {
    pub chunk_index: i32,
    pub text: String,
    pub token_count: i32,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub vector_id: String,
    pub metadata: Value,
}

#[derive(Clone)]
pub struct KnowledgeAssetRepository {
    base: BaseRepository,
}

impl KnowledgeAssetRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    pub async fn create_submission(
        &self,
        request: CreateKnowledgeSubmission,
    ) -> RepositoryResult<KnowledgeAsset> {
        if request.title.trim().is_empty() {
            return Err(RepositoryError::Validation("Title is required".into()));
        }
        if request.original_filename.trim().is_empty() {
            return Err(RepositoryError::Validation(
                "Original filename is required".into(),
            ));
        }

        let mut tx = self.base.pool().begin().await?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_assets (
                school_id, title, description, source_type, status, language,
                subject, grade, template_type, tags, created_by
            ) VALUES ($1, $2, $3, $4, 'submitted', $5, $6, $7, $8, $9, $10)
            RETURNING id, school_id, title, description, source_type,
                      status::text AS status, language, subject, grade,
                      template_type, tags, created_by, reviewed_by, published_at,
                      failure_reason, created_at, updated_at
            "#,
        )
        .bind(request.school_id)
        .bind(request.title.trim())
        .bind(request.description)
        .bind(request.source_type)
        .bind(request.language)
        .bind(request.subject)
        .bind(request.grade)
        .bind(request.template_type)
        .bind(request.tags)
        .bind(request.created_by)
        .fetch_one(&mut *tx)
        .await?;

        let asset = Self::row_to_asset(&row)?;

        sqlx::query(
            r#"
            INSERT INTO knowledge_source_files (
                asset_id, original_file_url, original_filename, mime_type,
                file_size_bytes, sha256, page_count, is_scanned_pdf
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(asset.id)
        .bind(request.original_file_url)
        .bind(request.original_filename)
        .bind(request.mime_type)
        .bind(request.file_size_bytes)
        .bind(request.sha256)
        .bind(request.page_count)
        .bind(request.is_scanned_pdf)
        .execute(&mut *tx)
        .await?;

        Self::append_audit_in_tx(
            &mut *tx,
            request.created_by,
            "SchoolManager",
            "knowledge_asset.submitted",
            asset.id,
            asset.school_id,
            serde_json::json!({"status": "submitted"}),
        )
        .await?;

        tx.commit().await?;
        Ok(asset)
    }

    pub async fn find_by_id(&self, asset_id: Uuid) -> RepositoryResult<KnowledgeAsset> {
        let row = sqlx::query(
            r#"
            SELECT id, school_id, title, description, source_type,
                   status::text AS status, language, subject, grade,
                   template_type, tags, created_by, reviewed_by, published_at,
                   failure_reason, created_at, updated_at
            FROM knowledge_assets WHERE id = $1
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "KnowledgeAsset".into(),
            id: asset_id.to_string(),
        })?;
        Self::row_to_asset(&row)
    }

    pub async fn list_for_admin(&self) -> RepositoryResult<Vec<KnowledgeAsset>> {
        let rows = sqlx::query(
            r#"
            SELECT id, school_id, title, description, source_type,
                   status::text AS status, language, subject, grade,
                   template_type, tags, created_by, reviewed_by, published_at,
                   failure_reason, created_at, updated_at
            FROM knowledge_assets
            ORDER BY created_at DESC
            LIMIT 500
            "#,
        )
        .fetch_all(&*self.base.pool())
        .await?;
        rows.iter().map(Self::row_to_asset).collect()
    }

    pub async fn list_for_school(&self, school_id: Uuid) -> RepositoryResult<Vec<KnowledgeAsset>> {
        let rows = sqlx::query(
            r#"
            SELECT id, school_id, title, description, source_type,
                   status::text AS status, language, subject, grade,
                   template_type, tags, created_by, reviewed_by, published_at,
                   failure_reason, created_at, updated_at
            FROM knowledge_assets
            WHERE school_id = $1
            ORDER BY created_at DESC
            LIMIT 500
            "#,
        )
        .bind(school_id)
        .fetch_all(&*self.base.pool())
        .await?;
        rows.iter().map(Self::row_to_asset).collect()
    }

    pub async fn list_available_for_teacher(
        &self,
        teacher_user_id: Uuid,
        context_scope: &str,
        context_key: &str,
    ) -> RepositoryResult<Vec<KnowledgeAssetWithSelection>> {
        let rows = sqlx::query(
            r#"
            SELECT ka.id, ka.school_id, ka.title, ka.description, ka.source_type,
                   ka.status::text AS status, ka.language, ka.subject, ka.grade,
                   ka.template_type, ka.tags, ka.created_by, ka.reviewed_by,
                   ka.published_at, ka.failure_reason, ka.created_at, ka.updated_at,
                   COALESCE(tas.enabled, FALSE) AS enabled
            FROM teachers t
            JOIN users u ON u.id = t.user_id
            JOIN knowledge_assets ka ON ka.school_id = u.school_id
            LEFT JOIN teacher_asset_selections tas
              ON tas.teacher_id = t.id
             AND tas.asset_id = ka.id
             AND tas.context_scope = $2
             AND tas.context_key = $3
            WHERE t.user_id = $1 AND ka.status = 'published'
            ORDER BY ka.subject NULLS LAST, ka.grade NULLS LAST, ka.title
            "#,
        )
        .bind(teacher_user_id)
        .bind(context_scope)
        .bind(context_key)
        .fetch_all(&*self.base.pool())
        .await?;

        rows.iter()
            .map(|row| {
                Ok(KnowledgeAssetWithSelection {
                    asset: Self::row_to_asset(row)?,
                    enabled: row.try_get("enabled")?,
                    context_scope: context_scope.to_string(),
                    context_key: context_key.to_string(),
                })
            })
            .collect()
    }

    pub async fn set_teacher_selection(
        &self,
        teacher_user_id: Uuid,
        asset_id: Uuid,
        enabled: bool,
        context_scope: &str,
        context_key: &str,
    ) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            INSERT INTO teacher_asset_selections (
                teacher_id, asset_id, enabled, context_scope, context_key
            )
            SELECT t.id, ka.id, $3, $4, $5
            FROM teachers t
            JOIN users u ON u.id = t.user_id
            JOIN knowledge_assets ka ON ka.id = $2
            WHERE t.user_id = $1
              AND ka.school_id = u.school_id
              AND ka.status = 'published'
            ON CONFLICT (teacher_id, asset_id, context_scope, context_key)
            DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = NOW()
            "#,
        )
        .bind(teacher_user_id)
        .bind(asset_id)
        .bind(enabled)
        .bind(context_scope)
        .bind(context_key)
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::Unauthorized);
        }
        Ok(())
    }

    pub async fn attach_verified_ocr(
        &self,
        asset_id: Uuid,
        raw_text: &str,
        clean_text: &str,
        provider: &str,
        verified_by: Uuid,
        text_sha256: &str,
    ) -> RepositoryResult<()> {
        let mut tx = self.base.pool().begin().await?;
        sqlx::query(
            r#"
            INSERT INTO knowledge_ocr_texts (
                asset_id, raw_text, clean_text, ocr_provider,
                ocr_verified_by, ocr_verified_at, text_sha256
            ) VALUES ($1, $2, $3, $4, $5, NOW(), $6)
            ON CONFLICT (asset_id) DO UPDATE SET
                raw_text = EXCLUDED.raw_text,
                clean_text = EXCLUDED.clean_text,
                ocr_provider = EXCLUDED.ocr_provider,
                ocr_verified_by = EXCLUDED.ocr_verified_by,
                ocr_verified_at = NOW(),
                text_sha256 = EXCLUDED.text_sha256,
                updated_at = NOW()
            "#,
        )
        .bind(asset_id)
        .bind(raw_text)
        .bind(clean_text)
        .bind(provider)
        .bind(verified_by)
        .bind(text_sha256)
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query(
            "UPDATE knowledge_assets SET status = 'ocr_ready', reviewed_by = $2, failure_reason = NULL WHERE id = $1 RETURNING school_id",
        )
        .bind(asset_id)
        .bind(verified_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "KnowledgeAsset".into(),
            id: asset_id.to_string(),
        })?;
        let school_id: Uuid = row.try_get("school_id")?;

        Self::append_audit_in_tx(
            &mut *tx,
            verified_by,
            "PlatformAdmin",
            "knowledge_asset.ocr_verified",
            asset_id,
            school_id,
            serde_json::json!({"ocr_provider": provider, "text_sha256": text_sha256}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_for_embedding(&self, asset_id: Uuid) -> RepositoryResult<AssetForEmbedding> {
        let row = sqlx::query(
            r#"
            SELECT ka.id, ka.school_id, ka.title, ka.description, ka.source_type,
                   ka.status::text AS status, ka.language, ka.subject, ka.grade,
                   ka.template_type, ka.tags, ka.created_by, ka.reviewed_by,
                   ka.published_at, ka.failure_reason, ka.created_at, ka.updated_at,
                   kot.clean_text
            FROM knowledge_assets ka
            JOIN knowledge_ocr_texts kot ON kot.asset_id = ka.id
            WHERE ka.id = $1 AND ka.status IN ('ocr_ready', 'failed', 'embedded', 'embedding_pending')
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| {
            RepositoryError::Validation("Asset must have verified OCR before embedding".into())
        })?;

        Ok(AssetForEmbedding {
            asset: Self::row_to_asset(&row)?,
            clean_text: row.try_get("clean_text")?,
        })
    }
    pub async fn complete_embedding(
        &self,
        asset_id: Uuid,
        job_id: Uuid,
        actor_id: Uuid,
        chunks: &[PersistedChunk],
    ) -> RepositoryResult<()> {
        let mut tx = self.base.pool().begin().await?;
        sqlx::query("DELETE FROM knowledge_chunks WHERE asset_id = $1")
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;

        for chunk in chunks {
            sqlx::query(
                r#"
                INSERT INTO knowledge_chunks (
                    asset_id, chunk_index, text, token_count, embedding_provider,
                    embedding_model, vector_id, metadata_json
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(asset_id)
            .bind(chunk.chunk_index)
            .bind(&chunk.text)
            .bind(chunk.token_count)
            .bind(&chunk.embedding_provider)
            .bind(&chunk.embedding_model)
            .bind(&chunk.vector_id)
            .bind(&chunk.metadata)
            .execute(&mut *tx)
            .await?;
        }

        let row = sqlx::query(
            "UPDATE knowledge_assets SET status = 'embedded', reviewed_by = $2 WHERE id = $1 RETURNING school_id",
        )
        .bind(asset_id)
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        let school_id: Uuid = row.try_get("school_id")?;
        sqlx::query(
            "UPDATE ingestion_jobs SET status = 'succeeded', finished_at = NOW() WHERE id = $1",
        )
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
        Self::append_audit_in_tx(
            &mut *tx,
            actor_id,
            "PlatformAdmin",
            "knowledge_asset.embedded",
            asset_id,
            school_id,
            serde_json::json!({"chunk_count": chunks.len()}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn publish(&self, asset_id: Uuid, actor_id: Uuid) -> RepositoryResult<()> {
        let mut tx = self.base.pool().begin().await?;
        let row = sqlx::query(
            r#"
            UPDATE knowledge_assets ka
            SET status = 'published', reviewed_by = $2, published_at = NOW()
            WHERE ka.id = $1
              AND ka.status = 'embedded'
              AND EXISTS (SELECT 1 FROM knowledge_chunks kc WHERE kc.asset_id = ka.id)
            RETURNING school_id
            "#,
        )
        .bind(asset_id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            RepositoryError::Validation(
                "Only embedded assets with persisted chunks can be published".into(),
            )
        })?;
        let school_id: Uuid = row.try_get("school_id")?;
        Self::append_audit_in_tx(
            &mut *tx,
            actor_id,
            "PlatformAdmin",
            "knowledge_asset.published",
            asset_id,
            school_id,
            serde_json::json!({}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn archive(&self, asset_id: Uuid, actor_id: Uuid) -> RepositoryResult<()> {
        let mut tx = self.base.pool().begin().await?;
        let row = sqlx::query(
            "UPDATE knowledge_assets SET status = 'archived', reviewed_by = $2, archived_at = NOW() WHERE id = $1 RETURNING school_id",
        )
        .bind(asset_id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "KnowledgeAsset".into(),
            id: asset_id.to_string(),
        })?;
        let school_id: Uuid = row.try_get("school_id")?;
        sqlx::query("UPDATE teacher_asset_selections SET enabled = FALSE WHERE asset_id = $1")
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;
        Self::append_audit_in_tx(
            &mut *tx,
            actor_id,
            "PlatformAdmin",
            "knowledge_asset.archived",
            asset_id,
            school_id,
            serde_json::json!({}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn authorized_enabled_asset_ids(
        &self,
        teacher_user_id: Uuid,
        requested_asset_ids: &[Uuid],
        context_scope: &str,
        context_key: &str,
    ) -> RepositoryResult<Vec<Uuid>> {
        let rows = sqlx::query(
            r#"
            SELECT ka.id
            FROM teachers t
            JOIN users u ON u.id = t.user_id
            JOIN teacher_asset_selections tas ON tas.teacher_id = t.id
            JOIN knowledge_assets ka ON ka.id = tas.asset_id
            WHERE t.user_id = $1
              AND ka.school_id = u.school_id
              AND ka.status = 'published'
              AND tas.enabled = TRUE
              AND tas.context_scope = $2
              AND tas.context_key = $3
              AND ka.id = ANY($4)
            "#,
        )
        .bind(teacher_user_id)
        .bind(context_scope)
        .bind(context_key)
        .bind(requested_asset_ids)
        .fetch_all(&*self.base.pool())
        .await?;
        rows.iter()
            .map(|row| row.try_get("id").map_err(Into::into))
            .collect()
    }

    pub async fn append_query_audit(
        &self,
        actor_id: Uuid,
        school_id: Uuid,
        asset_ids: &[Uuid],
        result_count: usize,
    ) -> RepositoryResult<()> {
        sqlx::query(
            r#"
            INSERT INTO knowledge_audit_logs (
                actor_id, actor_role, action, target_type, target_id, school_id, details_json
            ) VALUES ($1, 'Teacher', 'knowledge_asset.queried', 'knowledge_asset_set', $2, $3, $4)
            "#,
        )
        .bind(actor_id)
        .bind(asset_ids.first().copied().unwrap_or(Uuid::nil()))
        .bind(school_id)
        .bind(serde_json::json!({"asset_ids": asset_ids, "result_count": result_count}))
        .execute(&*self.base.pool())
        .await?;
        Ok(())
    }

    async fn append_audit_in_tx(
        tx: &mut PgConnection,
        actor_id: Uuid,
        actor_role: &str,
        action: &str,
        target_id: Uuid,
        school_id: Uuid,
        details: Value,
    ) -> RepositoryResult<()> {
        sqlx::query(
            r#"
            INSERT INTO knowledge_audit_logs (
                actor_id, actor_role, action, target_type, target_id, school_id, details_json
            ) VALUES ($1, $2, $3, 'knowledge_asset', $4, $5, $6)
            "#,
        )
        .bind(actor_id)
        .bind(actor_role)
        .bind(action)
        .bind(target_id)
        .bind(school_id)
        .bind(details)
        .execute(&mut *tx)
        .await?;
        Ok(())
    }

    fn row_to_asset(row: &sqlx::postgres::PgRow) -> RepositoryResult<KnowledgeAsset> {
        let status: String = row.try_get("status")?;
        Ok(KnowledgeAsset {
            id: row.try_get("id")?,
            school_id: row.try_get("school_id")?,
            title: row.try_get("title")?,
            description: row.try_get("description")?,
            source_type: row.try_get("source_type")?,
            status: KnowledgeAssetStatus::parse(&status)?,
            language: row.try_get("language")?,
            subject: row.try_get("subject")?,
            grade: row.try_get("grade")?,
            template_type: row.try_get("template_type")?,
            tags: row.try_get("tags")?,
            created_by: row.try_get("created_by")?,
            reviewed_by: row.try_get("reviewed_by")?,
            published_at: row.try_get("published_at")?,
            failure_reason: row.try_get("failure_reason")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl Repository for KnowledgeAssetRepository {
    fn pool(&self) -> Arc<AuthorizedPool> {
        self.base.pool()
    }
}
