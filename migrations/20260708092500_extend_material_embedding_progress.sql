-- Bring the legacy material-vectorization tracking schema in line with the
-- progress and cancellation fields used by the existing teacher workflow.
-- Columns remain nullable for compatibility with rows created before this
-- migration and with the Option-based API models.
ALTER TABLE material_embeddings
    ADD COLUMN IF NOT EXISTS current_batch INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_batches INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS cancelled BOOLEAN DEFAULT FALSE;

ALTER TABLE material_embeddings
    DROP CONSTRAINT IF EXISTS material_embeddings_current_batch_nonnegative,
    ADD CONSTRAINT material_embeddings_current_batch_nonnegative
        CHECK (current_batch IS NULL OR current_batch >= 0),
    DROP CONSTRAINT IF EXISTS material_embeddings_total_batches_nonnegative,
    ADD CONSTRAINT material_embeddings_total_batches_nonnegative
        CHECK (total_batches IS NULL OR total_batches >= 0),
    DROP CONSTRAINT IF EXISTS material_embeddings_batch_progress_consistent,
    ADD CONSTRAINT material_embeddings_batch_progress_consistent
        CHECK (
            current_batch IS NULL
            OR total_batches IS NULL
            OR current_batch <= total_batches
        );

CREATE INDEX IF NOT EXISTS idx_material_embeddings_active_progress
    ON material_embeddings (status, cancelled)
    WHERE status IN ('pending', 'processing');

COMMENT ON COLUMN material_embeddings.current_batch IS
    'Count of embedding batches completed for progress reporting.';
COMMENT ON COLUMN material_embeddings.total_batches IS
    'Total embedding batches planned for the material.';
COMMENT ON COLUMN material_embeddings.cancelled IS
    'Cancellation flag checked by background vectorization workers.';
