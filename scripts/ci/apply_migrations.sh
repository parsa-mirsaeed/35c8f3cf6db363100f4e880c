#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set}"

shopt -s nullglob
base_migrations=(packages/api/migration/migrations/*.sql)
incremental_migrations=(migrations/*.sql)
migrations=("${base_migrations[@]}" "${incremental_migrations[@]}")

if (( ${#base_migrations[@]} == 0 )); then
    echo "No canonical base migrations found under packages/api/migration/migrations" >&2
    exit 1
fi

if (( ${#incremental_migrations[@]} == 0 )); then
    echo "No incremental migrations found under migrations" >&2
    exit 1
fi

migration_checksum() {
    local migration="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${migration}" | awk '{print $1}'
    else
        shasum -a 256 "${migration}" | awk '{print $1}'
    fi
}

sql_literal() {
    local value="$1"
    printf '%s' "${value//\'/\'\'}"
}

# Reproduce external database objects that exist in deployed Supabase environments
# but are not owned by this repository's application migrations. The local table
# remains empty; only its contract is needed by UUID sync and provisioning migrations.
#
# A self-hosted Supabase database already owns and protects auth.users. PostgreSQL
# checks schema CREATE privileges even for CREATE TABLE IF NOT EXISTS, so create the
# compatibility stub only when the table is genuinely absent. This preserves the
# plain-PostgreSQL CI contract without attempting to modify Supabase Auth internals.
#
# edutalent_migration_files makes the canonical file runner safe to execute on every
# container start. A previously applied file is skipped only when its checksum is
# unchanged; edited historical migrations fail closed.
psql "$DATABASE_URL" --set=ON_ERROR_STOP=1 <<'SQL'
CREATE SCHEMA IF NOT EXISTS auth;
DO $edutalent_auth_stub$
BEGIN
    IF to_regclass('auth.users') IS NULL THEN
        EXECUTE $create_auth_users$
            CREATE TABLE auth.users (
                id UUID PRIMARY KEY,
                email TEXT UNIQUE,
                raw_user_meta_data JSONB NOT NULL DEFAULT '{}'::JSONB,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $create_auth_users$;
    END IF;
END
$edutalent_auth_stub$;

CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS public.edutalent_migration_files (
    path TEXT PRIMARY KEY,
    checksum TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
SQL

pending=()
for migration in "${migrations[@]}"; do
    checksum="$(migration_checksum "${migration}")"
    migration_key="$(sql_literal "${migration}")"
    stored_checksum="$(psql "$DATABASE_URL" --tuples-only --no-align --command="SELECT checksum FROM public.edutalent_migration_files WHERE path = '${migration_key}'")"

    if [[ -z "${stored_checksum}" ]]; then
        pending+=("${migration}")
    elif [[ "${stored_checksum}" == "${checksum}" ]]; then
        echo "Already applied: ${migration}"
    else
        echo "Applied migration changed on disk: ${migration}" >&2
        echo "Stored checksum:  ${stored_checksum}" >&2
        echo "Current checksum: ${checksum}" >&2
        exit 1
    fi
done

if (( ${#pending[@]} == 0 )); then
    echo "All migrations already applied successfully"
    exit 0
fi

# Historical incremental migrations are not fully topologically ordered: some
# seed/index files precede migrations that create their dependencies. Every attempt
# is wrapped in a transaction, so a failed dependency attempt leaves no partial
# changes. Canonical package migrations are ordered first, then remaining gaps are
# resolved in deterministic retry passes.
pass=1
max_passes=$(( ${#pending[@]} + 1 ))

while (( ${#pending[@]} > 0 )); do
    echo "Migration pass ${pass}: ${#pending[@]} file(s) pending"
    progress=0
    deferred=()

    for migration in "${pending[@]}"; do
        echo "::group::Applying ${migration}"
        attempt_log="$(mktemp)"
        checksum="$(migration_checksum "${migration}")"
        migration_key="$(sql_literal "${migration}")"
        marker_sql="INSERT INTO public.edutalent_migration_files (path, checksum) VALUES ('${migration_key}', '${checksum}');"

        if psql "$DATABASE_URL" \
            --set=ON_ERROR_STOP=1 \
            --single-transaction \
            --file="${migration}" \
            --command="${marker_sql}" >"${attempt_log}" 2>&1; then
            cat "${attempt_log}"
            rm -f "${attempt_log}"
            progress=$((progress + 1))
            echo "::endgroup::"
            continue
        fi

        cat "${attempt_log}"
        rm -f "${attempt_log}"
        deferred+=("${migration}")
        echo "Deferred until dependencies are available: ${migration}"
        echo "::endgroup::"
    done

    if (( ${#deferred[@]} == 0 )); then
        echo "All migrations applied successfully"
        exit 0
    fi

    if (( progress == 0 )); then
        echo "No migration made progress during pass ${pass}." >&2
        echo "Unresolved migrations:" >&2
        printf '  - %s\n' "${deferred[@]}" >&2
        echo "Re-running unresolved migrations to emit their final errors:" >&2

        for migration in "${deferred[@]}"; do
            echo "::group::Final failure: ${migration}"
            psql "$DATABASE_URL" \
                --set=ON_ERROR_STOP=1 \
                --single-transaction \
                --file="${migration}" || true
            echo "::endgroup::"
        done
        exit 1
    fi

    pending=("${deferred[@]}")
    pass=$((pass + 1))
    if (( pass > max_passes )); then
        echo "Exceeded maximum migration dependency passes" >&2
        exit 1
    fi
done
