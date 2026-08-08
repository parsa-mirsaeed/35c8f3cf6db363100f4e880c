use crate::domain::{AssignmentId, StudentId};
use crate::repositories::{
    AssignmentPersonalizationJobRepository, ClaimedAssignmentPersonalizationJob,
    PersonalizationFailureDisposition, PersonalizationFailureKind, RepositoryError,
};
use crate::rls_context::{AuthorizedActor, AuthorizedPool, AuthorizedTx, RlsContextError};
use crate::services::llm_service::LlmError;
use crate::services::{AssignmentPersonalizationService, PersonalizationError};
use sqlx::{PgPool, Row};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug)]
enum WorkerProcessError {
    Repository(RepositoryError),
    Personalization(PersonalizationError),
}

impl From<RepositoryError> for WorkerProcessError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}

impl From<PersonalizationError> for WorkerProcessError {
    fn from(error: PersonalizationError) -> Self {
        Self::Personalization(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureAction {
    CancelAuthorization,
    Record {
        kind: PersonalizationFailureKind,
        retry_after_seconds: u64,
    },
}

/// Start the durable assignment personalization worker.
///
/// Queue discovery is global only through the narrow SECURITY DEFINER claim and
/// stale-recovery functions. Every actual personalization operation runs as the
/// original active Teacher in the job's school-scoped transaction. Generated
/// content and the job-success transition therefore commit atomically.
pub fn start_assignment_personalization_worker(
    raw_pool: Arc<PgPool>,
    pool: Arc<AuthorizedPool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker_id = Uuid::new_v4();
        let poll_interval = Duration::from_millis(env_u64("ASSIGNMENT_JOB_POLL_MS", 1_000));
        let heartbeat_interval =
            Duration::from_secs(env_u64("ASSIGNMENT_JOB_HEARTBEAT_SECONDS", 20).max(5));
        let stale_after_seconds = env_i64("ASSIGNMENT_JOB_STALE_SECONDS", 120).max(60);
        let recovery_interval =
            Duration::from_secs(env_u64("ASSIGNMENT_JOB_RECOVERY_INTERVAL_SECONDS", 60).max(30));
        let max_attempts = env_i32("ASSIGNMENT_JOB_MAX_ATTEMPTS", 5).clamp(1, 10);
        let mut last_recovery = Instant::now() - recovery_interval;

        loop {
            if last_recovery.elapsed() >= recovery_interval {
                match run_authorized(
                    &raw_pool,
                    AuthorizedActor::system_queue(worker_id),
                    recover_stale_jobs(&pool, stale_after_seconds, max_attempts),
                )
                .await
                {
                    Ok(Ok(reconciled)) if reconciled > 0 => tracing::warn!(
                        reconciled,
                        "Reconciled stale assignment personalization jobs"
                    ),
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => tracing::error!(
                        error_code = repository_error_code(&error),
                        "Assignment personalization stale-job recovery failed"
                    ),
                    Err(_) => tracing::error!(
                        "Unable to open assignment personalization recovery transaction"
                    ),
                }
                last_recovery = Instant::now();
            }

            let claimed = run_authorized(
                &raw_pool,
                AuthorizedActor::system_queue(worker_id),
                claim_next_job(&pool, worker_id),
            )
            .await;

            let job = match claimed {
                Ok(Ok(Some(job))) => job,
                Ok(Ok(None)) => {
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }
                Ok(Err(error)) => {
                    tracing::error!(
                        error_code = repository_error_code(&error),
                        "Assignment personalization queue claim failed"
                    );
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }
                Err(_) => {
                    tracing::error!("Unable to open assignment personalization queue transaction");
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }
            };

            tracing::info!(
                job_id = %job.id,
                assignment_id = %job.assignment_id,
                school_id = %job.school_id,
                attempt = job.attempt_count,
                profile = %job.profile_name,
                profile_version = job.profile_version,
                model = %job.model_name,
                "Processing durable assignment personalization job"
            );

            let heartbeat_repository =
                AssignmentPersonalizationJobRepository::new(Arc::clone(&pool));
            let heartbeat_raw_pool = Arc::clone(&raw_pool);
            let heartbeat_job = job.clone();
            let (heartbeat_stop, mut heartbeat_stop_rx) = tokio::sync::watch::channel(false);
            let heartbeat_task = tokio::spawn(async move {
                let mut ticker = tokio::time::interval(heartbeat_interval);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            let actor = AuthorizedActor::new(
                                heartbeat_job.requested_by,
                                "Teacher",
                                Some(heartbeat_job.school_id),
                            );
                            match actor {
                                Ok(actor) => match run_authorized(
                                    &heartbeat_raw_pool,
                                    actor,
                                    heartbeat_repository.heartbeat(
                                        heartbeat_job.id,
                                        heartbeat_job.lease_owner,
                                    ),
                                ).await {
                                    Ok(Ok(())) => {}
                                    _ => tracing::warn!(
                                        job_id = %heartbeat_job.id,
                                        "Unable to refresh assignment personalization lease"
                                    ),
                                },
                                Err(_) => tracing::warn!(
                                    job_id = %heartbeat_job.id,
                                    "Unable to build assignment personalization heartbeat actor"
                                ),
                            }
                        }
                        changed = heartbeat_stop_rx.changed() => {
                            if changed.is_err() || *heartbeat_stop_rx.borrow() {
                                break;
                            }
                        }
                    }
                }
            });

            let actor = AuthorizedActor::new(job.requested_by, "Teacher", Some(job.school_id));
            let processed = match actor {
                Ok(actor) => {
                    run_authorized(
                        &raw_pool,
                        actor,
                        process_claimed_job(Arc::clone(&pool), &job),
                    )
                    .await
                }
                Err(error) => Err(error),
            };

            let _ = heartbeat_stop.send(true);
            let _ = heartbeat_task.await;

            match processed {
                Ok(Ok(())) => tracing::info!(
                    job_id = %job.id,
                    assignment_id = %job.assignment_id,
                    "Assignment personalization job completed"
                ),
                Ok(Err(error)) => {
                    handle_processing_failure(
                        &raw_pool,
                        Arc::clone(&pool),
                        &job,
                        error,
                        max_attempts,
                    )
                    .await;
                }
                Err(_) => {
                    handle_processing_failure(
                        &raw_pool,
                        Arc::clone(&pool),
                        &job,
                        WorkerProcessError::Repository(RepositoryError::Validation(
                            "authorized transaction unavailable".into(),
                        )),
                        max_attempts,
                    )
                    .await;
                }
            }
        }
    })
}

async fn process_claimed_job(
    pool: Arc<AuthorizedPool>,
    job: &ClaimedAssignmentPersonalizationJob,
) -> Result<(), WorkerProcessError> {
    let repository = AssignmentPersonalizationJobRepository::new(Arc::clone(&pool));
    repository.authorize_claimed_job(job).await?;

    let service = AssignmentPersonalizationService::new(Arc::clone(&pool))?;
    service
        .personalize_for_student(
            AssignmentId::from(job.assignment_id),
            StudentId::from(job.student_id),
            None,
        )
        .await?;

    // Re-check after the provider call and after the generated content has been
    // staged in this transaction. If authorization changed while AI was working,
    // returning an error rolls back the generated content as well.
    repository.authorize_claimed_job(job).await?;
    repository.complete(job.id, job.lease_owner).await?;
    Ok(())
}

async fn handle_processing_failure(
    raw_pool: &PgPool,
    pool: Arc<AuthorizedPool>,
    job: &ClaimedAssignmentPersonalizationJob,
    error: WorkerProcessError,
    max_attempts: i32,
) {
    let action = classify_failure(&error);
    let actor = match AuthorizedActor::new(job.requested_by, "Teacher", Some(job.school_id)) {
        Ok(actor) => actor,
        Err(_) => {
            tracing::error!(
                job_id = %job.id,
                "Unable to build assignment personalization failure actor"
            );
            return;
        }
    };
    let repository = AssignmentPersonalizationJobRepository::new(pool);

    match action {
        FailureAction::CancelAuthorization => {
            match run_authorized(
                raw_pool,
                actor,
                repository.cancel_claimed_job(job.id, job.lease_owner),
            )
            .await
            {
                Ok(Ok(())) => tracing::warn!(
                    job_id = %job.id,
                    "Cancelled assignment personalization after authorization changed"
                ),
                _ => tracing::error!(
                    job_id = %job.id,
                    "Unable to persist assignment personalization cancellation"
                ),
            }
        }
        FailureAction::Record {
            kind,
            retry_after_seconds,
        } => {
            match run_authorized(
                raw_pool,
                actor,
                repository.record_failure(job, kind, retry_after_seconds, max_attempts),
            )
            .await
            {
                Ok(Ok(PersonalizationFailureDisposition::Requeued)) => tracing::warn!(
                    job_id = %job.id,
                    attempt = job.attempt_count,
                    error_code = kind.code(),
                    "Assignment personalization job requeued"
                ),
                Ok(Ok(PersonalizationFailureDisposition::FailedPermanently)) => tracing::error!(
                    job_id = %job.id,
                    attempts = job.attempt_count,
                    error_code = kind.code(),
                    "Assignment personalization job reached terminal policy"
                ),
                Ok(Ok(PersonalizationFailureDisposition::IgnoredInactive)) => tracing::info!(
                    job_id = %job.id,
                    "Ignoring assignment personalization failure because the lease is inactive"
                ),
                _ => tracing::error!(
                    job_id = %job.id,
                    error_code = kind.code(),
                    "Unable to persist assignment personalization retry state"
                ),
            }
        }
    }
}

fn classify_failure(error: &WorkerProcessError) -> FailureAction {
    match error {
        WorkerProcessError::Repository(
            RepositoryError::Unauthorized | RepositoryError::NotFound { .. },
        ) => FailureAction::CancelAuthorization,
        WorkerProcessError::Repository(_) => FailureAction::Record {
            kind: PersonalizationFailureKind::ProcessingUnavailable,
            retry_after_seconds: 2,
        },
        WorkerProcessError::Personalization(
            PersonalizationError::AssignmentNotFound(_)
            | PersonalizationError::StudentNotFound(_)
            | PersonalizationError::CustomAssignmentNotFound(_),
        ) => FailureAction::CancelAuthorization,
        WorkerProcessError::Personalization(PersonalizationError::LlmError(llm_error)) => {
            classify_llm_failure(llm_error)
        }
        WorkerProcessError::Personalization(
            PersonalizationError::StudentContextError(_) | PersonalizationError::DatabaseError(_),
        ) => FailureAction::Record {
            kind: PersonalizationFailureKind::ProcessingUnavailable,
            retry_after_seconds: 2,
        },
    }
}

fn classify_llm_failure(error: &LlmError) -> FailureAction {
    match error {
        LlmError::RateLimited {
            retry_after_seconds,
        } => FailureAction::Record {
            kind: PersonalizationFailureKind::RateLimited,
            retry_after_seconds: *retry_after_seconds,
        },
        LlmError::MissingApiKey | LlmError::RequestFailed(_) | LlmError::TemporarilyUnavailable => {
            FailureAction::Record {
                kind: PersonalizationFailureKind::GatewayUnavailable,
                retry_after_seconds: 10,
            }
        }
        LlmError::ParseError(_) | LlmError::InvalidResponse(_) | LlmError::ApiError { .. } => {
            FailureAction::Record {
                kind: PersonalizationFailureKind::InvalidGatewayResponse,
                retry_after_seconds: 5,
            }
        }
        LlmError::SecretInPrompt | LlmError::PromptTooLarge => FailureAction::Record {
            kind: PersonalizationFailureKind::ContentRejected,
            retry_after_seconds: 0,
        },
        LlmError::MissingSchoolId => FailureAction::Record {
            kind: PersonalizationFailureKind::ProcessingUnavailable,
            retry_after_seconds: 5,
        },
    }
}

async fn claim_next_job(
    pool: &AuthorizedPool,
    worker_id: Uuid,
) -> Result<Option<ClaimedAssignmentPersonalizationJob>, RepositoryError> {
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
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(ClaimedAssignmentPersonalizationJob {
            id: row.try_get("job_id")?,
            school_id: row.try_get("school_id")?,
            assignment_id: row.try_get("assignment_id")?,
            student_id: row.try_get("student_id")?,
            requested_by: row.try_get("requested_by")?,
            attempt_count: row.try_get("attempt_count")?,
            model_name: row.try_get("model_name")?,
            profile_name: row.try_get("profile_name")?,
            profile_version: row.try_get("profile_version")?,
            lease_owner: row.try_get("lease_owner")?,
        })
    })
    .transpose()
}

async fn recover_stale_jobs(
    pool: &AuthorizedPool,
    stale_after_seconds: i64,
    max_attempts: i32,
) -> Result<u64, RepositoryError> {
    let reconciled = sqlx::query_scalar::<_, i64>(
        "SELECT public.recover_stale_assignment_personalization_jobs($1, $2)",
    )
    .bind(stale_after_seconds.max(60))
    .bind(max_attempts.clamp(1, 10))
    .fetch_one(pool)
    .await?;
    Ok(reconciled.max(0) as u64)
}

async fn run_authorized<T, E, F>(
    raw_pool: &PgPool,
    actor: AuthorizedActor,
    operation: F,
) -> Result<Result<T, E>, RlsContextError>
where
    F: Future<Output = Result<T, E>>,
{
    let transaction = AuthorizedTx::begin(raw_pool, actor).await?;
    transaction.scope(operation, Result::is_ok).await
}

fn repository_error_code(error: &RepositoryError) -> &'static str {
    match error {
        RepositoryError::Database(_) => "database_error",
        RepositoryError::NotFound { .. } => "not_found",
        RepositoryError::Duplicate { .. } => "duplicate",
        RepositoryError::Validation(_) => "validation_error",
        RepositoryError::Unauthorized => "unauthorized",
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_i64(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_i32(name: &str, default: i32) -> i32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::llm_service::{
        BaseAssignment, ExternalLlmClient, LlmConfig, PerformanceMetrics, StudentContext,
    };
    use std::ffi::OsString;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    static MOCK_GATEWAY_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[derive(Clone, Copy)]
    enum MockGatewayFault {
        Timeout,
        RateLimited,
        InvalidJson,
        Outage,
    }

    fn spawn_mock_gateway(fault: MockGatewayFault) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local mock AI gateway");
        let address = listener.local_addr().expect("mock gateway address");

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept mock AI gateway request");
            let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
            let mut request = [0_u8; 16_384];
            let _ = stream.read(&mut request);

            let (status, content_type, body) = match fault {
                MockGatewayFault::Timeout => {
                    thread::sleep(Duration::from_millis(250));
                    (
                        "200 OK",
                        "application/json",
                        r#"{"model":"deepseek-chat","choices":[{"index":0,"message":{"role":"assistant","content":"{}"},"finish_reason":"stop"}]}"#,
                    )
                }
                MockGatewayFault::RateLimited => (
                    "429 Too Many Requests",
                    "application/json",
                    r#"{"error":{"code":"provider_rate_limited","message":"rate limited","retry_after_seconds":19}}"#,
                ),
                MockGatewayFault::InvalidJson => (
                    "200 OK",
                    "application/json",
                    "not-json-provider-secret-sentinel",
                ),
                MockGatewayFault::Outage => (
                    "503 Service Unavailable",
                    "application/json",
                    r#"{"error":{"code":"ai_temporarily_unavailable","message":"offline","retry_after_seconds":30}}"#,
                ),
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });

        format!("http://{address}")
    }

    fn restore_env(name: &str, previous: Option<OsString>) {
        match previous {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }

    fn client_through_local_mock_proxy(
        proxy_origin: &str,
        request_timeout: Duration,
    ) -> ExternalLlmClient {
        let names = ["HTTP_PROXY", "http_proxy", "NO_PROXY", "no_proxy"];
        let previous = names.map(|name| std::env::var_os(name));
        std::env::set_var("HTTP_PROXY", proxy_origin);
        std::env::set_var("http_proxy", proxy_origin);
        std::env::set_var("NO_PROXY", "");
        std::env::set_var("no_proxy", "");

        let client = ExternalLlmClient::with_config(LlmConfig {
            api_key: "abcdefghijklmnopqrstuvwxyz123456".to_string(),
            base_url: "http://ai-gateway:8090".to_string(),
            model: "deepseek-chat".to_string(),
            max_tokens: 1_024,
            temperature: 0.2,
            request_timeout,
            default_school_id: None,
            max_prompt_chars: 20_000,
        })
        .expect("construct fixed-origin LLM client through local mock proxy");

        for (name, value) in names.into_iter().zip(previous) {
            restore_env(name, value);
        }
        client
    }

    fn base_assignment() -> BaseAssignment {
        BaseAssignment {
            title: "Mock gateway assignment".to_string(),
            body: "Explain the concept".to_string(),
            subject: "Science".to_string(),
            due_date: "2030-01-01".to_string(),
            lecture_title: None,
            lecture_number: None,
        }
    }

    fn student_context() -> StudentContext {
        StudentContext {
            school_id: Uuid::new_v4(),
            student_id: "internal-test-student".to_string(),
            student_name: "Internal Test Student".to_string(),
            talent_profile: None,
            teacher_reports: Vec::new(),
            previous_performance: PerformanceMetrics::default(),
        }
    }

    async fn mock_gateway_error(fault: MockGatewayFault, request_timeout: Duration) -> LlmError {
        let proxy_origin = spawn_mock_gateway(fault);
        let client = client_through_local_mock_proxy(&proxy_origin, request_timeout);
        client
            .personalize_assignment(&base_assignment(), &student_context())
            .await
            .expect_err("mock gateway fault must fail personalization")
    }

    #[test]
    fn gateway_failures_map_to_bounded_safe_retry_classes() {
        assert_eq!(
            classify_llm_failure(&LlmError::RateLimited {
                retry_after_seconds: 17,
            }),
            FailureAction::Record {
                kind: PersonalizationFailureKind::RateLimited,
                retry_after_seconds: 17,
            }
        );
        assert_eq!(
            classify_llm_failure(&LlmError::TemporarilyUnavailable),
            FailureAction::Record {
                kind: PersonalizationFailureKind::GatewayUnavailable,
                retry_after_seconds: 10,
            }
        );
        assert_eq!(
            classify_llm_failure(&LlmError::ParseError("raw provider payload".into())),
            FailureAction::Record {
                kind: PersonalizationFailureKind::InvalidGatewayResponse,
                retry_after_seconds: 5,
            }
        );
        assert_eq!(
            classify_llm_failure(&LlmError::InvalidResponse("wrong schema".into())),
            FailureAction::Record {
                kind: PersonalizationFailureKind::InvalidGatewayResponse,
                retry_after_seconds: 5,
            }
        );
        assert_eq!(
            classify_llm_failure(&LlmError::PromptTooLarge),
            FailureAction::Record {
                kind: PersonalizationFailureKind::ContentRejected,
                retry_after_seconds: 0,
            }
        );
    }

    #[tokio::test]
    async fn local_mock_gateway_faults_drive_worker_retry_policy() {
        let _env_guard = MOCK_GATEWAY_ENV_LOCK.lock().await;

        let timeout =
            mock_gateway_error(MockGatewayFault::Timeout, Duration::from_millis(40)).await;
        assert!(matches!(timeout, LlmError::RequestFailed(_)));
        assert_eq!(
            classify_llm_failure(&timeout),
            FailureAction::Record {
                kind: PersonalizationFailureKind::GatewayUnavailable,
                retry_after_seconds: 10,
            }
        );

        let rate_limited =
            mock_gateway_error(MockGatewayFault::RateLimited, Duration::from_secs(1)).await;
        assert!(matches!(
            rate_limited,
            LlmError::RateLimited {
                retry_after_seconds: 19
            }
        ));
        assert_eq!(
            classify_llm_failure(&rate_limited),
            FailureAction::Record {
                kind: PersonalizationFailureKind::RateLimited,
                retry_after_seconds: 19,
            }
        );

        let invalid =
            mock_gateway_error(MockGatewayFault::InvalidJson, Duration::from_secs(1)).await;
        assert!(matches!(invalid, LlmError::ParseError(_)));
        assert!(!format!("{invalid}").contains("provider-secret-sentinel"));
        assert_eq!(
            classify_llm_failure(&invalid),
            FailureAction::Record {
                kind: PersonalizationFailureKind::InvalidGatewayResponse,
                retry_after_seconds: 5,
            }
        );

        let outage = mock_gateway_error(MockGatewayFault::Outage, Duration::from_secs(1)).await;
        assert!(matches!(outage, LlmError::TemporarilyUnavailable));
        assert_eq!(
            classify_llm_failure(&outage),
            FailureAction::Record {
                kind: PersonalizationFailureKind::GatewayUnavailable,
                retry_after_seconds: 10,
            }
        );
    }

    #[test]
    fn environment_helpers_fail_to_safe_defaults() {
        let key = "ASSIGNMENT_WORKER_TEST_INVALID";
        std::env::set_var(key, "not-a-number");
        assert_eq!(env_u64(key, 11), 11);
        assert_eq!(env_i64(key, 12), 12);
        assert_eq!(env_i32(key, 13), 13);
        std::env::remove_var(key);
    }
}
