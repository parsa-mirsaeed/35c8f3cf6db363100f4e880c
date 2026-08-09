#!/usr/bin/env bash
# PR-12 browser evidence on an exact head (shared entry point).
#
# Applies migrations, loads the synthetic fixture, starts the local mock IdP,
# builds the real Dioxus web bundle, starts the production-like server, waits
# for readiness, then runs the tagged Playwright selection. Never contacts a
# live external service. Fails closed. E2E_GREP selects the tag tier.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

: "${E2E_HEAD_SHA:=$(git rev-parse HEAD)}"
: "${E2E_GREP:=@smoke}"
export E2E_HEAD_SHA
export E2E_BASE_URL="${E2E_BASE_URL:-http://127.0.0.1:8080}"
export E2E_ALLOWED_ORIGINS="${E2E_ALLOWED_ORIGINS:-${E2E_BASE_URL},http://127.0.0.1:9100}"
export DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:5432/edutalent_ci}"
export MOCK_IDP_SECRET="${MOCK_IDP_SECRET:-e2e-local-only-secret}"

bash scripts/ci/verify_browser_harness.sh
bash scripts/ci/apply_migrations.sh
psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 -f tests/e2e/fixtures/seed.sql

node tests/e2e/fixtures/mock-idp.mjs &
MOCK_IDP_PID=$!
SERVER_PID=""
cleanup() {
  kill "${MOCK_IDP_PID}" 2>/dev/null || true
  if [[ -n "${SERVER_PID}" ]]; then
    kill "${SERVER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# Evidence runs against the production-like server build, never a mock-only UI.
dx build --web --package web

export SUPABASE_URL="http://127.0.0.1:9100"
export SUPABASE_PROJECT_REF="e2e-local"
export SUPABASE_AUDIENCE="authenticated"
export SUPABASE_PUBLISHABLE_KEY="e2e-publishable"
export SUPABASE_SECRET_KEY="e2e-secret"
export JWT_SECRET="${MOCK_IDP_SECRET}"
export IP="127.0.0.1"
export PORT="8080"
export RUN_MIGRATIONS="false"

server_bin="target/dx/web/debug/web/server"
if [[ ! -x "${server_bin}" ]]; then
  server_bin="target/dx/web/debug/web/web"
fi
test -x "${server_bin}"
"${server_bin}" &
SERVER_PID=$!

ready=false
for _ in $(seq 1 90); do
  if curl --fail --silent "${E2E_BASE_URL}/healthz" >/dev/null 2>&1; then
    ready=true
    break
  fi
  sleep 2
done
if [[ "${ready}" != "true" ]]; then
  echo "production-like server did not become ready" >&2
  exit 1
fi

npm --prefix tests/e2e exec -- playwright test --grep "${E2E_GREP}"

echo "browser evidence complete for head ${E2E_HEAD_SHA} (grep: ${E2E_GREP})"
