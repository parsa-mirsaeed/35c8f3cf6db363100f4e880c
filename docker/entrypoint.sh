#!/usr/bin/env bash
set -euo pipefail

wait_for_database() {
    : "${DATABASE_URL:?DATABASE_URL must be set}"

    echo "Waiting for PostgreSQL..."
    until pg_isready --dbname="${DATABASE_URL}" >/dev/null 2>&1; do
        sleep 1
    done
    echo "PostgreSQL is ready."
}

run_migrations() {
    wait_for_database
    cd /opt/edutalent
    bash scripts/ci/apply_migrations.sh
}

case "${1:-server}" in
    migrate)
        run_migrations
        ;;
    server)
        if [[ "${RUN_MIGRATIONS:-true}" == "true" ]]; then
            run_migrations
        else
            wait_for_database
        fi
        cd /opt/edutalent
        exec ./server
        ;;
    *)
        exec "$@"
        ;;
esac
