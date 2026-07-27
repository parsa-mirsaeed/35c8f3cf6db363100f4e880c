//! Durable queue handling for external-AI outages.
//!
//! Provider availability is not a property of the document. A timeout, rate
//! limit, open circuit, or unreachable local gateway must therefore never mark a
//! governed asset permanently failed. These failures return the active job to the
//! authoritative PostgreSQL queue with capped backoff while preserving archive
//! and cancellation races.

use crate::repositories::{
    ClaimedKnowledgeIngestionJob, EmbeddingFailureDisposition, RepositoryError,
};
use sqlx::PgPool;
use std::sync::Arc;

pub async fn requeue_provider_outage(
    pool: Arc<PgPool>,
    job: &ClaimedKnowledgeIngestionJob,
    error_code: &str,
    requested_retry_after_seconds: u64,
) -> Result<EmbeddingFailureDisposition, RepositoryError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(job.asset_id)
        .execute(&mut *tx)
        .await?;

    let active = sqlx::query_scalar::<_, i32>(
        r#"
        SELECT 1
        FROM ingestion_jobs job
        JOIN knowledge_assets asset ON asset.id = job.asset_id
        WHERE job.id = $1
          AND job.asset_id = $2
          AND job.status = 'running'
          AND asset.status = 'embedding_pending'
        FOR UPDATE OF job, asset
        "#,
    )
    .bind(job.id)
    .bind(job.asset_id)
    .fetch_optional(&mut *tx)
    .await?
    .is_some();

    if !active {
        tx.commit().await?;
        return Ok(EmbeddingFailureDisposition::IgnoredInactive);
    }

    let backoff_seconds =
        provider_backoff_seconds(job.attempts, requested_retry_after_seconds);
    let updated = sqlx::query(
        r#"
        UPDATE ingestion_jobs
        SET status = 'queued',
            available_at = NOW() + make_interval(secs => $2),
            locked_at = NULL,
            heartbeat_at = NULL,
            error_message = $3
        WHERE id = $1
          AND status = 'running'
        "#,
    )
    .bind(job.id)
    .bind(backoff_seconds)
    .bind(sanitized_outage_code(error_code))
    .execute(&mut *tx)
    .await?;

    if updated.rows_affected() != 1 {
        tx.rollback().await?;
        return Ok(EmbeddingFailureDisposition::IgnoredInactive);
    }

    tx.commit().await?;
    Ok(EmbeddingFailureDisposition::Requeued)
}

fn provider_backoff_seconds(attempts: i32, requested_retry_after_seconds: u64) -> i64 {
    let exponential = 2u64.saturating_pow(attempts.clamp(1, 8) as u32);
    requested_retry_after_seconds
        .max(exponential)
        .clamp(5, 300) as i64
}

fn sanitized_outage_code(value: &str) -> &'static str {
    match value {
        "provider_rate_limited" => "provider_rate_limited",
        "provider_circuit_open" => "provider_circuit_open",
        "gateway_unreachable" => "gateway_unreachable",
        _ => "provider_temporarily_unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outage_backoff_is_bounded_and_honors_provider_delay() {
        assert_eq!(provider_backoff_seconds(1, 0), 5);
        assert_eq!(provider_backoff_seconds(3, 30), 30);
        assert_eq!(provider_backoff_seconds(50, 0), 256);
        assert_eq!(provider_backoff_seconds(2, 3_600), 300);
    }

    #[test]
    fn outage_errors_are_reduced_to_non_sensitive_codes() {
        assert_eq!(
            sanitized_outage_code("provider_rate_limited"),
            "provider_rate_limited"
        );
        assert_eq!(
            sanitized_outage_code("secret provider body"),
            "provider_temporarily_unavailable"
        );
    }
}
