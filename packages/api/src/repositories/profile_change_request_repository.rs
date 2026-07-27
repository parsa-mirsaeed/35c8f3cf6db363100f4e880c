use crate::domain::{PcrStatus, ProfileChangeRequestId, UserId};
use crate::models::{
    CreateProfileChangeRequestRequest, DecideProfileChangeRequestRequest, ProfileChangeRequest,
};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Profile change request repository for handling profile change request-related database operations
#[derive(Clone)]
pub struct ProfileChangeRequestRepository {
    base: BaseRepository,
}

impl ProfileChangeRequestRepository {
    /// Create a new profile change request repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new profile change request
    pub async fn create(
        &self,
        user_id: UserId,
        request: CreateProfileChangeRequestRequest,
    ) -> RepositoryResult<ProfileChangeRequest> {
        let row = sqlx::query(
            r#"
            INSERT INTO profile_change_requests (user_id, payload_diff, requested_by, status)
            VALUES ($1, $2, $3, $4)
            RETURNING id, user_id, payload_diff, requested_by, status, decided_by, decided_at, created_at
            "#
        )
        .bind(Uuid::from(user_id))
        .bind(&request.payload_diff)
        .bind(Uuid::from(request.user_id))
        .bind(PcrStatus::Pending)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(ProfileChangeRequest {
            id: ProfileChangeRequestId::from(row.get::<uuid::Uuid, _>("id")),
            user_id: UserId::from(row.get::<uuid::Uuid, _>("user_id")),
            payload_diff: row.get("payload_diff"),
            requested_by: UserId::from(row.get::<uuid::Uuid, _>("requested_by")),
            status: row.get("status"),
            decided_by: row
                .get::<Option<uuid::Uuid>, _>("decided_by")
                .map(|uuid| UserId::from(uuid)),
            decided_at: row.get("decided_at"),
            created_at: row.get("created_at"),
        })
    }

    /// Get profile change request by ID
    pub async fn find_by_id(
        &self,
        id: ProfileChangeRequestId,
    ) -> RepositoryResult<ProfileChangeRequest> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, payload_diff, requested_by, status, decided_by, decided_at, created_at
            FROM profile_change_requests
            WHERE id = $1
            "#
        )
        .bind(Uuid::from(id))
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(ProfileChangeRequest {
            id: ProfileChangeRequestId::from(row.get::<uuid::Uuid, _>("id")),
            user_id: UserId::from(row.get::<uuid::Uuid, _>("user_id")),
            payload_diff: row.get("payload_diff"),
            requested_by: UserId::from(row.get::<uuid::Uuid, _>("requested_by")),
            status: row.get("status"),
            decided_by: row
                .get::<Option<uuid::Uuid>, _>("decided_by")
                .map(|uuid| UserId::from(uuid)),
            decided_at: row.get("decided_at"),
            created_at: row.get("created_at"),
        })
    }

    /// Decide on a profile change request (approve/reject)
    pub async fn decide(
        &self,
        id: ProfileChangeRequestId,
        decider_id: UserId,
        request: DecideProfileChangeRequestRequest,
    ) -> RepositoryResult<ProfileChangeRequest> {
        let row = sqlx::query(
            r#"
            UPDATE profile_change_requests
            SET status = $1, decided_by = $2, decided_at = NOW()
            WHERE id = $3
            RETURNING id, user_id, payload_diff, requested_by, status, decided_by, decided_at, created_at
            "#
        )
        .bind(request.status)
        .bind(Uuid::from(decider_id))
        .bind(Uuid::from(id))
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(ProfileChangeRequest {
            id: ProfileChangeRequestId::from(row.get::<uuid::Uuid, _>("id")),
            user_id: UserId::from(row.get::<uuid::Uuid, _>("user_id")),
            payload_diff: row.get("payload_diff"),
            requested_by: UserId::from(row.get::<uuid::Uuid, _>("requested_by")),
            status: row.get("status"),
            decided_by: row
                .get::<Option<uuid::Uuid>, _>("decided_by")
                .map(|uuid| UserId::from(uuid)),
            decided_at: row.get("decided_at"),
            created_at: row.get("created_at"),
        })
    }

    /// List profile change requests for a user
    pub async fn list_by_user(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ProfileChangeRequest>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, payload_diff, requested_by, status, decided_by, decided_at, created_at
            FROM profile_change_requests
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(Uuid::from(user_id))
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ProfileChangeRequest {
                id: ProfileChangeRequestId::from(row.get::<uuid::Uuid, _>("id")),
                user_id: UserId::from(row.get::<uuid::Uuid, _>("user_id")),
                payload_diff: row.get("payload_diff"),
                requested_by: UserId::from(row.get::<uuid::Uuid, _>("requested_by")),
                status: row.get("status"),
                decided_by: row
                    .get::<Option<uuid::Uuid>, _>("decided_by")
                    .map(|uuid| UserId::from(uuid)),
                decided_at: row.get("decided_at"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    /// List profile change requests by status
    pub async fn list_by_status(
        &self,
        status: PcrStatus,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ProfileChangeRequest>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, payload_diff, requested_by, status, decided_by, decided_at, created_at
            FROM profile_change_requests
            WHERE status = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ProfileChangeRequest {
                id: ProfileChangeRequestId::from(row.get::<uuid::Uuid, _>("id")),
                user_id: UserId::from(row.get::<uuid::Uuid, _>("user_id")),
                payload_diff: row.get("payload_diff"),
                requested_by: UserId::from(row.get::<uuid::Uuid, _>("requested_by")),
                status: row.get("status"),
                decided_by: row
                    .get::<Option<uuid::Uuid>, _>("decided_by")
                    .map(|uuid| UserId::from(uuid)),
                decided_at: row.get("decided_at"),
                created_at: row.get("created_at"),
            })
            .collect())
    }
}

#[async_trait]
pub trait ProfileChangeRequestRepositoryTrait: Send + Sync {
    async fn create(
        &self,
        user_id: UserId,
        request: CreateProfileChangeRequestRequest,
    ) -> RepositoryResult<ProfileChangeRequest>;
    async fn find_by_id(
        &self,
        id: ProfileChangeRequestId,
    ) -> RepositoryResult<ProfileChangeRequest>;
    async fn decide(
        &self,
        id: ProfileChangeRequestId,
        decider_id: UserId,
        request: DecideProfileChangeRequestRequest,
    ) -> RepositoryResult<ProfileChangeRequest>;
    async fn list_by_user(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ProfileChangeRequest>>;
    async fn list_by_status(
        &self,
        status: PcrStatus,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ProfileChangeRequest>>;
}

#[async_trait]
impl ProfileChangeRequestRepositoryTrait for ProfileChangeRequestRepository {
    async fn create(
        &self,
        user_id: UserId,
        request: CreateProfileChangeRequestRequest,
    ) -> RepositoryResult<ProfileChangeRequest> {
        self.create(user_id, request).await
    }

    async fn find_by_id(
        &self,
        id: ProfileChangeRequestId,
    ) -> RepositoryResult<ProfileChangeRequest> {
        self.find_by_id(id).await
    }

    async fn decide(
        &self,
        id: ProfileChangeRequestId,
        decider_id: UserId,
        request: DecideProfileChangeRequestRequest,
    ) -> RepositoryResult<ProfileChangeRequest> {
        self.decide(id, decider_id, request).await
    }

    async fn list_by_user(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ProfileChangeRequest>> {
        self.list_by_user(user_id, limit, offset).await
    }

    async fn list_by_status(
        &self,
        status: PcrStatus,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ProfileChangeRequest>> {
        self.list_by_status(status, limit, offset).await
    }
}
