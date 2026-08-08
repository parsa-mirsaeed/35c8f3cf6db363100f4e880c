-- PR-05: durable, school-scoped assignment personalization queue.
--
-- Publication already fans one custom_assignment row per authorized enrolled
-- student. This migration attaches queue creation to that fan-out transaction,
-- so assignment publication, custom-assignment creation, and personalization
-- enqueue either commit together or roll back together.

CREATE TABLE public.assignment_personalization_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES public.schools(id) ON DELETE CASCADE,
    assignment_id UUID NOT NULL REFERENCES public.assignments(id) ON DELETE CASCADE,
    student_id UUID NOT NULL REFERENCES public.students(id) ON DELETE CASCADE,
    class_section_id UUID NOT NULL REFERENCES public.class_sections(id) ON DELETE CASCADE,
    requested_by UUID NOT NULL,
    target_scope TEXT NOT NULL DEFAULT 'student' CHECK (target_scope = 'student'),
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    lease_owner UUID,
    heartbeat_at TIMESTAMPTZ,
    last_error_code TEXT
        CHECK (last_error_code IS NULL OR last_error_code ~ '^[a-z0-9_]{1,64}$'),
    last_error_summary TEXT
        CHECK (last_error_summary IS NULL OR char_length(last_error_summary) <= 160),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    idempotency_key TEXT NOT NULL UNIQUE CHECK (char_length(idempotency_key) <= 256),
    model_name TEXT NOT NULL,
    profile_name TEXT NOT NULL,
    profile_version INTEGER NOT NULL CHECK (profile_version > 0),
    UNIQUE (assignment_id, student_id, profile_name, profile_version)
);

CREATE INDEX assignment_personalization_jobs_ready_idx
    ON public.assignment_personalization_jobs (available_at, created_at)
    WHERE status = 'queued';
CREATE INDEX assignment_personalization_jobs_school_status_idx
    ON public.assignment_personalization_jobs (school_id, status, created_at DESC);
CREATE INDEX assignment_personalization_jobs_assignment_idx
    ON public.assignment_personalization_jobs (assignment_id, status);
CREATE INDEX assignment_personalization_jobs_heartbeat_idx
    ON public.assignment_personalization_jobs (heartbeat_at)
    WHERE status = 'running';

ALTER TABLE public.assignment_personalization_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_personalization_jobs FORCE ROW LEVEL SECURITY;

CREATE POLICY assignment_personalization_jobs_select_policy
ON public.assignment_personalization_jobs
FOR SELECT
USING (
    (
        public.get_role() = 'Teacher'
        AND requested_by = public.get_user_id()
        AND school_id = public.get_school_id()
    )
    OR (
        public.get_role() = 'SchoolManager'
        AND school_id = public.get_school_id()
    )
    OR public.get_role() = 'PlatformAdmin'
);

CREATE POLICY assignment_personalization_jobs_insert_policy
ON public.assignment_personalization_jobs
FOR INSERT
WITH CHECK (
    public.get_role() = 'Teacher'
    AND requested_by = public.get_user_id()
    AND school_id = public.get_school_id()
);

CREATE POLICY assignment_personalization_jobs_update_policy
ON public.assignment_personalization_jobs
FOR UPDATE
USING (
    public.get_role() = 'Teacher'
    AND requested_by = public.get_user_id()
    AND school_id = public.get_school_id()
)
WITH CHECK (
    public.get_role() = 'Teacher'
    AND requested_by = public.get_user_id()
    AND school_id = public.get_school_id()
);

CREATE OR REPLACE FUNCTION public.enqueue_assignment_personalization_job()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog, public
AS $$
BEGIN
    INSERT INTO public.assignment_personalization_jobs (
        school_id,
        assignment_id,
        student_id,
        class_section_id,
        requested_by,
        status,
        available_at,
        idempotency_key,
        model_name,
        profile_name,
        profile_version
    )
    SELECT
        teacher.school_id,
        assignment.id,
        NEW.student_id,
        assignment.class_section_id,
        teacher.user_id,
        'queued',
        NOW(),
        concat(
            assignment.id::text,
            ':',
            NEW.student_id::text,
            ':assignment_personalization_v1:1'
        ),
        'deepseek-chat',
        'assignment_personalization_v1',
        1
    FROM public.assignments AS assignment
    JOIN public.teachers AS teacher ON teacher.id = assignment.teacher_id
    JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
    JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
    JOIN public.class_sections AS class_section ON class_section.id = assignment.class_section_id
    JOIN public.teaching_assignments AS teaching_assignment
      ON teaching_assignment.teacher_id = teacher.id
     AND teaching_assignment.class_section_id = assignment.class_section_id
    JOIN public.students AS student ON student.id = NEW.student_id
    JOIN public.users AS student_user ON student_user.id = student.user_id
    JOIN public.enrollments AS enrollment
      ON enrollment.student_id = student.id
     AND enrollment.class_section_id = assignment.class_section_id
    WHERE assignment.id = NEW.assignment_id
      AND assignment.status = 'Published'::assignment_status
      AND teacher_user.is_active = TRUE
      AND teacher_role.name::text = 'Teacher'
      AND teacher.school_id = class_section.school_id
      AND student.school_id = teacher.school_id
      AND student_user.school_id = teacher.school_id
      AND student_user.is_active = TRUE
      AND NEW.prompt_ctx IS NULL
    ON CONFLICT (assignment_id, student_id, profile_name, profile_version) DO NOTHING;

    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS custom_assignments_enqueue_personalization
    ON public.custom_assignments;
CREATE TRIGGER custom_assignments_enqueue_personalization
AFTER INSERT ON public.custom_assignments
FOR EACH ROW
EXECUTE FUNCTION public.enqueue_assignment_personalization_job();

-- Backfill any already-published custom assignments that were waiting for the
-- old process-local task when this migration is applied.
INSERT INTO public.assignment_personalization_jobs (
    school_id,
    assignment_id,
    student_id,
    class_section_id,
    requested_by,
    status,
    available_at,
    idempotency_key,
    model_name,
    profile_name,
    profile_version
)
SELECT
    teacher.school_id,
    assignment.id,
    custom_assignment.student_id,
    assignment.class_section_id,
    teacher.user_id,
    'queued',
    NOW(),
    concat(
        assignment.id::text,
        ':',
        custom_assignment.student_id::text,
        ':assignment_personalization_v1:1'
    ),
    'deepseek-chat',
    'assignment_personalization_v1',
    1
FROM public.custom_assignments AS custom_assignment
JOIN public.assignments AS assignment ON assignment.id = custom_assignment.assignment_id
JOIN public.teachers AS teacher ON teacher.id = assignment.teacher_id
JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
JOIN public.class_sections AS class_section ON class_section.id = assignment.class_section_id
JOIN public.teaching_assignments AS teaching_assignment
  ON teaching_assignment.teacher_id = teacher.id
 AND teaching_assignment.class_section_id = assignment.class_section_id
JOIN public.students AS student ON student.id = custom_assignment.student_id
JOIN public.users AS student_user ON student_user.id = student.user_id
JOIN public.enrollments AS enrollment
  ON enrollment.student_id = student.id
 AND enrollment.class_section_id = assignment.class_section_id
WHERE assignment.status = 'Published'::assignment_status
  AND custom_assignment.prompt_ctx IS NULL
  AND teacher_user.is_active = TRUE
  AND teacher_role.name::text = 'Teacher'
  AND teacher.school_id = class_section.school_id
  AND student.school_id = teacher.school_id
  AND student_user.school_id = teacher.school_id
  AND student_user.is_active = TRUE
ON CONFLICT (assignment_id, student_id, profile_name, profile_version) DO NOTHING;

CREATE OR REPLACE FUNCTION public.claim_next_assignment_personalization_job(
    p_worker_id UUID
)
RETURNS TABLE (
    job_id UUID,
    school_id UUID,
    assignment_id UUID,
    student_id UUID,
    requested_by UUID,
    attempt_count INTEGER,
    model_name TEXT,
    profile_name TEXT,
    profile_version INTEGER,
    lease_owner UUID
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF public.get_role() <> 'system_job'
       OR NOT public.get_elevated_operation()
       OR public.get_school_id() IS NOT NULL
       OR public.get_user_id() IS NULL
       OR public.get_user_id() <> p_worker_id THEN
        RAISE EXCEPTION 'bounded assignment personalization queue context required'
            USING ERRCODE = '42501';
    END IF;

    -- A committed personalized payload is authoritative. A crash cannot leave
    -- content committed while the job remains running because the worker writes
    -- both in one transaction, but this reconciliation also makes manual/admin
    -- repair idempotent.
    UPDATE public.assignment_personalization_jobs AS job
    SET status = 'succeeded',
        completed_at = COALESCE(job.completed_at, NOW()),
        lease_owner = NULL,
        heartbeat_at = NULL,
        last_error_code = NULL,
        last_error_summary = NULL
    FROM public.custom_assignments AS custom_assignment
    WHERE job.status IN ('queued', 'running')
      AND custom_assignment.assignment_id = job.assignment_id
      AND custom_assignment.student_id = job.student_id
      AND custom_assignment.prompt_ctx IS NOT NULL;

    -- Reconcile authorization before claiming. Jobs are cancelled rather than
    -- executed when the teacher, assignment, enrollment, or student is no longer
    -- authorized.
    UPDATE public.assignment_personalization_jobs AS job
    SET status = 'cancelled',
        completed_at = NOW(),
        lease_owner = NULL,
        heartbeat_at = NULL,
        last_error_code = 'authorization_revoked',
        last_error_summary = 'Assignment personalization authorization is no longer valid'
    WHERE job.status IN ('queued', 'running')
      AND NOT EXISTS (
          SELECT 1
          FROM public.assignments AS assignment
          JOIN public.teachers AS teacher ON teacher.id = assignment.teacher_id
          JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
          JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
          JOIN public.class_sections AS class_section ON class_section.id = assignment.class_section_id
          JOIN public.teaching_assignments AS teaching_assignment
            ON teaching_assignment.teacher_id = teacher.id
           AND teaching_assignment.class_section_id = assignment.class_section_id
          JOIN public.students AS student ON student.id = job.student_id
          JOIN public.users AS student_user ON student_user.id = student.user_id
          JOIN public.enrollments AS enrollment
            ON enrollment.student_id = student.id
           AND enrollment.class_section_id = assignment.class_section_id
          JOIN public.custom_assignments AS custom_assignment
            ON custom_assignment.assignment_id = assignment.id
           AND custom_assignment.student_id = student.id
          WHERE assignment.id = job.assignment_id
            AND assignment.status = 'Published'::assignment_status
            AND teacher.user_id = job.requested_by
            AND teacher_user.is_active = TRUE
            AND teacher_role.name::text = 'Teacher'
            AND teacher.school_id = job.school_id
            AND class_section.school_id = job.school_id
            AND student.school_id = job.school_id
            AND student_user.school_id = job.school_id
            AND student_user.is_active = TRUE
            AND custom_assignment.prompt_ctx IS NULL
      );

    RETURN QUERY
    WITH candidate AS (
        SELECT job.id
        FROM public.assignment_personalization_jobs AS job
        JOIN public.assignments AS assignment ON assignment.id = job.assignment_id
        JOIN public.teachers AS teacher ON teacher.id = assignment.teacher_id
        JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
        JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
        JOIN public.class_sections AS class_section ON class_section.id = assignment.class_section_id
        JOIN public.teaching_assignments AS teaching_assignment
          ON teaching_assignment.teacher_id = teacher.id
         AND teaching_assignment.class_section_id = assignment.class_section_id
        JOIN public.students AS student ON student.id = job.student_id
        JOIN public.users AS student_user ON student_user.id = student.user_id
        JOIN public.enrollments AS enrollment
          ON enrollment.student_id = student.id
         AND enrollment.class_section_id = assignment.class_section_id
        JOIN public.custom_assignments AS custom_assignment
          ON custom_assignment.assignment_id = assignment.id
         AND custom_assignment.student_id = student.id
        WHERE job.status = 'queued'
          AND job.available_at <= NOW()
          AND assignment.status = 'Published'::assignment_status
          AND teacher.user_id = job.requested_by
          AND teacher_user.is_active = TRUE
          AND teacher_role.name::text = 'Teacher'
          AND teacher.school_id = job.school_id
          AND class_section.school_id = job.school_id
          AND student.school_id = job.school_id
          AND student_user.school_id = job.school_id
          AND student_user.is_active = TRUE
          AND custom_assignment.prompt_ctx IS NULL
        ORDER BY job.available_at, job.created_at
        FOR UPDATE OF job SKIP LOCKED
        LIMIT 1
    )
    UPDATE public.assignment_personalization_jobs AS job
    SET status = 'running',
        attempt_count = job.attempt_count + 1,
        started_at = COALESCE(job.started_at, NOW()),
        lease_owner = p_worker_id,
        heartbeat_at = NOW(),
        last_error_code = NULL,
        last_error_summary = NULL
    FROM candidate
    WHERE job.id = candidate.id
    RETURNING
        job.id,
        job.school_id,
        job.assignment_id,
        job.student_id,
        job.requested_by,
        job.attempt_count,
        job.model_name,
        job.profile_name,
        job.profile_version,
        job.lease_owner;
END
$$;

CREATE OR REPLACE FUNCTION public.recover_stale_assignment_personalization_jobs(
    p_stale_after_seconds BIGINT,
    p_max_attempts INTEGER
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

REVOKE EXECUTE ON FUNCTION public.claim_next_assignment_personalization_job(UUID) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION public.recover_stale_assignment_personalization_jobs(BIGINT, INTEGER) FROM PUBLIC;

COMMENT ON TABLE public.assignment_personalization_jobs IS
    'Durable, idempotent per-student assignment personalization jobs. No prompts, provider payloads, or secrets are stored here.';
COMMENT ON FUNCTION public.claim_next_assignment_personalization_job(UUID) IS
    'Claims one globally discoverable personalization job only for the bounded system queue context; processing remains teacher/school scoped.';
COMMENT ON FUNCTION public.recover_stale_assignment_personalization_jobs(BIGINT, INTEGER) IS
    'Requeues recoverable stale personalization jobs and terminally fails jobs that reached the bounded worker-attempt policy.';
