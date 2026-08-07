use crate::repositories::{BaseRepository, Repository, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct KnowledgeAuditLog {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_role: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub school_id: Option<Uuid>,
    pub details: Value,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct KnowledgeAuditRepository {
    base: BaseRepository,
}

impl KnowledgeAuditRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    pub async fn list_recent(&self, limit: i64) -> RepositoryResult<Vec<KnowledgeAuditLog>> {
        let rows = sqlx::query(
            r#"
            SELECT id, actor_id, actor_role, action, target_type, target_id,
                   school_id, details_json, request_id, created_at
            FROM knowledge_audit_logs
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(&*self.base.pool())
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(KnowledgeAuditLog {
                    id: row.try_get("id")?,
                    actor_id: row.try_get("actor_id")?,
                    actor_role: row.try_get("actor_role")?,
                    action: row.try_get("action")?,
                    target_type: row.try_get("target_type")?,
                    target_id: row.try_get("target_id")?,
                    school_id: row.try_get("school_id")?,
                    details: row.try_get("details_json")?,
                    request_id: row.try_get("request_id")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }
}
