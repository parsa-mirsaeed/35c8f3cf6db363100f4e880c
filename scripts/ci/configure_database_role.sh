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

# Supabase's hardened postgres role is deliberately not a superuser. It has the
# CREATEROLE, CREATEDB, REPLICATION, and BYPASSRLS attributes needed to create
# and reconcile this constrained backend role, but PostgreSQL does not allow a
# non-superuser to specify either SUPERUSER or NOSUPERUSER in ALTER ROLE.
# Refuse an unexpectedly privileged pre-existing target instead of trying to
# demote it or silently continuing with a compromised trust boundary.
#
# Feed SQL through stdin rather than --command so psql performs its safe
# :'app_role' variable substitution before sending the query to PostgreSQL.
# Boolean values explicitly cast to text are rendered as "true" or "false";
# they are not psql's native abbreviated "t" or "f" display values. Keep every
# cast postcondition on this normalized representation.
existing_super="$(
    psql "${DATABASE_ADMIN_URL}" \
        --tuples-only \
        --no-align \
        --set=ON_ERROR_STOP=1 \
        --set=app_role="${DATABASE_APP_USER}" <<'SQL'
SELECT rolsuper::text
FROM pg_roles
WHERE rolname = :'app_role';
SQL
)"
case "${existing_super}" in
    ""|false) ;;
    true)
        echo "Refusing to configure superuser role ${DATABASE_APP_USER}" >&2
        exit 1
        ;;
    *)
        echo "Unable to verify existing role attributes for ${DATABASE_APP_USER}: ${existing_super}" >&2
        exit 1
        ;;
esac

psql "${DATABASE_ADMIN_URL}" \
    --set=ON_ERROR_STOP=1 \
    --set=app_role="${DATABASE_APP_USER}" \
    --set=app_password="${DATABASE_APP_PASSWORD}" <<'SQL'
-- EduTalent's current repository layer performs authenticated server-side
-- authorization and does not set transaction-local RLS claims. Therefore the
-- backend role is deliberately BYPASSRLS, matching a Supabase service role, but
-- is not a superuser and receives no DDL/role/database/replication privileges.
-- Supabase browser/client roles remain subject to the existing RLS policies.
SELECT format(
    'CREATE ROLE %I LOGIN PASSWORD %L NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOREPLICATION BYPASSRLS',
    :'app_role',
    :'app_password'
)
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'app_role')
\gexec

-- Do not include NOSUPERUSER here. The pinned Supabase postgres role is a
-- non-superuser, and PostgreSQL reserves changes to the SUPERUSER property for
-- superusers even when the requested value is NOSUPERUSER. The explicit check
-- above guarantees an existing target is not privileged before reconciliation.
SELECT format(
    'ALTER ROLE %I WITH LOGIN PASSWORD %L NOINHERIT NOCREATEDB NOCREATEROLE NOREPLICATION BYPASSRLS',
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
-- application type with the correct object class. Table-created row types and
-- array helper types are excluded; standalone composites, enums, ranges,
-- multiranges, base types, and domains are included. Only types owned by the
-- migration identity are modified. Extension-owned types must already expose
-- USAGE (normally through PUBLIC) and are covered by the postcondition below.
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
if [[ "${role_state}" != "true|false|false|false|false|false|true" ]]; then
    echo "Backend role attributes do not match the production security contract: ${role_state:-<missing>}" >&2
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

echo "Configured dedicated non-superuser EduTalent database role."
