#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_ADMIN_URL:?DATABASE_ADMIN_URL must be set}"
: "${DATABASE_APP_USER:?DATABASE_APP_USER must be set}"
: "${DATABASE_APP_PASSWORD:?DATABASE_APP_PASSWORD must be set}"

if [[ ! "${DATABASE_APP_USER}" =~ ^[a-z_][a-z0-9_]{0,62}$ ]]; then
    echo "DATABASE_APP_USER must be a lowercase PostgreSQL identifier" >&2
    exit 1
fi

# Keep the generated credential URL-safe so it can be embedded in DATABASE_URL
# without ambiguous parser/percent-encoding behavior.
if [[ ! "${DATABASE_APP_PASSWORD}" =~ ^[A-Za-z0-9._~-]{32,128}$ ]]; then
    echo "DATABASE_APP_PASSWORD must be 32-128 URL-safe characters" >&2
    exit 1
fi

# Supabase's hardened postgres role is deliberately not a superuser. Refuse an
# unexpectedly privileged or role-member target instead of silently preserving
# a SET ROLE path outside the long-running application boundary.
existing_state="$(
    psql "${DATABASE_ADMIN_URL}" \
        --tuples-only \
        --no-align \
        --field-separator='|' \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${DATABASE_APP_USER}" <<'SQL'
SELECT role_entry.rolsuper::text,
       EXISTS (
           SELECT 1
           FROM pg_auth_members membership
           WHERE membership.member = role_entry.oid
       )::text
FROM pg_roles AS role_entry
WHERE role_entry.rolname = :'app_role';
SQL
)"
case "${existing_state}" in
    ""|"false|false") ;;
    "true|"*|"false|true")
        echo "Refusing to configure privileged or role-member identity ${DATABASE_APP_USER}: ${existing_state}" >&2
        exit 1
        ;;
    *)
        echo "Unable to verify existing role attributes for ${DATABASE_APP_USER}: ${existing_state}" >&2
        exit 1
        ;;
esac

psql "${DATABASE_ADMIN_URL}" \
    --set=ON_ERROR_STOP=1 \
    --set=app_role="${DATABASE_APP_USER}" \
    --set=app_password="${DATABASE_APP_PASSWORD}" <<'SQL'
-- The long-running application/worker identity is deliberately NOBYPASSRLS.
-- Every protected query must execute through a transaction carrying canonical
-- app.user_id, app.user_role, and app.school_id context.
SELECT format(
    'CREATE ROLE %I LOGIN PASSWORD %L NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS',
    :'app_role',
    :'app_password'
)
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'app_role')
\gexec

-- Do not include NOSUPERUSER here. The pinned Supabase postgres role is a
-- non-superuser, and PostgreSQL reserves changes to the SUPERUSER property for
-- superusers. The explicit precondition above rejects a privileged target.
SELECT format(
    'ALTER ROLE %I WITH LOGIN PASSWORD %L NOINHERIT NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS',
    :'app_role',
    :'app_password'
)
\gexec

SELECT format('GRANT CONNECT ON DATABASE %I TO %I', current_database(), :'app_role')
\gexec
SELECT format('GRANT USAGE ON SCHEMA public TO %I', :'app_role')
\gexec
SELECT format('REVOKE CREATE ON SCHEMA public FROM %I', :'app_role')
\gexec
SELECT format(
    'GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO %I',
    :'app_role'
)
\gexec
SELECT format(
    'GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO %I',
    :'app_role'
)
\gexec

-- PostgreSQL has no "ALL TYPES IN SCHEMA" grant syntax. Grant each concrete
-- repository-owned application type with the correct object class.
SELECT format(
    'GRANT USAGE ON %s %I.%I TO %I',
    CASE WHEN type_entry.typtype = 'd' THEN 'DOMAIN' ELSE 'TYPE' END,
    namespace_entry.nspname,
    type_entry.typname,
    :'app_role'
)
FROM pg_type AS type_entry
JOIN pg_namespace AS namespace_entry
    ON namespace_entry.oid = type_entry.typnamespace
LEFT JOIN pg_class AS relation_entry
    ON relation_entry.oid = type_entry.typrelid
WHERE namespace_entry.nspname = 'public'
  AND type_entry.typisdefined
  AND type_entry.typelem = 0
  AND type_entry.typowner = (
      SELECT role_entry.oid
      FROM pg_roles AS role_entry
      WHERE role_entry.rolname = current_user
  )
  AND (
      type_entry.typtype IN ('b', 'd', 'e', 'r', 'm')
      OR (type_entry.typtype = 'c' AND relation_entry.relkind = 'c')
  )
ORDER BY type_entry.oid
\gexec

SELECT format('GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO %I', :'app_role')
\gexec

-- Pool-scoped context setup is retired. Runtime code sets transaction-local
-- values only after beginning a pinned AuthorizedTx.
SELECT format(
    'REVOKE EXECUTE ON FUNCTION public.set_app_context(uuid, text, uuid) FROM %I',
    :'app_role'
)
WHERE to_regprocedure('public.set_app_context(uuid,text,uuid)') IS NOT NULL
\gexec

-- Migration integrity state is never writable by the application process.
SELECT format(
    'REVOKE ALL PRIVILEGES ON TABLE public.edutalent_migration_files FROM %I',
    :'app_role'
)
WHERE to_regclass('public.edutalent_migration_files') IS NOT NULL
\gexec
SELECT format(
    'REVOKE ALL PRIVILEGES ON TABLE public._sqlx_migrations FROM %I',
    :'app_role'
)
WHERE to_regclass('public._sqlx_migrations') IS NOT NULL
\gexec

SELECT format(
    'ALTER ROLE %I IN DATABASE %I SET search_path = public',
    :'app_role',
    current_database()
)
\gexec
SQL

role_state="$(
    psql "${DATABASE_ADMIN_URL}" \
        --tuples-only \
        --no-align \
        --field-separator='|' \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${DATABASE_APP_USER}" <<'SQL'
SELECT rolcanlogin::text,
       rolsuper::text,
       rolinherit::text,
       rolcreatedb::text,
       rolcreaterole::text,
       rolreplication::text,
       rolbypassrls::text
FROM pg_roles
WHERE rolname = :'app_role';
SQL
)"
if [[ "${role_state}" != "true|false|false|false|false|false|false" ]]; then
    echo "Backend role attributes do not match the NOBYPASSRLS production contract: ${role_state:-<missing>}" >&2
    exit 1
fi

membership_state="$(
    psql "${DATABASE_ADMIN_URL}" \
        --tuples-only \
        --no-align \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${DATABASE_APP_USER}" <<'SQL'
SELECT EXISTS (
    SELECT 1
    FROM pg_auth_members membership
    JOIN pg_roles member_role ON member_role.oid = membership.member
    WHERE member_role.rolname = :'app_role'
)::text;
SQL
)"
if [[ "${membership_state}" != "false" ]]; then
    echo "Backend role retains prohibited role memberships" >&2
    exit 1
fi

type_usage_state="$(
    psql "${DATABASE_ADMIN_URL}" \
        --tuples-only \
        --no-align \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${DATABASE_APP_USER}" <<'SQL'
SELECT COALESCE(
    bool_and(has_type_privilege(:'app_role', type_entry.oid, 'USAGE')),
    true
)::text
FROM pg_type AS type_entry
JOIN pg_namespace AS namespace_entry
    ON namespace_entry.oid = type_entry.typnamespace
LEFT JOIN pg_class AS relation_entry
    ON relation_entry.oid = type_entry.typrelid
WHERE namespace_entry.nspname = 'public'
  AND type_entry.typisdefined
  AND type_entry.typelem = 0
  AND (
      type_entry.typtype IN ('b', 'd', 'e', 'r', 'm')
      OR (type_entry.typtype = 'c' AND relation_entry.relkind = 'c')
  );
SQL
)"
if [[ "${type_usage_state}" != "true" ]]; then
    echo "Backend role is missing USAGE on one or more public application types: ${type_usage_state:-<missing>}" >&2
    exit 1
fi

legacy_context_execute="$(
    psql "${DATABASE_ADMIN_URL}" \
        --tuples-only \
        --no-align \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${DATABASE_APP_USER}" <<'SQL'
SELECT COALESCE(
    has_function_privilege(:'app_role', 'public.set_app_context(uuid,text,uuid)', 'EXECUTE'),
    false
)::text;
SQL
)"
if [[ "${legacy_context_execute}" != "false" ]]; then
    echo "Backend role can still execute retired pool-scoped set_app_context" >&2
    exit 1
fi

echo "Configured dedicated NOBYPASSRLS EduTalent database role."
