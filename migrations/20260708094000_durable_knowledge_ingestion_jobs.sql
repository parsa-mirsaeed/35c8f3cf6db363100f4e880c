-- Durable queue metadata for governed knowledge ingestion.
ALTER TABLE ingestion_jobs
    ADD COLUMN IF NOT EXISTS requested_by UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN IF NOT EXISTS locked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS heartbeat_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_ingestion_jobs_claimable
    ON ingestion_jobs (available_at, created_at)
    WHERE stage = 'embed' AND status = 'queued';

-- Existing development databases may contain more than one active embedding job
-- from the former synchronous implementation. Keep the newest active job and
-- cancel older duplicates before enforcing the queue invariant.
WITH ranked_active_jobs AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY asset_id
               ORDER BY (status = 'running') DESC, created_at DESC, id DESC
           ) AS position
    FROM ingestion_jobs
    WHERE stage = 'embed'
      AND status IN ('queued', 'running')
)
UPDATE ingestion_jobs job
SET status = 'cancelled',
    finished_at = COALESCE(job.finished_at, NOW()),
    error_message = COALESCE(job.error_message, 'Superseded by durable queue migration'),
    locked_at = NULL,
    heartbeat_at = NULL
FROM ranked_active_jobs ranked
WHERE job.id = ranked.id
  AND ranked.position > 1;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ingestion_jobs_one_active_embed
    ON ingestion_jobs (asset_id)
    WHERE stage = 'embed' AND status IN ('queued', 'running');

COMMENT ON COLUMN ingestion_jobs.requested_by IS
    'Platform administrator who requested the governed ingestion job.';
COMMENT ON COLUMN ingestion_jobs.available_at IS
    'Earliest time a worker may claim the job; used for retry backoff.';
COMMENT ON COLUMN ingestion_jobs.locked_at IS
    'Time the current worker claimed the job.';
COMMENT ON COLUMN ingestion_jobs.heartbeat_at IS
    'Most recent worker heartbeat for stale-job recovery.';
