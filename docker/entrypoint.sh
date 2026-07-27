#!/usr/bin/env bash
set -euo pipefail

wait_for_database_url() {
    local database_url="$1"
    echo "Waiting for PostgreSQL..."
    until pg_isready --dbname="${database_url}" >/dev/null 2>&1; do
        sleep 1
    done
    echo "PostgreSQL is ready."
}

run_migrations() {
    : "${DATABASE_URL:?DATABASE_URL must be set}"
    wait_for_database_url "${DATABASE_URL}"
    cd /opt/edutalent
    bash scripts/ci/apply_migrations.sh
}

configure_database_role() {
    : "${DATABASE_ADMIN_URL:?DATABASE_ADMIN_URL must be set}"
    wait_for_database_url "${DATABASE_ADMIN_URL}"
    cd /opt/edutalent
    bash scripts/ci/configure_database_role.sh
}

case "${1:-server}" in
    migrate)
        run_migrations
        ;;
    configure-database-role)
        configure_database_role
        ;;
    ai-gateway)
        : "${AI_GATEWAY_INTERNAL_TOKEN:?AI_GATEWAY_INTERNAL_TOKEN must be set}"
        cd /opt/edutalent
        exec ./ai_gateway
        ;;
    server)
        : "${DATABASE_URL:?DATABASE_URL must be set}"
        if [[ "${RUN_MIGRATIONS:-true}" == "true" ]]; then
            run_migrations
        else
            wait_for_database_url "${DATABASE_URL}"
        fi
        cd /opt/edutalent
        exec ./server
        ;;
    *)
        exec "$@"
        ;;
esac
