-- Break the recursive policy graph between enrollments and students without
-- weakening tenant isolation.
--
-- Before this migration:
--   enrollments_select_policy -> students
--   students_select_policy    -> enrollments
-- PostgreSQL expands both FORCE-RLS policies and aborts with:
--   infinite recursion detected in policy for relation "enrollments"
--
-- EduTalent's migration runner may defer historical files until their helper
-- dependencies exist. This correction must therefore run only after the legacy
-- RLS policy migration and the transaction-scoped RLS finalizer have completed;
-- otherwise this file could create enrollments_select_policy before the deferred
-- historical migration attempts to create its original version.

DO $prerequisites$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM public.edutalent_migration_files
        WHERE path = 'migrations/20260103000001_enable_rls_policies.sql'
    ) OR NOT EXISTS (
        SELECT 1
        FROM public.edutalent_migration_files
        WHERE path = 'migrations/20260805121600_finalize_transaction_scoped_rls.sql'
    ) THEN
        RAISE EXCEPTION
            'enrollment RLS recursion fix requires the finalized legacy RLS policy chain';
    END IF;
END
$prerequisites$;

-- The helper below performs only the narrow relationship check required by the
-- enrollment policy. It executes as the migration owner (the controlled
-- BYPASSRLS administration identity), has a fixed search path, returns only a
-- boolean, and is not executable by PUBLIC. The dedicated EduTalent runtime
-- role receives EXECUTE through scripts/ci/configure_database_role.sh, which
-- grants application functions after migrations while keeping that role
-- NOBYPASSRLS and without role memberships.
CREATE OR REPLACE FUNCTION public.enrollment_student_actor_matches(
    p_student_id uuid
)
RETURNS boolean
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $function$
    SELECT EXISTS (
        SELECT 1
        FROM public.students AS s
        WHERE s.id = p_student_id
          AND (
              s.user_id = public.get_user_id()
              OR s.parent_id = public.get_user_id()
          )
    );
$function$;

REVOKE ALL
ON FUNCTION public.enrollment_student_actor_matches(uuid)
FROM PUBLIC;

DROP POLICY IF EXISTS enrollments_select_policy
ON public.enrollments;

CREATE POLICY enrollments_select_policy
ON public.enrollments
FOR SELECT
USING (
    -- The enrolled student or that student's parent.
    public.enrollment_student_actor_matches(enrollments.student_id)

    -- A teacher assigned to the enrollment's class.
    OR EXISTS (
        SELECT 1
        FROM public.teaching_assignments AS ta
        JOIN public.teachers AS t
          ON t.id = ta.teacher_id
        WHERE ta.class_section_id = enrollments.class_section_id
          AND t.user_id = public.get_user_id()
    )

    -- A School Manager scoped to the class's school.
    OR (
        public.is_school_manager()
        AND EXISTS (
            SELECT 1
            FROM public.class_sections AS cs
            WHERE cs.id = enrollments.class_section_id
              AND cs.school_id = public.get_school_id()
        )
    )
);
