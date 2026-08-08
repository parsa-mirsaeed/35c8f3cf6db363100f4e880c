use super::{
    AssignmentPersonalizationJobRepository, AuthorizedAssignmentRepository, AuthorizedTeacher,
    ClaimedAssignmentPersonalizationJob, PersonalizationFailureDisposition,
    PersonalizationFailureKind,
};
use crate::models::CreateAssignmentRequest;
use crate::rls_context::{AuthorizedActor, AuthorizedPool, AuthorizedTx};
use chrono::{Duration, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::future::Future;
use std::sync::Arc;
use uuid::Uuid;

// The production queue claim is intentionally global. These database-backed
// tests share one PostgreSQL database, so serialize queue fixtures that can
// claim/reconcile jobs and explicitly prioritize the fixture under test. This
// prevents one test from consuming another test's valid global queue work.
static QUEUE_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn run_as<T, F>(pool: &PgPool, actor: AuthorizedActor, future: F) -> T
where
    F: Future<Output = T>,
{
    AuthorizedTx::begin(pool, actor)
        .await
        .expect("begin authorized personalization test transaction")
        .scope(future, |_| true)
        .await
        .expect("finish authorized personalization test transaction")
}

fn actor(user_id: Uuid, role: &str, school_id: Uuid) -> AuthorizedActor {
    AuthorizedActor::new(user_id, role, Some(school_id)).expect("valid test actor")
}

async fn role_id(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM roles WHERE name::text = $1 LIMIT 1")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|error| panic!("missing {name} role fixture: {error}"))
}

async fn insert_user(
    pool: &PgPool,
    suffix: &str,
    label: &str,
    role_id: Uuid,
    school_id: Uuid,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata)
        VALUES ($1, $2, $3, $4, $5, TRUE, '{}'::jsonb)
        "#,
    )
    .bind(id)
    .bind(label)
    .bind(format!(
        "{}-{suffix}@example.test",
        label.to_lowercase().replace(' ', "-")
    ))
    .bind(role_id)
    .bind(school_id)
    .execute(pool)
    .await
    .unwrap_or_else(|error| panic!("insert {label}: {error}"));
    id
}

async fn insert_teacher(pool: &PgPool, user_id: Uuid, school_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teachers (id, user_id, school_id, subject, created_at) VALUES ($1, $2, $3, NULL, NOW())",
    )
    .bind(id)
    .bind(user_id)
    .bind(school_id)
    .execute(pool)
    .await
    .expect("insert teacher");
    id
}

async fn insert_student(pool: &PgPool, user_id: Uuid, school_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO students (id, user_id, school_id, parent_id, talent_profile_ref, created_at) VALUES ($1, $2, $3, NULL, NULL, NOW())",
    )
    .bind(id)
    .bind(user_id)
    .bind(school_id)
    .execute(pool)
    .await
    .expect("insert student");
    id
}

struct QueueFixture {
    pool: Arc<PgPool>,
    repository: AuthorizedAssignmentRepository,
    teacher_user: Uuid,
    school_id: Uuid,
    teacher: AuthorizedTeacher,
    assignment_id: Uuid,
    student_ids: Vec<Uuid>,
}

async fn queue_fixture(student_count: usize) -> QueueFixture {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for personalization queue tests");
    let pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(8)
            .connect(&database_url)
            .await
            .expect("connect personalization queue database"),
    );
    let suffix = Uuid::new_v4().simple().to_string();
    let school_id = Uuid::new_v4();
    sqlx::query("INSERT INTO schools (id, name) VALUES ($1, $2)")
        .bind(school_id)
        .bind(format!("Personalization Queue School {suffix}"))
        .execute(&*pool)
        .await
        .expect("insert school");

    let subject_id = Uuid::new_v4();
    sqlx::query("INSERT INTO subjects (id, code, name) VALUES ($1, $2, $3)")
        .bind(subject_id)
        .bind(format!("PERS-{suffix}"))
        .bind(format!("Personalization Subject {suffix}"))
        .execute(&*pool)
        .await
        .expect("insert subject");

    let teacher_role = role_id(&pool, "Teacher").await;
    let student_role = role_id(&pool, "Student").await;
    let teacher_user = insert_user(
        &pool,
        &suffix,
        "Personalization Teacher",
        teacher_role,
        school_id,
    )
    .await;
    let teacher_id = insert_teacher(&pool, teacher_user, school_id).await;

    let class_section_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO class_sections (id, school_id, subject_id, name, term) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(class_section_id)
    .bind(school_id)
    .bind(subject_id)
    .bind(format!("Personalization Class {suffix}"))
    .bind("2026")
    .execute(&*pool)
    .await
    .expect("insert class");
    sqlx::query(
        "INSERT INTO teaching_assignments (id, class_section_id, teacher_id) VALUES ($1, $2, $3)",
    )
    .bind(Uuid::new_v4())
    .bind(class_section_id)
    .bind(teacher_id)
    .execute(&*pool)
    .await
    .expect("insert teaching assignment");

    let mut student_ids = Vec::with_capacity(student_count);
    for index in 0..student_count {
        let user_id = insert_user(
            &pool,
            &suffix,
            &format!("Personalization Student {index}"),
            student_role,
            school_id,
        )
        .await;
        let student_id = insert_student(&pool, user_id, school_id).await;
        sqlx::query(
            "INSERT INTO enrollments (id, class_section_id, student_id, enrolled_at) VALUES ($1, $2, $3, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(class_section_id)
        .bind(student_id)
        .execute(&*pool)
        .await
        .expect("insert enrollment");
        student_ids.push(student_id);
    }

    let repository = AuthorizedAssignmentRepository::new(pool.clone());
    let teacher = run_as(
        pool.as_ref(),
        actor(teacher_user, "Teacher", school_id),
        repository.resolve_active_teacher(teacher_user, "Teacher"),
    )
    .await
    .expect("resolve active teacher");
    let assignment = run_as(
        pool.as_ref(),
        actor(teacher_user, "Teacher", school_id),
        repository.create_for_teacher(
            teacher,
            CreateAssignmentRequest {
                class_section_id: class_section_id.into(),
                subject_id: subject_id.into(),
                lecture_id: None,
                lecture_title: None,
                lecture_number: None,
                title: "Durable personalization fixture".into(),
                body: "Personalize this assignment safely".into(),
                due_at: Utc::now() + Duration::days(7),
                material_ids: None,
            },
        ),
    )
    .await
    .expect("create assignment");
    let assignment_id: Uuid = assignment.id.into();

    run_as(
        pool.as_ref(),
        actor(teacher_user, "Teacher", school_id),
        repository.publish_for_teacher(teacher, assignment.id),
    )
    .await
    .expect("publish assignment");

    QueueFixture {
        pool,
        repository,
        teacher_user,
        school_id,
        teacher,
        assignment_id,
        student_ids,
    }
}

async fn prioritize_assignment_jobs(pool: &PgPool, assignment_id: Uuid) {
    sqlx::query(
        r#"
        UPDATE assignment_personalization_jobs
        SET available_at = TIMESTAMPTZ '1970-01-01 00:00:00+00'
        WHERE assignment_id = $1
          AND status = 'queued'
        "#,
    )
    .bind(assignment_id)
    .execute(pool)
    .await
    .expect("prioritize personalization fixture jobs");
}

async fn claim_next(pool: &PgPool, worker_id: Uuid) -> Option<ClaimedAssignmentPersonalizationJob> {
    let authorized_pool = AuthorizedPool::new();
    run_as(pool, AuthorizedActor::system_queue(worker_id), async {
        let row = sqlx::query(
            r#"
            SELECT
                job_id,
                school_id,
                assignment_id,
                student_id,
                requested_by,
                attempt_count,
                model_name,
                profile_name,
                profile_version,
                lease_owner
            FROM public.claim_next_assignment_personalization_job($1)
            "#,
        )
        .bind(worker_id)
        .fetch_optional(&authorized_pool)
        .await
        .expect("claim personalization job");

        row.map(|row| ClaimedAssignmentPersonalizationJob {
            id: row.get("job_id"),
            school_id: row.get("school_id"),
            assignment_id: row.get("assignment_id"),
            student_id: row.get("student_id"),
            requested_by: row.get("requested_by"),
            attempt_count: row.get("attempt_count"),
            model_name: row.get("model_name"),
            profile_name: row.get("profile_name"),
            profile_version: row.get("profile_version"),
            lease_owner: row.get("lease_owner"),
        })
    })
    .await
}

#[cfg(feature = "server")]
#[tokio::test]
async fn publication_enqueues_atomically_and_duplicate_publish_is_idempotent() {
    let _queue_guard = QUEUE_TEST_LOCK.lock().await;
    let fixture = queue_fixture(2).await;

    let custom_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM custom_assignments WHERE assignment_id = $1")
            .bind(fixture.assignment_id)
            .fetch_one(&*fixture.pool)
            .await
            .expect("count custom assignments");
    let job_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assignment_personalization_jobs WHERE assignment_id = $1",
    )
    .bind(fixture.assignment_id)
    .fetch_one(&*fixture.pool)
    .await
    .expect("count personalization jobs");
    assert_eq!(custom_count, 2);
    assert_eq!(job_count, custom_count);

    run_as(
        fixture.pool.as_ref(),
        actor(fixture.teacher_user, "Teacher", fixture.school_id),
        fixture
            .repository
            .publish_for_teacher(fixture.teacher, fixture.assignment_id.into()),
    )
    .await
    .expect("duplicate publish remains idempotent");

    let job_count_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assignment_personalization_jobs WHERE assignment_id = $1",
    )
    .bind(fixture.assignment_id)
    .fetch_one(&*fixture.pool)
    .await
    .expect("count jobs after duplicate publish");
    assert_eq!(job_count_after, 2);

    let job_repository = AssignmentPersonalizationJobRepository::new(fixture.pool.clone());
    let summary = run_as(
        fixture.pool.as_ref(),
        actor(fixture.teacher_user, "Teacher", fixture.school_id),
        job_repository.summary_for_teacher(fixture.teacher_user),
    )
    .await
    .expect("teacher queue summary");
    assert_eq!(summary.total, 2);
    assert_eq!(summary.queued, 2);
    assert_eq!(summary.failed, 0);

    let persisted = sqlx::query(
        r#"
        SELECT model_name, profile_name, profile_version, last_error_code, last_error_summary
        FROM assignment_personalization_jobs
        WHERE assignment_id = $1
        "#,
    )
    .bind(fixture.assignment_id)
    .fetch_all(&*fixture.pool)
    .await
    .expect("read queue metadata");
    assert_eq!(persisted.len(), 2);
    for row in persisted {
        assert_eq!(row.get::<String, _>("model_name"), "deepseek-chat");
        assert_eq!(
            row.get::<String, _>("profile_name"),
            "assignment_personalization_v1"
        );
        assert_eq!(row.get::<i32, _>("profile_version"), 1);
        assert!(row.get::<Option<String>, _>("last_error_code").is_none());
        assert!(row.get::<Option<String>, _>("last_error_summary").is_none());
    }
}

#[cfg(feature = "server")]
#[tokio::test]
async fn partial_completion_resumes_remaining_student_and_stale_lease_is_reclaimed() {
    let _queue_guard = QUEUE_TEST_LOCK.lock().await;
    let fixture = queue_fixture(2).await;
    let completed_student = fixture.student_ids[0];
    sqlx::query(
        r#"
        UPDATE custom_assignments
        SET prompt_ctx = '{"personalized_assignment":{"title":"done"}}'::jsonb
        WHERE assignment_id = $1 AND student_id = $2
        "#,
    )
    .bind(fixture.assignment_id)
    .bind(completed_student)
    .execute(&*fixture.pool)
    .await
    .expect("mark one student personalized");
    prioritize_assignment_jobs(fixture.pool.as_ref(), fixture.assignment_id).await;

    let worker_id = Uuid::new_v4();
    let claimed = claim_next(fixture.pool.as_ref(), worker_id)
        .await
        .expect("remaining student should be claimable");
    assert_ne!(claimed.student_id, completed_student);
    assert_eq!(claimed.assignment_id, fixture.assignment_id);
    assert_eq!(claimed.attempt_count, 1);

    let completed_status: String = sqlx::query_scalar(
        "SELECT status FROM assignment_personalization_jobs WHERE assignment_id = $1 AND student_id = $2",
    )
    .bind(fixture.assignment_id)
    .bind(completed_student)
    .fetch_one(&*fixture.pool)
    .await
    .expect("read reconciled completed job");
    assert_eq!(completed_status, "succeeded");

    sqlx::query(
        "UPDATE assignment_personalization_jobs SET heartbeat_at = NOW() - INTERVAL '10 minutes' WHERE id = $1",
    )
    .bind(claimed.id)
    .execute(&*fixture.pool)
    .await
    .expect("age worker lease");

    let authorized_pool = AuthorizedPool::new();
    let recovered: i64 = run_as(
        fixture.pool.as_ref(),
        AuthorizedActor::system_queue(worker_id),
        sqlx::query_scalar("SELECT public.recover_stale_assignment_personalization_jobs(60)")
            .fetch_one(&authorized_pool),
    )
    .await
    .expect("recover stale personalization job");
    assert_eq!(recovered, 1);

    let status: String =
        sqlx::query_scalar("SELECT status FROM assignment_personalization_jobs WHERE id = $1")
            .bind(claimed.id)
            .fetch_one(&*fixture.pool)
            .await
            .expect("read recovered status");
    assert_eq!(status, "queued");
}

#[cfg(feature = "server")]
#[tokio::test]
async fn revoked_enrollment_is_cancelled_before_processing() {
    let _queue_guard = QUEUE_TEST_LOCK.lock().await;
    let fixture = queue_fixture(1).await;
    let student_id = fixture.student_ids[0];
    let target_job_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM assignment_personalization_jobs WHERE assignment_id = $1 AND student_id = $2",
    )
    .bind(fixture.assignment_id)
    .bind(student_id)
    .fetch_one(&*fixture.pool)
    .await
    .expect("read target personalization job");

    sqlx::query("DELETE FROM enrollments WHERE student_id = $1")
        .bind(student_id)
        .execute(&*fixture.pool)
        .await
        .expect("remove enrollment");

    let worker_id = Uuid::new_v4();
    if let Some(claimed) = claim_next(fixture.pool.as_ref(), worker_id).await {
        assert_ne!(
            claimed.id, target_job_id,
            "revoked target must be cancelled during reconciliation, never claimed"
        );
    }

    let row = sqlx::query(
        r#"
        SELECT status, last_error_code, last_error_summary
        FROM assignment_personalization_jobs
        WHERE assignment_id = $1 AND student_id = $2
        "#,
    )
    .bind(fixture.assignment_id)
    .bind(student_id)
    .fetch_one(&*fixture.pool)
    .await
    .expect("read cancelled job");
    assert_eq!(row.get::<String, _>("status"), "cancelled");
    assert_eq!(
        row.get::<Option<String>, _>("last_error_code").as_deref(),
        Some("authorization_revoked")
    );
    assert_eq!(
        row.get::<Option<String>, _>("last_error_summary")
            .as_deref(),
        Some("Assignment personalization authorization is no longer valid")
    );
}

#[cfg(feature = "server")]
#[tokio::test]
async fn retry_backoff_is_bounded_and_cross_school_status_is_isolated() {
    let _queue_guard = QUEUE_TEST_LOCK.lock().await;
    let fixture = queue_fixture(1).await;
    prioritize_assignment_jobs(fixture.pool.as_ref(), fixture.assignment_id).await;

    let worker_id = Uuid::new_v4();
    let claimed = claim_next(fixture.pool.as_ref(), worker_id)
        .await
        .expect("claim personalization job");
    assert_eq!(claimed.assignment_id, fixture.assignment_id);
    let repository = AssignmentPersonalizationJobRepository::new(fixture.pool.clone());

    let disposition = run_as(
        fixture.pool.as_ref(),
        actor(fixture.teacher_user, "Teacher", fixture.school_id),
        repository.record_failure(&claimed, PersonalizationFailureKind::RateLimited, 120, 5),
    )
    .await
    .expect("record retryable failure");
    assert_eq!(disposition, PersonalizationFailureDisposition::Requeued);

    let row = sqlx::query(
        r#"
        SELECT status, available_at > NOW() AS delayed, last_error_code, last_error_summary
        FROM assignment_personalization_jobs
        WHERE id = $1
        "#,
    )
    .bind(claimed.id)
    .fetch_one(&*fixture.pool)
    .await
    .expect("read retry state");
    assert_eq!(row.get::<String, _>("status"), "queued");
    assert!(row.get::<bool, _>("delayed"));
    assert_eq!(
        row.get::<Option<String>, _>("last_error_code").as_deref(),
        Some("rate_limited")
    );
    let safe_summary = row
        .get::<Option<String>, _>("last_error_summary")
        .expect("safe summary");
    assert!(!safe_summary.contains("provider payload"));
    assert!(!safe_summary.to_ascii_lowercase().contains("api_key"));

    let suffix = Uuid::new_v4().simple().to_string();
    let other_school = Uuid::new_v4();
    sqlx::query("INSERT INTO schools (id, name) VALUES ($1, $2)")
        .bind(other_school)
        .bind(format!("Other Queue School {suffix}"))
        .execute(&*fixture.pool)
        .await
        .expect("insert other school");
    let teacher_role = role_id(&fixture.pool, "Teacher").await;
    let other_user = insert_user(
        &fixture.pool,
        &suffix,
        "Other Queue Teacher",
        teacher_role,
        other_school,
    )
    .await;
    insert_teacher(&fixture.pool, other_user, other_school).await;

    let other_repository = AuthorizedAssignmentRepository::new(fixture.pool.clone());
    run_as(
        fixture.pool.as_ref(),
        actor(other_user, "Teacher", other_school),
        other_repository.resolve_active_teacher(other_user, "Teacher"),
    )
    .await
    .expect("resolve other teacher");
    let other_summary = run_as(
        fixture.pool.as_ref(),
        actor(other_user, "Teacher", other_school),
        repository.summary_for_teacher(other_user),
    )
    .await
    .expect("other teacher summary");
    assert_eq!(other_summary.total, 0);
}

#[cfg(feature = "server")]
#[tokio::test]
async fn explicit_retry_reuses_the_stable_job_identity() {
    let _queue_guard = QUEUE_TEST_LOCK.lock().await;
    let fixture = queue_fixture(1).await;
    let student_id = fixture.student_ids[0];
    let original_job_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM assignment_personalization_jobs WHERE assignment_id = $1 AND student_id = $2",
    )
    .bind(fixture.assignment_id)
    .bind(student_id)
    .fetch_one(&*fixture.pool)
    .await
    .expect("read original job id");

    sqlx::query(
        r#"
        UPDATE assignment_personalization_jobs
        SET status = 'failed', attempt_count = 5, completed_at = NOW(),
            last_error_code = 'gateway_unavailable',
            last_error_summary = 'AI gateway is temporarily unavailable'
        WHERE id = $1
        "#,
    )
    .bind(original_job_id)
    .execute(&*fixture.pool)
    .await
    .expect("force failed state");

    let custom_assignment_id = run_as(
        fixture.pool.as_ref(),
        actor(fixture.teacher_user, "Teacher", fixture.school_id),
        AssignmentPersonalizationJobRepository::new(fixture.pool.clone()).requeue_for_teacher(
            fixture.teacher_user,
            fixture.assignment_id,
            student_id,
        ),
    )
    .await
    .expect("explicit durable retry");
    assert_ne!(custom_assignment_id, Uuid::nil());

    let row = sqlx::query(
        "SELECT id, status, attempt_count FROM assignment_personalization_jobs WHERE assignment_id = $1 AND student_id = $2",
    )
    .bind(fixture.assignment_id)
    .bind(student_id)
    .fetch_one(&*fixture.pool)
    .await
    .expect("read requeued job");
    assert_eq!(row.get::<Uuid, _>("id"), original_job_id);
    assert_eq!(row.get::<String, _>("status"), "queued");
    assert_eq!(row.get::<i32, _>("attempt_count"), 0);
}
