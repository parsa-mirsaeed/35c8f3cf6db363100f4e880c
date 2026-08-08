use crate::domain::{PcrStatus, ProfileChangeRequestId, SchoolId, UserId};
use crate::models::{
    CreateProfileChangeRequestRequest, DecideProfileChangeRequestRequest, ProfileChangeRequest,
};
use crate::repositories::{base::*, RepositoryResult};
use async_trait::async_trait;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

/// Profile-change persistence. Public callers should prefer the actor/school
/// scoped methods below rather than fetching a request and authorizing it later.
#[derive(Clone)]
pub struct ProfileChangeRequestRepository {
    base: BaseRepository,
}

impl ProfileChangeRequestRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

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
            "#,
        )
        .bind(Uuid::from(user_id))
        .bind(&request.payload_diff)
        .bind(Uuid::from(request.user_id))
        .bind(PcrStatus::Pending)
        .fetch_one(&*self.base.pool())
        .await?;
        Ok(row_to_request(&row))
    }

    /// Internal compatibility lookup. Browser-facing flows use `find_for_school`
    /// or `list_by_user` so object scope is part of the SQL predicate.
    pub async fn find_by_id(
        &self,
        id: ProfileChangeRequestId,
    ) -> RepositoryResult<ProfileChangeRequest> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, payload_diff, requested_by, status, decided_by, decided_at, created_at
            FROM profile_change_requests
            WHERE id = $1
            "#,
        )
        .bind(Uuid::from(id))
        .fetch_one(&*self.base.pool())
        .await?;
        Ok(row_to_request(&row))
    }

    pub async fn find_for_school(
        &self,
        id: ProfileChangeRequestId,
        school_id: SchoolId,
    ) -> RepositoryResult<ProfileChangeRequest> {
        let row = sqlx::query(
            r#"
            SELECT pcr.id, pcr.user_id, pcr.payload_diff, pcr.requested_by,
                   pcr.status, pcr.decided_by, pcr.decided_at, pcr.created_at
            FROM profile_change_requests pcr
            JOIN users target_user ON target_user.id = pcr.user_id
            WHERE pcr.id = $1
              AND target_user.school_id = $2
            "#,
        )
        .bind(Uuid::from(id))
        .bind(Uuid::from(school_id))
        .fetch_one(&*self.base.pool())
        .await?;
        Ok(row_to_request(&row))
    }

    /// Internal compatibility mutation. Browser-facing manager flows use
    /// `decide_for_school`, which binds target school in the UPDATE predicate.
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
            "#,
        )
        .bind(request.status)
        .bind(Uuid::from(decider_id))
        .bind(Uuid::from(id))
        .fetch_one(&*self.base.pool())
        .await?;
        Ok(row_to_request(&row))
    }

    pub async fn decide_for_school(
        &self,
        id: ProfileChangeRequestId,
        decider_id: UserId,
        school_id: SchoolId,
        request: DecideProfileChangeRequestRequest,
    ) -> RepositoryResult<ProfileChangeRequest> {
        let row = sqlx::query(
            r#"
            UPDATE profile_change_requests AS pcr
            SET status = $1, decided_by = $2, decided_at = NOW()
            WHERE pcr.id = $3
              AND EXISTS (
                  SELECT 1
                  FROM users target_user
                  WHERE target_user.id = pcr.user_id
                    AND target_user.school_id = $4
              )
            RETURNING pcr.id, pcr.user_id, pcr.payload_diff, pcr.requested_by,
                      pcr.status, pcr.decided_by, pcr.decided_at, pcr.created_at
            "#,
        )
        .bind(request.status)
        .bind(Uuid::from(decider_id))
        .bind(Uuid::from(id))
        .bind(Uuid::from(school_id))
        .fetch_one(&*self.base.pool())
        .await?;
        Ok(row_to_request(&row))
    }

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
            "#,
        )
        .bind(Uuid::from(user_id))
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;
        Ok(rows.iter().map(row_to_request).collect())
    }

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
            "#,
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;
        Ok(rows.iter().map(row_to_request).collect())
    }

    pub async fn list_for_school_by_status(
        &self,
        school_id: SchoolId,
        status: PcrStatus,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ProfileChangeRequest>> {
        let rows = sqlx::query(
            r#"
            SELECT pcr.id, pcr.user_id, pcr.payload_diff, pcr.requested_by,
                   pcr.status, pcr.decided_by, pcr.decided_at, pcr.created_at
            FROM profile_change_requests pcr
            JOIN users target_user ON target_user.id = pcr.user_id
            WHERE target_user.school_id = $1
              AND pcr.status = $2
            ORDER BY pcr.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(Uuid::from(school_id))
        .bind(status)
        .bind(limit.clamp(1, 100))
        .bind(offset.max(0))
        .fetch_all(&*self.base.pool())
        .await?;
        Ok(rows.iter().map(row_to_request).collect())
    }
}

fn row_to_request(row: &PgRow) -> ProfileChangeRequest {
    ProfileChangeRequest {
        id: ProfileChangeRequestId::from(row.get::<Uuid, _>("id")),
        user_id: UserId::from(row.get::<Uuid, _>("user_id")),
        payload_diff: row.get("payload_diff"),
        requested_by: UserId::from(row.get::<Uuid, _>("requested_by")),
        status: row.get("status"),
        decided_by: row.get::<Option<Uuid>, _>("decided_by").map(UserId::from),
        decided_at: row.get("decided_at"),
        created_at: row.get("created_at"),
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
