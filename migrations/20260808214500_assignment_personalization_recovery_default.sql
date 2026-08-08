-- Keep stale-worker recovery bounded while allowing the default policy to be
-- used by callers that do not need to override the worker attempt limit.
-- The two-argument signature remains unchanged; only the second argument gains
-- a safe default so existing one-argument SQL callers resolve to the same
-- SECURITY DEFINER function rather than a separate compatibility surface.

CREATE OR REPLACE FUNCTION public.recover_stale_assignment_personalization_jobs(
    p_stale_after_seconds BIGINT,
    p_max_attempts INTEGER DEFAULT 5
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    reconciled BIGINT;
    max_attempts INTEGER := GREATEST(1, LEAST(COALESCE(p_max_attempts, 5), 10));
BEGIN
    IF public.get_role() <> 'system_job'
       OR NOT public.get_elevated_operation()
       OR public.get_school_id() IS NOT NULL
       OR public.get_user_id() IS NULL THEN
        RAISE EXCEPTION 'bounded assignment personalization queue context required'
            USING ERRCODE = '42501';
    END IF;

    WITH updated AS (
        UPDATE public.assignment_personalization_jobs AS job
        SET status = CASE
                WHEN job.attempt_count >= max_attempts THEN 'failed'
                ELSE 'queued'
            END,
            available_at = CASE
                WHEN job.attempt_count >= max_attempts THEN job.available_at
                ELSE NOW()
            END,
            lease_owner = NULL,
            heartbeat_at = NULL,
            completed_at = CASE
                WHEN job.attempt_count >= max_attempts THEN NOW()
                ELSE NULL
            END,
            last_error_code = CASE
                WHEN job.attempt_count >= max_attempts THEN 'worker_restart_limit'
                ELSE 'stale_lease_recovered'
            END,
            last_error_summary = CASE
                WHEN job.attempt_count >= max_attempts
                    THEN 'Personalization stopped after repeated worker interruptions'
                ELSE 'Recovered after stale personalization worker lease'
            END
        WHERE job.status = 'running'
          AND COALESCE(job.heartbeat_at, job.started_at, job.created_at)
                < NOW() - make_interval(
                    secs => GREATEST(p_stale_after_seconds, 60)::DOUBLE PRECISION
                )
        RETURNING 1
    )
    SELECT COUNT(*) INTO reconciled FROM updated;

    RETURN reconciled;
END
$$;

REVOKE EXECUTE ON FUNCTION public.recover_stale_assignment_personalization_jobs(BIGINT, INTEGER)
FROM PUBLIC;

COMMENT ON FUNCTION public.recover_stale_assignment_personalization_jobs(BIGINT, INTEGER) IS
    'Requeues stale assignment personalization jobs below the bounded attempt limit and fails repeatedly interrupted jobs; defaults to five attempts when no override is supplied.';
