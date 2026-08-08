use crate::repositories::{BaseRepository, Repository, RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

pub const ASSIGNMENT_PERSONALIZATION_MODEL: &str = "deepseek-chat";
pub const ASSIGNMENT_PERSONALIZATION_PROFILE: &str = "assignment_personalization_v1";
pub const ASSIGNMENT_PERSONALIZATION_PROFILE_VERSION: i32 = 1;

#[derive(Debug, Clone)]
pub struct ClaimedAssignmentPersonalizationJob {
    pub id: Uuid,
    pub school_id: Uuid,
    pub assignment_id: Uuid,
    pub student_id: Uuid,
    pub requested_by: Uuid,
    pub attempt_count: i32,
    pub model_name: String,
    pub profile_name: String,
    pub profile_version: i32,
    pub lease_owner: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalizationFailureKind {
    GatewayUnavailable,
    RateLimited,
    InvalidGatewayResponse,
    ProcessingUnavailable,
    ContentRejected,
}

impl PersonalizationFailureKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::GatewayUnavailable => "gateway_unavailable",
            Self::RateLimited => "rate_limited",
            Self::InvalidGatewayResponse => "invalid_gateway_response",
            Self::ProcessingUnavailable => "processing_unavailable",
            Self::ContentRejected => "content_rejected",
        }
    }

    pub fn safe_summary(self) -> &'static str {
        match self {
            Self::GatewayUnavailable => "AI gateway is temporarily unavailable",
            Self::RateLimited => "AI personalization is temporarily rate limited",
            Self::InvalidGatewayResponse => "AI gateway returned an invalid response",
            Self::ProcessingUnavailable => "Personalization processing is temporarily unavailable",
            Self::ContentRejected => "Personalization input was rejected by safety limits",
        }
    }

    pub fn retryable(self) -> bool {
        !matches!(self, Self::ContentRejected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalizationFailureDisposition {
    Requeued,
    FailedPermanently,
    IgnoredInactive,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonalizationQueueSummary {
    pub queued: i64,
    pub running: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub total: i64,
    pub max_attempt_count: i32,
    pub last_completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct AssignmentPersonalizationJobRepository {
    base: BaseRepository,
}

impl AssignmentPersonalizationJobRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Explicit teacher retry remains durable. The target is re-authorized in
    /// SQL and a failed/cancelled job is returned to the queue without creating
    /// another idempotency identity. Already-personalized work is a no-op.
    pub async fn requeue_for_teacher(
        &self,
        requested_by: Uuid,
        assignment_id: Uuid,
        student_id: Uuid,
    ) -> RepositoryResult<Uuid> {
        let row = sqlx::query(
            r#"
            WITH target AS (
                SELECT
                    custom_assignment.id AS custom_assignment_id,
                    custom_assignment.prompt_ctx,
                    teacher.school_id,
                    assignment.class_section_id
                FROM assignments assignment
                JOIN teachers teacher ON teacher.id = assignment.teacher_id
                JOIN users teacher_user ON teacher_user.id = teacher.user_id
                JOIN roles teacher_role ON teacher_role.id = teacher_user.role_id
                JOIN class_sections class_section ON class_section.id = assignment.class_section_id
                JOIN teaching_assignments teaching_assignment
                  ON teaching_assignment.teacher_id = teacher.id
                 AND teaching_assignment.class_section_id = assignment.class_section_id
                JOIN students student ON student.id = $2
                JOIN users student_user ON student_user.id = student.user_id
                JOIN enrollments enrollment
                  ON enrollment.student_id = student.id
                 AND enrollment.class_section_id = assignment.class_section_id
                JOIN custom_assignments custom_assignment
                  ON custom_assignment.assignment_id = assignment.id
                 AND custom_assignment.student_id = student.id
                WHERE assignment.id = $1
                  AND assignment.status = 'Published'::assignment_status
                  AND teacher.user_id = $3
                  AND teacher_user.is_active = TRUE
                  AND teacher_role.name::text = 'Teacher'
                  AND teacher.school_id = class_section.school_id
                  AND student.school_id = teacher.school_id
                  AND student_user.school_id = teacher.school_id
                  AND student_user.is_active = TRUE
            ),
            upserted AS (
                INSERT INTO assignment_personalization_jobs (
                    school_id,
                    assignment_id,
                    student_id,
                    class_section_id,
                    requested_by,
                    status,
                    attempt_count,
                    available_at,
                    lease_owner,
                    heartbeat_at,
                    last_error_code,
                    last_error_summary,
                    completed_at,
                    idempotency_key,
                    model_name,
                    profile_name,
                    profile_version
                )
                SELECT
                    target.school_id,
                    $1,
                    $2,
                    target.class_section_id,
                    $3,
                    'queued',
                    0,
                    NOW(),
                    NULL,
                    NULL,
                    NULL,
                    NULL,
                    NULL,
                    concat($1::text, ':', $2::text, ':assignment_personalization_v1:1'),
                    $4,
                    $5,
                    $6
                FROM target
                WHERE target.prompt_ctx IS NULL
                ON CONFLICT (assignment_id, student_id, profile_name, profile_version)
                DO UPDATE SET
                    status = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.status
                        ELSE 'queued'
                    END,
                    attempt_count = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.attempt_count
                        ELSE 0
                    END,
                    available_at = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.available_at
                        ELSE NOW()
                    END,
                    lease_owner = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.lease_owner
                        ELSE NULL
                    END,
                    heartbeat_at = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.heartbeat_at
                        ELSE NULL
                    END,
                    last_error_code = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.last_error_code
                        ELSE NULL
                    END,
                    last_error_summary = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.last_error_summary
                        ELSE NULL
                    END,
                    completed_at = CASE
                        WHEN assignment_personalization_jobs.status = 'succeeded'
                            THEN assignment_personalization_jobs.completed_at
                        ELSE NULL
                    END
                RETURNING id
            )
            SELECT target.custom_assignment_id
            FROM target
            LEFT JOIN upserted ON TRUE
            LIMIT 1
            "#,
        )
        .bind(assignment_id)
        .bind(student_id)
        .bind(requested_by)
        .bind(ASSIGNMENT_PERSONALIZATION_MODEL)
        .bind(ASSIGNMENT_PERSONALIZATION_PROFILE)
        .bind(ASSIGNMENT_PERSONALIZATION_PROFILE_VERSION)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or(RepositoryError::Unauthorized)?;

        Ok(row.try_get("custom_assignment_id")?)
    }

    /// Validate the exact claimed assignment/student relationship under the
    /// original Teacher actor. This is called both before and after the provider
    /// request; if enrollment or authorization changes mid-flight, the outer
    /// transaction rolls back generated content.
    pub async fn authorize_claimed_job(
        &self,
        job: &ClaimedAssignmentPersonalizationJob,
    ) -> RepositoryResult<()> {
        let authorized = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM assignment_personalization_jobs queue
                JOIN assignments assignment ON assignment.id = queue.assignment_id
                JOIN teachers teacher ON teacher.id = assignment.teacher_id
                JOIN users teacher_user ON teacher_user.id = teacher.user_id
                JOIN roles teacher_role ON teacher_role.id = teacher_user.role_id
                JOIN class_sections class_section ON class_section.id = assignment.class_section_id
                JOIN teaching_assignments teaching_assignment
                  ON teaching_assignment.teacher_id = teacher.id
                 AND teaching_assignment.class_section_id = assignment.class_section_id
                JOIN students student ON student.id = queue.student_id
                JOIN users student_user ON student_user.id = student.user_id
                JOIN enrollments enrollment
                  ON enrollment.student_id = student.id
                 AND enrollment.class_section_id = assignment.class_section_id
                JOIN custom_assignments custom_assignment
                  ON custom_assignment.assignment_id = assignment.id
                 AND custom_assignment.student_id = student.id
                WHERE queue.id = $1
                  AND queue.status = 'running'
                  AND queue.lease_owner = $2
                  AND queue.requested_by = $3
                  AND queue.school_id = $4
                  AND queue.assignment_id = $5
                  AND queue.student_id = $6
                  AND assignment.status = 'Published'::assignment_status
                  AND teacher.user_id = queue.requested_by
                  AND teacher_user.is_active = TRUE
                  AND teacher_role.name::text = 'Teacher'
                  AND teacher.school_id = queue.school_id
                  AND class_section.school_id = queue.school_id
                  AND student.school_id = queue.school_id
                  AND student_user.school_id = queue.school_id
                  AND student_user.is_active = TRUE
            )
            "#,
        )
        .bind(job.id)
        .bind(job.lease_owner)
        .bind(job.requested_by)
        .bind(job.school_id)
        .bind(job.assignment_id)
        .bind(job.student_id)
        .fetch_one(&*self.base.pool())
        .await?;

        if !authorized {
            return Err(RepositoryError::Unauthorized);
        }
        Ok(())
    }

    pub async fn heartbeat(&self, job_id: Uuid, lease_owner: Uuid) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET heartbeat_at = NOW()
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job_id)
        .bind(lease_owner)
        .execute(&*self.base.pool())
        .await?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::Validation(
                "Personalization job lease is no longer active".into(),
            ));
        }
        Ok(())
    }

    pub async fn complete(&self, job_id: Uuid, lease_owner: Uuid) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET status = 'succeeded',
                completed_at = NOW(),
                lease_owner = NULL,
                heartbeat_at = NULL,
                last_error_code = NULL,
                last_error_summary = NULL
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job_id)
        .bind(lease_owner)
        .execute(&*self.base.pool())
        .await?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::Validation(
                "Personalization job lease changed before completion".into(),
            ));
        }
        Ok(())
    }

    pub async fn cancel_claimed_job(
        &self,
        job_id: Uuid,
        lease_owner: Uuid,
    ) -> RepositoryResult<()> {
        sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET status = 'cancelled',
                completed_at = NOW(),
                lease_owner = NULL,
                heartbeat_at = NULL,
                last_error_code = 'authorization_revoked',
                last_error_summary = 'Assignment personalization authorization is no longer valid'
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job_id)
        .bind(lease_owner)
        .execute(&*self.base.pool())
        .await?;
        Ok(())
    }

    pub async fn record_failure(
        &self,
        job: &ClaimedAssignmentPersonalizationJob,
        kind: PersonalizationFailureKind,
        retry_after_seconds: u64,
        max_attempts: i32,
    ) -> RepositoryResult<PersonalizationFailureDisposition> {
        let active = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM assignment_personalization_jobs
                WHERE id = $1
                  AND status = 'running'
                  AND lease_owner = $2
            )
            "#,
        )
        .bind(job.id)
        .bind(job.lease_owner)
        .fetch_one(&*self.base.pool())
        .await?;
        if !active {
            return Ok(PersonalizationFailureDisposition::IgnoredInactive);
        }

        let max_attempts = max_attempts.clamp(1, 10);
        let should_retry = kind.retryable() && job.attempt_count < max_attempts;
        if should_retry {
            let exponential = 2_i64.pow(job.attempt_count.clamp(1, 10) as u32);
            let delay_seconds = exponential.max(retry_after_seconds.min(3_600) as i64);
            sqlx::query(
                r#"
                UPDATE assignment_personalization_jobs
                SET status = 'queued',
                    available_at = NOW() + make_interval(secs => $3::DOUBLE PRECISION),
                    lease_owner = NULL,
                    heartbeat_at = NULL,
                    last_error_code = $4,
                    last_error_summary = $5
                WHERE id = $1
                  AND status = 'running'
                  AND lease_owner = $2
                "#,
            )
            .bind(job.id)
            .bind(job.lease_owner)
            .bind(delay_seconds)
            .bind(kind.code())
            .bind(kind.safe_summary())
            .execute(&*self.base.pool())
            .await?;
            return Ok(PersonalizationFailureDisposition::Requeued);
        }

        sqlx::query(
            r#"
            UPDATE assignment_personalization_jobs
            SET status = 'failed',
                completed_at = NOW(),
                lease_owner = NULL,
                heartbeat_at = NULL,
                last_error_code = $3,
                last_error_summary = $4
            WHERE id = $1
              AND status = 'running'
              AND lease_owner = $2
            "#,
        )
        .bind(job.id)
        .bind(job.lease_owner)
        .bind(kind.code())
        .bind(kind.safe_summary())
        .execute(&*self.base.pool())
        .await?;
        Ok(PersonalizationFailureDisposition::FailedPermanently)
    }

    /// Safe teacher-facing metrics. No prompts, generated content, provider
    /// payloads, secrets, or raw error bodies are returned.
    pub async fn summary_for_teacher(
        &self,
        requested_by: Uuid,
    ) -> RepositoryResult<PersonalizationQueueSummary> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'queued') AS queued,
                COUNT(*) FILTER (WHERE status = 'running') AS running,
                COUNT(*) FILTER (WHERE status = 'succeeded') AS succeeded,
                COUNT(*) FILTER (WHERE status = 'failed') AS failed,
                COUNT(*) FILTER (WHERE status = 'cancelled') AS cancelled,
                COUNT(*) AS total,
                COALESCE(MAX(attempt_count), 0) AS max_attempt_count,
                MAX(completed_at) AS last_completed_at
            FROM assignment_personalization_jobs
            WHERE requested_by = $1
              AND school_id = (SELECT school_id FROM users WHERE id = $1)
            "#,
        )
        .bind(requested_by)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(PersonalizationQueueSummary {
            queued: row.try_get("queued")?,
            running: row.try_get("running")?,
            succeeded: row.try_get("succeeded")?,
            failed: row.try_get("failed")?,
            cancelled: row.try_get("cancelled")?,
            total: row.try_get("total")?,
            max_attempt_count: row.try_get("max_attempt_count")?,
            last_completed_at: row.try_get("last_completed_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_failure_messages_are_fixed_and_non_sensitive() {
        for kind in [
            PersonalizationFailureKind::GatewayUnavailable,
            PersonalizationFailureKind::RateLimited,
            PersonalizationFailureKind::InvalidGatewayResponse,
            PersonalizationFailureKind::ProcessingUnavailable,
            PersonalizationFailureKind::ContentRejected,
        ] {
            assert!(kind.code().len() <= 64);
            assert!(kind
                .code()
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_'));
            assert!(kind.safe_summary().len() <= 160);
            let lowered = kind.safe_summary().to_ascii_lowercase();
            for forbidden in [
                "authorization: bearer",
                "api_key",
                "password",
                "postgresql://",
            ] {
                assert!(!lowered.contains(forbidden));
            }
        }
        assert!(!PersonalizationFailureKind::ContentRejected.retryable());
        assert!(PersonalizationFailureKind::RateLimited.retryable());
    }
}
