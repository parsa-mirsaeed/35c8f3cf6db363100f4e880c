use crate::repositories::{EmbeddingFailureDisposition, KnowledgeIngestionJobRepository};
use crate::services::ai_outage_queue::requeue_provider_outage;
use crate::services::embedding_service::EmbeddingError;
use crate::services::{KnowledgeAssetError, KnowledgeAssetService};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

/// Start the governed knowledge ingestion worker.
///
/// Jobs are claimed with PostgreSQL `FOR UPDATE SKIP LOCKED`, so this worker is
/// safe to run in multiple application instances. Queue state survives process
/// restarts, and stale running jobs are recovered at startup. External provider
/// outages are requeued independently from the permanent content-failure budget.
pub fn start_knowledge_ingestion_worker(pool: Arc<PgPool>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let repository = KnowledgeIngestionJobRepository::new(Arc::clone(&pool));
        let poll_interval = Duration::from_millis(env_u64("KNOWLEDGE_JOB_POLL_MS", 1_500));
        let heartbeat_interval =
            Duration::from_secs(env_u64("KNOWLEDGE_JOB_HEARTBEAT_SECONDS", 30).max(5));
        let stale_after_seconds = env_i64("KNOWLEDGE_JOB_STALE_SECONDS", 3_600).max(60);
        let max_attempts = env_i32("KNOWLEDGE_JOB_MAX_ATTEMPTS", 3).clamp(1, 10);

        match repository
            .recover_stale_embedding_jobs(stale_after_seconds)
            .await
        {
            Ok(recovered) if recovered > 0 => {
                tracing::warn!(recovered, "Recovered stale knowledge ingestion jobs");
            }
            Ok(_) => {}
            Err(error) => {
                tracing::error!(%error, "Unable to recover stale knowledge ingestion jobs");
            }
        }

        loop {
            match repository.claim_next_embedding().await {
                Ok(Some(job)) => {
                    tracing::info!(
                        job_id = %job.id,
                        asset_id = %job.asset_id,
                        attempt = job.attempts,
                        "Processing governed knowledge embedding job"
                    );

                    let heartbeat_repository = repository.clone();
                    let heartbeat_job_id = job.id;
                    let (heartbeat_stop, mut heartbeat_stop_rx) =
                        tokio::sync::watch::channel(false);
                    let heartbeat_task = tokio::spawn(async move {
                        let mut ticker = tokio::time::interval(heartbeat_interval);
                        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                        loop {
                            tokio::select! {
                                _ = ticker.tick() => {
                                    if let Err(error) = heartbeat_repository.heartbeat(heartbeat_job_id).await {
                                        tracing::warn!(
                                            job_id = %heartbeat_job_id,
                                            error = %error,
                                            "Unable to update knowledge ingestion heartbeat"
                                        );
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

                    let result = match KnowledgeAssetService::new(Arc::clone(&pool)).await {
                        Ok(service) => {
                            service
                                .process_embedding_job(job.id, job.asset_id, job.requested_by)
                                .await
                        }
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
                            if let Some((outage_code, retry_after_seconds)) =
                                provider_outage(&error)
                            {
                                match requeue_provider_outage(
                                    Arc::clone(&pool),
                                    &job,
                                    outage_code,
                                    retry_after_seconds,
                                )
                                .await
                                {
                                    Ok(EmbeddingFailureDisposition::Requeued) => tracing::warn!(
                                        job_id = %job.id,
                                        asset_id = %job.asset_id,
                                        attempt = job.attempts,
                                        outage_code,
                                        "Knowledge ingestion job remains queued during AI outage"
                                    ),
                                    Ok(EmbeddingFailureDisposition::IgnoredInactive) => {
                                        tracing::info!(
                                            job_id = %job.id,
                                            asset_id = %job.asset_id,
                                            "Ignoring AI outage because the job is no longer active"
                                        )
                                    }
                                    Ok(EmbeddingFailureDisposition::FailedPermanently) => {
                                        tracing::error!(
                                            job_id = %job.id,
                                            "Provider outage handler returned an impossible permanent failure"
                                        );
                                    }
                                    Err(repository_error) => tracing::error!(
                                        job_id = %job.id,
                                        error = %repository_error,
                                        outage_code,
                                        "Unable to persist AI outage retry"
                                    ),
                                }
                                continue;
                            }

                            let message = error.to_string();
                            match repository
                                .record_embedding_failure(&job, &message, max_attempts)
                                .await
                            {
                                Ok(EmbeddingFailureDisposition::Requeued) => tracing::warn!(
                                    job_id = %job.id,
                                    asset_id = %job.asset_id,
                                    attempt = job.attempts,
                                    error = %message,
                                    "Knowledge ingestion job requeued after transient processing failure"
                                ),
                                Ok(EmbeddingFailureDisposition::FailedPermanently) => {
                                    tracing::error!(
                                        job_id = %job.id,
                                        asset_id = %job.asset_id,
                                        attempts = job.attempts,
                                        error = %message,
                                        "Knowledge ingestion job failed permanently"
                                    )
                                }
                                Ok(EmbeddingFailureDisposition::IgnoredInactive) => tracing::info!(
                                    job_id = %job.id,
                                    asset_id = %job.asset_id,
                                    "Ignoring worker failure because the job is no longer active"
                                ),
                                Err(repository_error) => tracing::error!(
                                    job_id = %job.id,
                                    error = %repository_error,
                                    original_error = %message,
                                    "Unable to persist knowledge ingestion failure"
                                ),
                            }
                        }
                    }
                }
                Ok(None) => tokio::time::sleep(poll_interval).await,
                Err(error) => {
                    tracing::error!(%error, "Knowledge ingestion queue claim failed");
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    })
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
