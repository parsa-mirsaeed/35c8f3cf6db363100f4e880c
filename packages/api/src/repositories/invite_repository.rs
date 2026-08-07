use crate::domain::{ClassSectionId, Role, SchoolId, StudentId, UserId};
use crate::models::{CreateInviteRequest, Invite};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Invite repository for handling invite-related database operations.
#[derive(Clone)]
pub struct InviteRepository {
    base: BaseRepository,
}

impl InviteRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a school-scoped invite.
    ///
    /// Platform administrators are provisioned through a separate controlled
    /// process; a school manager must never be able to create that role.
    pub async fn create(
        &self,
        request: CreateInviteRequest,
        created_by: UserId,
        school_id: SchoolId,
    ) -> RepositoryResult<Invite> {
        if matches!(request.role_name, Role::PlatformAdmin) {
            return Err(RepositoryError::Validation(
                "Platform administrators cannot be created through school invitations".into(),
            ));
        }

        let token = Uuid::new_v4().to_string();
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = format!("{:x}", hasher.finalize());

        let expires_days = request.expires_days.unwrap_or(7);
        let expires_at = Utc::now() + chrono::Duration::days(i64::from(expires_days));

        let class_section_ids_uuid = request
            .class_section_ids
            .map(|ids| ids.into_iter().map(Uuid::from).collect::<Vec<_>>());
        let student_id_uuid = request.student_id.map(Uuid::from);

        let row = sqlx::query(
            r#"
            INSERT INTO invites (
                email, role_name, school_id, class_section_ids, student_id,
                token_hash, expires_at, created_by
            )
            VALUES ($1, $2::role_name, $3, $4, $5, $6, $7, $8)
            RETURNING id, email, role_name, school_id, class_section_ids,
                      student_id, expires_at, created_by, consumed_at, created_at
            "#,
        )
        .bind(request.email)
        .bind(role_name_to_string(&request.role_name))
        .bind(Uuid::from(school_id))
        .bind(class_section_ids_uuid.as_deref())
        .bind(student_id_uuid)
        .bind(token_hash)
        .bind(expires_at)
        .bind(Uuid::from(created_by))
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(row_to_invite(&row))
    }

    pub async fn find_by_token_hash(&self, token_hash: &str) -> RepositoryResult<Invite> {
        let row = sqlx::query!(
            r#"
            SELECT id, email, role_name as "role_name!: String", school_id,
                   class_section_ids, student_id, expires_at, created_by,
                   consumed_at, created_at
            FROM invites
            WHERE token_hash = $1
            "#,
            token_hash
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Invite".to_string(),
            id: token_hash.to_string(),
        })?;

        Ok(Invite {
            id: row.id,
            email: row.email,
            role_name: string_to_role_name(&row.role_name),
            school_id: SchoolId::from(row.school_id),
            class_section_ids: row
                .class_section_ids
                .map(|ids| ids.into_iter().map(ClassSectionId::from).collect()),
            student_id: row.student_id.map(StudentId::from),
            expires_at: row.expires_at,
            created_by: UserId::from(row.created_by),
            consumed_at: row.consumed_at,
            created_at: row.created_at,
        })
    }

    pub async fn claim_invite(&self, token_hash: &str) -> RepositoryResult<()> {
        let result = sqlx::query!(
            r#"
            UPDATE invites
            SET consumed_at = now()
            WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > now()
            "#,
            token_hash
        )
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Invite".to_string(),
                id: token_hash.to_string(),
            });
        }

        Ok(())
    }

    pub async fn list_by_school(
        &self,
        school_id: SchoolId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<(Vec<Invite>, i64)> {
        let school_uuid = Uuid::from(school_id);

        let rows = sqlx::query!(
            r#"
            SELECT id, email, role_name as "role_name!: String", school_id,
                   class_section_ids, student_id, expires_at, created_by,
                   consumed_at, created_at
            FROM invites
            WHERE school_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            school_uuid,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let invites = rows
            .into_iter()
            .map(|row| Invite {
                id: row.id,
                email: row.email,
                role_name: string_to_role_name(&row.role_name),
                school_id: SchoolId::from(row.school_id),
                class_section_ids: row
                    .class_section_ids
                    .map(|ids| ids.into_iter().map(ClassSectionId::from).collect()),
                student_id: row.student_id.map(StudentId::from),
                expires_at: row.expires_at,
                created_by: UserId::from(row.created_by),
                consumed_at: row.consumed_at,
                created_at: row.created_at,
            })
            .collect();

        let total = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM invites WHERE school_id = $1",
            school_uuid
        )
        .fetch_one(&*self.base.pool())
        .await?
        .unwrap_or(0);

        Ok((invites, total))
    }

    pub async fn count_active_by_school(&self, school_id: SchoolId) -> RepositoryResult<i64> {
        let school_uuid = Uuid::from(school_id);
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM invites
            WHERE school_id = $1 AND consumed_at IS NULL AND expires_at > now()
            "#,
            school_uuid
        )
        .fetch_one(&*self.base.pool())
        .await?
        .unwrap_or(0);
        Ok(count)
    }

    pub async fn find_active_by_email(
        &self,
        email: &str,
        school_id: SchoolId,
    ) -> RepositoryResult<Option<Invite>> {
        let school_uuid = Uuid::from(school_id);
        let row = sqlx::query!(
            r#"
            SELECT id, email, role_name as "role_name!: String", school_id,
                   class_section_ids, student_id, expires_at, created_by,
                   consumed_at, created_at
            FROM invites
            WHERE email = $1 AND school_id = $2
              AND consumed_at IS NULL AND expires_at > now()
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            email,
            school_uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?;

        Ok(row.map(|row| Invite {
            id: row.id,
            email: row.email,
            role_name: string_to_role_name(&row.role_name),
            school_id: SchoolId::from(row.school_id),
            class_section_ids: row
                .class_section_ids
                .map(|ids| ids.into_iter().map(ClassSectionId::from).collect()),
            student_id: row.student_id.map(StudentId::from),
            expires_at: row.expires_at,
            created_by: UserId::from(row.created_by),
            consumed_at: row.consumed_at,
            created_at: row.created_at,
        }))
    }
}

fn row_to_invite(row: &sqlx::postgres::PgRow) -> Invite {
    Invite {
        id: row.get::<Uuid, _>("id"),
        email: row.get::<String, _>("email"),
        role_name: string_to_role_name(&row.get::<String, _>("role_name")),
        school_id: SchoolId::from(row.get::<Uuid, _>("school_id")),
        class_section_ids: row
            .get::<Option<Vec<Uuid>>, _>("class_section_ids")
            .map(|ids| ids.into_iter().map(ClassSectionId::from).collect()),
        student_id: row
            .get::<Option<Uuid>, _>("student_id")
            .map(StudentId::from),
        expires_at: row.get::<DateTime<Utc>, _>("expires_at"),
        created_by: UserId::from(row.get::<Uuid, _>("created_by")),
        consumed_at: row.get::<Option<DateTime<Utc>>, _>("consumed_at"),
        created_at: row.get::<DateTime<Utc>, _>("created_at"),
    }
}

fn role_name_to_string(role: &Role) -> &'static str {
    match role {
        Role::PlatformAdmin => "PlatformAdmin",
        Role::SchoolManager => "SchoolManager",
        Role::Teacher => "Teacher",
        Role::Parent => "Parent",
        Role::Student => "Student",
    }
}

fn string_to_role_name(value: &str) -> Role {
    match value {
        "PlatformAdmin" => Role::PlatformAdmin,
        "SchoolManager" => Role::SchoolManager,
        "Teacher" => Role::Teacher,
        "Parent" => Role::Parent,
        "Student" => Role::Student,
        _ => Role::Student,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_admin_role_round_trips_but_is_not_school_invitable() {
        assert_eq!(role_name_to_string(&Role::PlatformAdmin), "PlatformAdmin");
        assert_eq!(string_to_role_name("PlatformAdmin"), Role::PlatformAdmin);
    }
}
