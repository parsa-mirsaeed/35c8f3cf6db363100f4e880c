use crate::repositories::{
    ClaimedKnowledgeIngestionJob, EmbeddingFailureDisposition, KnowledgeIngestionJobRepository,
    RepositoryError,
};
use crate::rls_context::{AuthorizedActor, AuthorizedPool, AuthorizedTx, RlsContextError};
use crate::services::ai_outage_queue::requeue_provider_outage;
use crate::services::embedding_service::EmbeddingError;
use crate::services::{KnowledgeAssetError, KnowledgeAssetService};
use sqlx::{PgPool, Row};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct ScopedKnowledgeIngestionJob {
    job: ClaimedKnowledgeIngestionJob,
    school_id: Uuid,
}

/// Start the governed knowledge ingestion worker.
///
/// Global queue discovery runs only through two bounded `SECURITY DEFINER`
/// functions under an explicit no-school scheduler context. Once a job is
/// claimed, every protected query runs in a separate school-scoped system-job
/// transaction. The long-running database role never bypasses RLS.
pub fn start_knowledge_ingestion_worker(
    raw_pool: Arc<PgPool>,
    pool: Arc<AuthorizedPool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let repository = KnowledgeIngestionJobRepository::new(Arc::clone(&pool));
        let worker_id = Uuid::new_v4();
        let poll_interval = Duration::from_millis(env_u64("KNOWLEDGE_JOB_POLL_MS", 1_500));
        let heartbeat_interval =
            Duration::from_secs(env_u64("KNOWLEDGE_JOB_HEARTBEAT_SECONDS", 30).max(5));
        let stale_after_seconds = env_i64("KNOWLEDGE_JOB_STALE_SECONDS", 3_600).max(60);
        let max_attempts = env_i32("KNOWLEDGE_JOB_MAX_ATTEMPTS", 3).clamp(1, 10);

        match run_authorized(
            &raw_pool,
            AuthorizedActor::system_queue(worker_id),
            recover_stale_embedding_jobs(&pool, stale_after_seconds),
        )
        .await
        {
            Ok(Ok(recovered)) if recovered > 0 => {
                tracing::warn!(recovered, "Recovered stale knowledge ingestion jobs");
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                tracing::error!(%error, "Unable to recover stale knowledge ingestion jobs");
            }
            Err(error) => {
                tracing::error!(%error, "Unable to open queue recovery authorization transaction");
            }
        }

        loop {
            let claimed = run_authorized(
                &raw_pool,
                AuthorizedActor::system_queue(worker_id),
                claim_next_embedding(&pool),
            )
            .await;

            let scoped_job = match claimed {
                Ok(Ok(Some(job))) => job,
                Ok(Ok(None)) => {
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }
                Ok(Err(error)) => {
                    tracing::error!(%error, "Knowledge ingestion queue claim failed");
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }
                Err(error) => {
                    tracing::error!(%error, "Unable to open queue claim authorization transaction");
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }
            };
            let job = scoped_job.job;
            let school_id = scoped_job.school_id;

            tracing::info!(
                job_id = %job.id,
                asset_id = %job.asset_id,
                school_id = %school_id,
                attempt = job.attempts,
                "Processing governed knowledge embedding job"
            );

            let heartbeat_repository = repository.clone();
            let heartbeat_raw_pool = Arc::clone(&raw_pool);
            let heartbeat_job = job.clone();
            let (heartbeat_stop, mut heartbeat_stop_rx) = tokio::sync::watch::channel(false);
            let heartbeat_task = tokio::spawn(async move {
                let mut ticker = tokio::time::interval(heartbeat_interval);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            match run_authorized(
                                &heartbeat_raw_pool,
                                AuthorizedActor::system_job(
                                    heartbeat_job.requested_by,
                                    school_id,
                                ),
                                heartbeat_repository.heartbeat(heartbeat_job.id),
                            ).await {
                                Ok(Ok(())) => {}
                                Ok(Err(error)) => tracing::warn!(
                                    job_id = %heartbeat_job.id,
                                    error = %error,
                                    "Unable to update knowledge ingestion heartbeat"
                                ),
                                Err(error) => tracing::warn!(
                                    job_id = %heartbeat_job.id,
                                    error = %error,
                                    "Unable to open heartbeat authorization transaction"
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

            let service = KnowledgeAssetService::new(Arc::clone(&pool)).await;
            let result = match service {
                Ok(service) => match run_authorized(
                    &raw_pool,
                    AuthorizedActor::system_job(job.requested_by, school_id),
                    service.process_embedding_job(job.id, job.asset_id, job.requested_by),
                )
                .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::error!(
                            job_id = %job.id,
                            error = %error,
                            "Unable to open embedding authorization transaction"
                        );
                        let _ = heartbeat_stop.send(true);
                        let _ = heartbeat_task.await;
                        continue;
                    }
                },
                Err(error) => Err(error),
            };

            let _ = heartbeat_stop.send(true);
            let _ = heartbeat_task.await;

            match result {
                Ok(chunk_count) => {
                    tracing::info!(
                        job_id = %job.id,
                        asset_id = %job.asset_id,
                        chunk_count,
                        "Governed knowledge embedding job completed"
                    );
                }
                Err(error) => {
                    if let Some((outage_code, retry_after_seconds)) = provider_outage(&error) {
                        match run_authorized(
                            &raw_pool,
                            AuthorizedActor::system_job(job.requested_by, school_id),
                            requeue_provider_outage(
                                Arc::clone(&pool),
                                &job,
                                outage_code,
                                retry_after_seconds,
                            ),
                        )
                        .await
                        {
                            Ok(Ok(EmbeddingFailureDisposition::Requeued)) => tracing::warn!(
                                job_id = %job.id,
                                asset_id = %job.asset_id,
                                attempt = job.attempts,
                                outage_code,
                                "Knowledge ingestion job remains queued during AI outage"
                            ),
                            Ok(Ok(EmbeddingFailureDisposition::IgnoredInactive)) => tracing::info!(
                                job_id = %job.id,
                                asset_id = %job.asset_id,
                                "Ignoring AI outage because the job is no longer active"
                            ),
                            Ok(Ok(EmbeddingFailureDisposition::FailedPermanently)) => {
                                tracing::error!(
                                    job_id = %job.id,
                                    "Provider outage handler returned an impossible permanent failure"
                                );
                            }
                            Ok(Err(repository_error)) => tracing::error!(
                                job_id = %job.id,
                                error = %repository_error,
                                outage_code,
                                "Unable to persist AI outage retry"
                            ),
                            Err(transaction_error) => tracing::error!(
                                job_id = %job.id,
                                error = %transaction_error,
                                outage_code,
                                "Unable to open AI outage retry transaction"
                            ),
                        }
                        continue;
                    }

                    let message = error.to_string();
                    match run_authorized(
                        &raw_pool,
                        AuthorizedActor::system_job(job.requested_by, school_id),
                        repository.record_embedding_failure(&job, &message, max_attempts),
                    )
                    .await
                    {
                        Ok(Ok(EmbeddingFailureDisposition::Requeued)) => tracing::warn!(
                            job_id = %job.id,
                            asset_id = %job.asset_id,
                            attempt = job.attempts,
                            error = %message,
                            "Knowledge ingestion job requeued after transient processing failure"
                        ),
                        Ok(Ok(EmbeddingFailureDisposition::FailedPermanently)) => tracing::error!(
                            job_id = %job.id,
                            asset_id = %job.asset_id,
                            attempts = job.attempts,
                            error = %message,
                            "Knowledge ingestion job failed permanently"
                        ),
                        Ok(Ok(EmbeddingFailureDisposition::IgnoredInactive)) => tracing::info!(
                            job_id = %job.id,
                            asset_id = %job.asset_id,
                            "Ignoring worker failure because the job is no longer active"
                        ),
                        Ok(Err(repository_error)) => tracing::error!(
                            job_id = %job.id,
                            error = %repository_error,
                            original_error = %message,
                            "Unable to persist knowledge ingestion failure"
                        ),
                        Err(transaction_error) => tracing::error!(
                            job_id = %job.id,
                            error = %transaction_error,
                            original_error = %message,
                            "Unable to open embedding failure transaction"
                        ),
                    }
                }
            }
        }
    })
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

async fn claim_next_embedding(
    pool: &AuthorizedPool,
) -> Result<Option<ScopedKnowledgeIngestionJob>, RepositoryError> {
    let row = sqlx::query(
        r#"
        SELECT job_id, asset_id, requested_by, school_id, attempts
        FROM public.claim_next_embedding_job()
        "#,
    )
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(ScopedKnowledgeIngestionJob {
            job: ClaimedKnowledgeIngestionJob {
                id: row.try_get("job_id")?,
                asset_id: row.try_get("asset_id")?,
                requested_by: row.try_get("requested_by")?,
                attempts: row.try_get("attempts")?,
            },
            school_id: row.try_get("school_id")?,
        })
    })
    .transpose()
}

async fn recover_stale_embedding_jobs(
    pool: &AuthorizedPool,
    stale_after_seconds: i64,
) -> Result<u64, RepositoryError> {
    let recovered = sqlx::query_scalar::<_, i64>("SELECT public.recover_stale_embedding_jobs($1)")
        .bind(stale_after_seconds.max(60))
        .fetch_one(pool)
        .await?;
    Ok(recovered.max(0) as u64)
}

fn provider_outage(error: &KnowledgeAssetError) -> Option<(&'static str, u64)> {
    match error {
        KnowledgeAssetError::Embedding(EmbeddingError::RateLimited {
            retry_after_seconds,
        }) => Some(("provider_rate_limited", *retry_after_seconds)),
        KnowledgeAssetError::Embedding(EmbeddingError::TemporarilyUnavailable) => {
            Some(("provider_circuit_open", 30))
        }
        KnowledgeAssetError::Embedding(EmbeddingError::RequestFailed(source))
            if source.is_timeout() || source.is_connect() =>
        {
            Some(("gateway_unreachable", 10))
        }
        KnowledgeAssetError::Embedding(EmbeddingError::GatewayError { status, .. })
            if *status == 502 || *status == 503 || *status == 504 =>
        {
            Some(("provider_temporarily_unavailable", 30))
        }
        _ => None,
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

    #[test]
    fn environment_helpers_use_defaults_for_invalid_values() {
        let key = "KNOWLEDGE_WORKER_TEST_INVALID";
        std::env::set_var(key, "not-a-number");
        assert_eq!(env_u64(key, 42), 42);
        assert_eq!(env_i64(key, 43), 43);
        assert_eq!(env_i32(key, 44), 44);
        std::env::remove_var(key);
    }

    #[test]
    fn provider_failures_are_separate_from_content_failures() {
        let unavailable = KnowledgeAssetError::Embedding(EmbeddingError::TemporarilyUnavailable);
        assert_eq!(
            provider_outage(&unavailable),
            Some(("provider_circuit_open", 30))
        );

        let rate_limited = KnowledgeAssetError::Embedding(EmbeddingError::RateLimited {
            retry_after_seconds: 17,
        });
        assert_eq!(
            provider_outage(&rate_limited),
            Some(("provider_rate_limited", 17))
        );

        assert!(provider_outage(&KnowledgeAssetError::EmptyText).is_none());
    }
}
