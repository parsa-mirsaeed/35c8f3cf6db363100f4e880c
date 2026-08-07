use crate::domain::{AuditLogId, UserId};
use crate::models::{AuditLog, CreateAuditLogRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use crate::utils::errors::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Audit log repository for handling audit log database operations
#[derive(Clone)]
pub struct AuditLogRepository {
    base: BaseRepository,
}

impl AuditLogRepository {
    /// Create a new audit log repository
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new audit log entry
    pub async fn create_internal(
        &self,
        request: CreateAuditLogRequest,
    ) -> RepositoryResult<AuditLog> {
        let row = sqlx::query!(
            r#"
            INSERT INTO audit_logs (actor_id, action, entity, entity_id, before, after, ip, user_agent, at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, actor_id, action, entity, entity_id, before, after, ip, user_agent, at
            "#,
            Uuid::from(request.actor_id),
            request.action,
            request.entity,
            request.entity_id.map(|id| Uuid::from(id)),
            request.before,
            request.after,
            request.ip.map(|ip_addr| IpNetwork::from(ip_addr)),
            request.user_agent,
            request.at
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(AuditLog {
            id: row.id.into(),
            actor_id: row.actor_id.into(),
            action: row.action,
            entity: row.entity,
            entity_id: row.entity_id.map(|id| id.into()),
            before: row.before,
            after: row.after,
            ip: row.ip.map(|ip_network| ip_network.ip()),
            user_agent: row.user_agent,
            at: row.at,
        })
    }

    /// Get audit log by ID
    pub async fn find_by_id_internal(
        &self,
        audit_log_id: AuditLogId,
    ) -> RepositoryResult<AuditLog> {
        let uuid = Uuid::from(audit_log_id);
        let row = sqlx::query!(
            r#"
            SELECT id, actor_id, action, entity, entity_id, before, after, ip, user_agent, at
            FROM audit_logs
            WHERE id = $1
            "#,
            uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "AuditLog".to_string(),
            id: audit_log_id.to_string(),
        })?;

        Ok(AuditLog {
            id: row.id.into(),
            actor_id: row.actor_id.into(),
            action: row.action,
            entity: row.entity,
            entity_id: row.entity_id.map(|id| id.into()),
            before: row.before,
            after: row.after,
            ip: row.ip.map(|ip_network| ip_network.ip()),
            user_agent: row.user_agent,
            at: row.at,
        })
    }

    /// List audit logs with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<AuditLog>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, actor_id, action, entity, entity_id, before, after, ip, user_agent, at
            FROM audit_logs
            ORDER BY at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut audit_logs = Vec::new();
        for row in rows {
            audit_logs.push(AuditLog {
                id: row.id.into(),
                actor_id: row.actor_id.into(),
                action: row.action,
                entity: row.entity,
                entity_id: row.entity_id.map(|id| id.into()),
                before: row.before,
                after: row.after,
                ip: row.ip.map(|ip_network| ip_network.ip()),
                user_agent: row.user_agent,
                at: row.at,
            });
        }

        Ok(audit_logs)
    }

    /// Count total audit logs
    pub async fn count(&self) -> RepositoryResult<i64> {
        let row: sqlx::postgres::PgRow = sqlx::query("SELECT COUNT(*) as count FROM audit_logs")
            .fetch_one(&*self.base.pool())
            .await?;

        Ok(row.get("count"))
    }

    /// Get audit logs by actor
    pub async fn find_by_actor(
        &self,
        actor_id: UserId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<AuditLog>> {
        let uuid = Uuid::from(actor_id);
        let rows = sqlx::query!(
            r#"
            SELECT id, actor_id, action, entity, entity_id, before, after, ip, user_agent, at
            FROM audit_logs
            WHERE actor_id = $1
            ORDER BY at DESC
            LIMIT $2 OFFSET $3
            "#,
            uuid,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut audit_logs = Vec::new();
        for row in rows {
            audit_logs.push(AuditLog {
                id: row.id.into(),
                actor_id: row.actor_id.into(),
                action: row.action,
                entity: row.entity,
                entity_id: row.entity_id.map(|id| id.into()),
                before: row.before,
                after: row.after,
                ip: row.ip.map(|ip_network| ip_network.ip()),
                user_agent: row.user_agent,
                at: row.at,
            });
        }

        Ok(audit_logs)
    }

    /// Get audit logs by entity
    pub async fn find_by_entity(
        &self,
        entity: &str,
        entity_id: Option<uuid::Uuid>,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<AuditLog>> {
        let rows = if let Some(id) = entity_id {
            sqlx::query(
                r#"
                SELECT id, actor_id, action, entity, entity_id, before, after, ip, user_agent, at
                FROM audit_logs
                WHERE entity = $1 AND entity_id = $2
                ORDER BY at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(entity)
            .bind(id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&*self.base.pool())
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT id, actor_id, action, entity, entity_id, before, after, ip, user_agent, at
                FROM audit_logs
                WHERE entity = $1 AND entity_id IS NULL
                ORDER BY at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(entity)
            .bind(limit)
            .bind(offset)
            .fetch_all(&*self.base.pool())
            .await?
        };

        let mut audit_logs = Vec::new();
        for row in rows {
            let id: Uuid = row.get("id");
            let actor_id: Uuid = row.get("actor_id");
            let action: String = row.get("action");
            let entity_name: String = row.get("entity");
            let db_entity_id: Option<Uuid> = row.get("entity_id");
            let before: Option<Value> = row.get("before");
            let after: Option<Value> = row.get("after");
            let ip_network: Option<IpNetwork> = row.get("ip");
            let ip = ip_network.map(|ip_network| ip_network.ip());
            let user_agent: Option<String> = row.get("user_agent");
            let at: DateTime<Utc> = row.get("at");

            audit_logs.push(AuditLog {
                id: id.into(),
                actor_id: actor_id.into(),
                action,
                entity: entity_name,
                entity_id: db_entity_id.map(|id| id.into()),
                before,
                after,
                ip,
                user_agent,
                at,
            });
        }

        Ok(audit_logs)
    }

    /// Get audit logs by date range
    pub async fn find_by_date_range(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<AuditLog>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, actor_id, action, entity, entity_id, before, after, ip, user_agent, at
            FROM audit_logs
            WHERE at >= $1 AND at <= $2
            ORDER BY at DESC
            LIMIT $3 OFFSET $4
            "#,
            start_date,
            end_date,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut audit_logs = Vec::new();
        for row in rows {
            audit_logs.push(AuditLog {
                id: row.id.into(),
                actor_id: row.actor_id.into(),
                action: row.action,
                entity: row.entity,
                entity_id: row.entity_id.map(|id| id.into()),
                before: row.before,
                after: row.after,
                ip: row.ip.map(|ip_network| ip_network.ip()),
                user_agent: row.user_agent,
                at: row.at,
            });
        }

        Ok(audit_logs)
    }

    /// Delete old audit logs (cleanup)
    pub async fn delete_older_than(&self, cutoff_date: DateTime<Utc>) -> RepositoryResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM audit_logs
            WHERE at < $1
            "#,
            cutoff_date
        )
        .execute(&*self.base.pool())
        .await?;

        Ok(result.rows_affected())
    }
}

#[async_trait]
impl crate::repositories::traits::AuditLogRepository for AuditLogRepository {
    async fn create(&self, request: CreateAuditLogRequest) -> Result<AuditLog, AppError> {
        self.create_internal(request).await.map_err(AppError::from)
    }

    async fn find_by_id(&self, id: AuditLogId) -> Result<Option<AuditLog>, AppError> {
        self.find_by_id_internal(id)
            .await
            .map(Some)
            .map_err(AppError::from)
    }

    async fn find_all(&self) -> Result<Vec<AuditLog>, AppError> {
        self.list(1000, 0).await.map_err(AppError::from)
    }
}
