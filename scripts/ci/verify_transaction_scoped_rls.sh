#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set}"

app_role="edutalent_app_ci"
app_password="RlsTransactionProbe.Password_2026_safe"

DATABASE_ADMIN_URL="${DATABASE_URL}" \
DATABASE_APP_USER="${app_role}" \
DATABASE_APP_PASSWORD="${app_password}" \
    bash scripts/ci/configure_database_role.sh

psql "${DATABASE_URL}" \
    --set=ON_ERROR_STOP=1 \
    --set=app_role="${app_role}" <<'SQL'
SELECT set_config('edutalent.verification_role', :'app_role', false);

DO $verification$
DECLARE
    role_state text;
    missing_force text[];
    obsolete_policy_count integer;
    verification_role text := current_setting('edutalent.verification_role');
BEGIN
    SELECT concat_ws('|',
        rolcanlogin::text,
        rolsuper::text,
        rolinherit::text,
        rolcreatedb::text,
        rolcreaterole::text,
        rolreplication::text,
        rolbypassrls::text
    )
    INTO role_state
    FROM pg_roles
    WHERE rolname = verification_role;

    IF role_state <> 'true|false|false|false|false|false|false' THEN
        RAISE EXCEPTION 'Unexpected runtime role state: %', role_state;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM pg_auth_members membership
        JOIN pg_roles member_role ON member_role.oid = membership.member
        WHERE member_role.rolname = verification_role
    ) THEN
        RAISE EXCEPTION 'Runtime role retains role memberships';
    END IF;

    SELECT array_agg(relation.relname ORDER BY relation.relname)
    INTO missing_force
    FROM pg_class AS relation
    JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
    WHERE namespace.nspname = 'public'
      AND relation.relkind IN ('r', 'p')
      AND relation.relrowsecurity
      AND NOT relation.relforcerowsecurity
      AND relation.relname NOT IN ('_sqlx_migrations', 'edutalent_migration_files');

    IF missing_force IS NOT NULL THEN
        RAISE EXCEPTION 'RLS-enabled application tables are not forced: %', missing_force;
    END IF;

    SELECT COUNT(*)
    INTO obsolete_policy_count
    FROM pg_policies
    WHERE schemaname = 'public'
      AND (COALESCE(qual, '') || COALESCE(with_check, '')) LIKE '%app.current_%';

    IF obsolete_policy_count <> 0 THEN
        RAISE EXCEPTION 'Policies reference obsolete app.current_* settings';
    END IF;

    IF pg_get_functiondef('public.get_user_id()'::regprocedure) NOT LIKE '%app.user_id%'
       OR pg_get_functiondef('public.get_role()'::regprocedure) NOT LIKE '%app.user_role%'
       OR pg_get_functiondef('public.get_school_id()'::regprocedure) NOT LIKE '%app.school_id%'
       OR pg_get_functiondef('public.get_elevated_operation()'::regprocedure)
          NOT LIKE '%app.elevated_operation%' THEN
        RAISE EXCEPTION 'Canonical RLS helper functions do not use the expected settings';
    END IF;

    IF has_function_privilege(
        verification_role,
        'public.set_app_context(uuid,text,uuid)',
        'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'Runtime role can execute retired set_app_context';
    END IF;

    IF NOT has_function_privilege(
        verification_role,
        'public.claim_next_embedding_job()',
        'EXECUTE'
    ) OR NOT has_function_privilege(
        verification_role,
        'public.recover_stale_embedding_jobs(bigint)',
        'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'Runtime role cannot execute bounded system queue functions';
    END IF;

    IF pg_get_functiondef('public.claim_next_embedding_job()'::regprocedure)
           NOT LIKE '%bounded system queue context required%'
       OR pg_get_functiondef('public.claim_next_embedding_job()'::regprocedure)
           NOT LIKE '%get_elevated_operation()%'
       OR pg_get_functiondef('public.recover_stale_embedding_jobs(bigint)'::regprocedure)
           NOT LIKE '%bounded system queue context required%'
       OR pg_get_functiondef('public.recover_stale_embedding_jobs(bigint)'::regprocedure)
           NOT LIKE '%get_elevated_operation()%' THEN
        RAISE EXCEPTION 'System queue functions do not enforce the bounded context';
    END IF;
END
$verification$;

DROP TABLE IF EXISTS public.edutalent_rls_transaction_probe;
CREATE TABLE public.edutalent_rls_transaction_probe (
    id UUID PRIMARY KEY,
    school_id UUID NOT NULL,
    marker TEXT NOT NULL
);
ALTER TABLE public.edutalent_rls_transaction_probe ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.edutalent_rls_transaction_probe FORCE ROW LEVEL SECURITY;
CREATE POLICY edutalent_rls_transaction_probe_school
ON public.edutalent_rls_transaction_probe
FOR SELECT
USING (school_id = public.get_school_id());
INSERT INTO public.edutalent_rls_transaction_probe (id, school_id, marker)
VALUES
    ('10000000-0000-0000-0000-000000000001', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'school-a'),
    ('20000000-0000-0000-0000-000000000002', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'school-b');
SELECT format(
    'GRANT SELECT ON TABLE public.edutalent_rls_transaction_probe TO %I',
    :'app_role'
)
\gexec
SQL

expect_role_failure() {
    local label="$1"
    local sql="$2"
    if psql "${DATABASE_URL}" \
        --quiet \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${app_role}" \
        >/tmp/edutalent-rls-denied.log 2>&1 <<SQL
SET ROLE :"app_role";
${sql};
SQL
    then
        echo "Runtime role unexpectedly succeeded: ${label}" >&2
        cat /tmp/edutalent-rls-denied.log >&2
        exit 1
    fi
}

expect_role_failure "create schema" "CREATE SCHEMA edutalent_forbidden_schema"
expect_role_failure "create role" "CREATE ROLE edutalent_forbidden_role"
expect_role_failure "create database" "CREATE DATABASE edutalent_forbidden_database"
expect_role_failure \
    "write migration registry" \
    "INSERT INTO public.edutalent_migration_files(path, checksum) VALUES ('forbidden', 'forbidden')"
expect_role_failure \
    "claim queue without bounded system context" \
    "SELECT * FROM public.claim_next_embedding_job()"

psql "${DATABASE_URL}" \
    --quiet \
    --set=ON_ERROR_STOP=1 \
    --set=app_role="${app_role}" <<'SQL'
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id = '33333333-3333-3333-3333-333333333333';
SET LOCAL app.user_role = 'system_job';
SET LOCAL app.school_id = '';
SET LOCAL app.elevated_operation = 'true';
SELECT COUNT(*) FROM public.claim_next_embedding_job();
SELECT public.recover_stale_embedding_jobs(60);
ROLLBACK;
SQL

query_school() {
    local school_id="$1"
    psql "${DATABASE_URL}" \
        --quiet \
        --tuples-only \
        --no-align \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${app_role}" \
        --set=school_id="${school_id}" <<'SQL'
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id = '11111111-1111-1111-1111-111111111111';
SET LOCAL app.user_role = 'Teacher';
SET LOCAL app.school_id = :'school_id';
SET LOCAL app.elevated_operation = 'false';
SELECT string_agg(marker, ',' ORDER BY marker)
FROM public.edutalent_rls_transaction_probe;
ROLLBACK;
SQL
}

query_school "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa" > /tmp/edutalent-rls-school-a.out &
pid_a=$!
query_school "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb" > /tmp/edutalent-rls-school-b.out &
pid_b=$!
wait "${pid_a}"
wait "${pid_b}"

school_a_result="$(grep -E '^school-' /tmp/edutalent-rls-school-a.out | tail -n 1)"
school_b_result="$(grep -E '^school-' /tmp/edutalent-rls-school-b.out | tail -n 1)"
if [[ "${school_a_result}" != "school-a" ]]; then
    echo "School A transaction observed unexpected rows: ${school_a_result:-<none>}" >&2
    cat /tmp/edutalent-rls-school-a.out >&2
    exit 1
fi
if [[ "${school_b_result}" != "school-b" ]]; then
    echo "School B transaction observed unexpected rows: ${school_b_result:-<none>}" >&2
    cat /tmp/edutalent-rls-school-b.out >&2
    exit 1
fi

context_after_commit="$(
    psql "${DATABASE_URL}" \
        --quiet \
        --tuples-only \
        --no-align \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${app_role}" <<'SQL'
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id = '11111111-1111-1111-1111-111111111111';
SET LOCAL app.user_role = 'Teacher';
SET LOCAL app.school_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
SET LOCAL app.elevated_operation = 'false';
COMMIT;
SELECT concat_ws('|',
    NULLIF(current_setting('app.user_id', true), ''),
    NULLIF(current_setting('app.user_role', true), ''),
    NULLIF(current_setting('app.school_id', true), ''),
    NULLIF(current_setting('app.elevated_operation', true), '')
);
SQL
)"
if [[ -n "${context_after_commit}" ]]; then
    echo "Transaction context leaked after commit: ${context_after_commit}" >&2
    exit 1
fi

context_after_rollback="$(
    psql "${DATABASE_URL}" \
        --quiet \
        --tuples-only \
        --no-align \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${app_role}" <<'SQL'
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id = '11111111-1111-1111-1111-111111111111';
SET LOCAL app.user_role = 'Teacher';
SET LOCAL app.school_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
SET LOCAL app.elevated_operation = 'false';
ROLLBACK;
SELECT concat_ws('|',
    NULLIF(current_setting('app.user_id', true), ''),
    NULLIF(current_setting('app.user_role', true), ''),
    NULLIF(current_setting('app.school_id', true), ''),
    NULLIF(current_setting('app.elevated_operation', true), '')
);
SQL
)"
if [[ -n "${context_after_rollback}" ]]; then
    echo "Transaction context leaked after rollback: ${context_after_rollback}" >&2
    exit 1
fi

psql "${DATABASE_URL}" --set=ON_ERROR_STOP=1 <<'SQL'
DROP TABLE public.edutalent_rls_transaction_probe;
SQL

rm -f \
    /tmp/edutalent-rls-denied.log \
    /tmp/edutalent-rls-school-a.out \
    /tmp/edutalent-rls-school-b.out

echo "transaction-scoped RLS and NOBYPASSRLS role verified"
