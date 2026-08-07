-- PR-03: bind the durable knowledge worker to transaction-scoped RLS.
--
-- Global queue discovery is exposed only through two narrow SECURITY DEFINER
-- functions. They can claim/recover queue state, but cannot read governed
-- content. Every operation after claim uses the returned school_id in a normal
-- NOBYPASSRLS transaction and remains subject to the policies below.

CREATE OR REPLACE FUNCTION public.claim_next_embedding_job()
RETURNS TABLE (
    job_id UUID,
    asset_id UUID,
    requested_by UUID,
    school_id UUID,
    attempts INTEGER
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF public.get_role() <> 'system_job'
       OR NOT public.get_elevated_operation()
       OR public.get_school_id() IS NOT NULL
       OR public.get_user_id() IS NULL THEN
        RAISE EXCEPTION 'bounded system queue context required'
            USING ERRCODE = '42501';
    END IF;

    RETURN QUERY
    WITH candidate AS (
        SELECT job.id, asset.school_id
        FROM public.ingestion_jobs AS job
        JOIN public.knowledge_assets AS asset ON asset.id = job.asset_id
        JOIN public.users AS requester
          ON requester.id = job.requested_by
         AND requester.is_active
        WHERE job.stage = 'embed'
          AND job.status = 'queued'
          AND job.available_at <= NOW()
          AND asset.status = 'embedding_pending'
        ORDER BY job.available_at, job.created_at
        FOR UPDATE OF job SKIP LOCKED
        LIMIT 1
    )
    UPDATE public.ingestion_jobs AS job
    SET status = 'running',
        attempts = job.attempts + 1,
        started_at = COALESCE(job.started_at, NOW()),
        locked_at = NOW(),
        heartbeat_at = NOW(),
        error_message = NULL
    FROM candidate
    WHERE job.id = candidate.id
    RETURNING job.id,
              job.asset_id,
              job.requested_by,
              candidate.school_id,
              job.attempts;
END
$$;

CREATE OR REPLACE FUNCTION public.recover_stale_embedding_jobs(
    p_stale_after_seconds BIGINT
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    recovered BIGINT;
BEGIN
    IF public.get_role() <> 'system_job'
       OR NOT public.get_elevated_operation()
       OR public.get_school_id() IS NOT NULL
       OR public.get_user_id() IS NULL THEN
        RAISE EXCEPTION 'bounded system queue context required'
            USING ERRCODE = '42501';
    END IF;

    WITH updated AS (
        UPDATE public.ingestion_jobs AS job
        SET status = 'queued',
            available_at = NOW(),
            locked_at = NULL,
            heartbeat_at = NULL,
            error_message = COALESCE(
                job.error_message,
                'Recovered after stale worker lock'
            )
        FROM public.knowledge_assets AS asset
        WHERE job.asset_id = asset.id
          AND job.stage = 'embed'
          AND job.status = 'running'
          AND asset.status = 'embedding_pending'
          AND COALESCE(
              job.heartbeat_at,
              job.locked_at,
              job.started_at,
              job.created_at
          ) < NOW() - make_interval(
              secs => GREATEST(p_stale_after_seconds, 60)::DOUBLE PRECISION
          )
        RETURNING 1
    )
    SELECT COUNT(*) INTO recovered FROM updated;

    RETURN recovered;
END
$$;

REVOKE EXECUTE ON FUNCTION public.claim_next_embedding_job() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION public.recover_stale_embedding_jobs(BIGINT) FROM PUBLIC;

DROP POLICY IF EXISTS knowledge_assets_scoped_select ON public.knowledge_assets;
CREATE POLICY knowledge_assets_scoped_select ON public.knowledge_assets
FOR SELECT USING (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'SchoolManager'
        AND school_id = public.get_school_id()
    )
    OR (
        public.get_role() = 'Teacher'
        AND school_id = public.get_school_id()
        AND status = 'published'
    )
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND school_id = public.get_school_id()
    )
);

DROP POLICY IF EXISTS knowledge_assets_admin_update ON public.knowledge_assets;
CREATE POLICY knowledge_assets_admin_update ON public.knowledge_assets
FOR UPDATE
USING (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND school_id = public.get_school_id()
    )
)
WITH CHECK (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND school_id = public.get_school_id()
    )
);

DROP POLICY IF EXISTS knowledge_ocr_texts_admin_all ON public.knowledge_ocr_texts;
CREATE POLICY knowledge_ocr_texts_admin_all ON public.knowledge_ocr_texts
FOR ALL
USING (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND EXISTS (
            SELECT 1
            FROM public.knowledge_assets AS asset
            WHERE asset.id = knowledge_ocr_texts.asset_id
              AND asset.school_id = public.get_school_id()
        )
    )
)
WITH CHECK (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND EXISTS (
            SELECT 1
            FROM public.knowledge_assets AS asset
            WHERE asset.id = knowledge_ocr_texts.asset_id
              AND asset.school_id = public.get_school_id()
        )
    )
);

DROP POLICY IF EXISTS knowledge_chunks_admin_all ON public.knowledge_chunks;
CREATE POLICY knowledge_chunks_admin_all ON public.knowledge_chunks
FOR ALL
USING (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND EXISTS (
            SELECT 1
            FROM public.knowledge_assets AS asset
            WHERE asset.id = knowledge_chunks.asset_id
              AND asset.school_id = public.get_school_id()
        )
    )
)
WITH CHECK (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND EXISTS (
            SELECT 1
            FROM public.knowledge_assets AS asset
            WHERE asset.id = knowledge_chunks.asset_id
              AND asset.school_id = public.get_school_id()
        )
    )
);

DROP POLICY IF EXISTS ingestion_jobs_admin_all ON public.ingestion_jobs;
CREATE POLICY ingestion_jobs_admin_all ON public.ingestion_jobs
FOR ALL
USING (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND EXISTS (
            SELECT 1
            FROM public.knowledge_assets AS asset
            WHERE asset.id = ingestion_jobs.asset_id
              AND asset.school_id = public.get_school_id()
        )
    )
)
WITH CHECK (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND EXISTS (
            SELECT 1
            FROM public.knowledge_assets AS asset
            WHERE asset.id = ingestion_jobs.asset_id
              AND asset.school_id = public.get_school_id()
        )
    )
);

DROP POLICY IF EXISTS knowledge_audit_logs_actor_insert ON public.knowledge_audit_logs;
CREATE POLICY knowledge_audit_logs_actor_insert ON public.knowledge_audit_logs
FOR INSERT WITH CHECK (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() IN ('SchoolManager', 'Teacher')
        AND actor_id = public.get_user_id()
        AND school_id = public.get_school_id()
    )
    OR (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND actor_id = public.get_user_id()
        AND school_id = public.get_school_id()
    )
);

COMMENT ON FUNCTION public.claim_next_embedding_job() IS
    'Atomically claims one durable embedding job for an explicitly authorized system scheduler.';
COMMENT ON FUNCTION public.recover_stale_embedding_jobs(BIGINT) IS
    'Requeues stale embedding jobs for an explicitly authorized system scheduler.';
