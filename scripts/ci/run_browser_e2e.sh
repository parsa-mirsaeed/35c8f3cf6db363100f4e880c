#!/usr/bin/env bash
# PR-12 browser evidence on an exact head (shared entry point).
#
# Applies migrations, loads the synthetic fixture, starts the local mock IdP,
# builds the real Dioxus web bundle, starts the production-like server, waits
# for readiness, then runs the tagged Playwright selection. Never contacts a
# live external service. Fails closed. E2E_GREP selects the tag tier.
set -euxo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

: "${E2E_HEAD_SHA:=$(git rev-parse HEAD)}"
: "${E2E_GREP:=@smoke}"
export E2E_HEAD_SHA
export E2E_BASE_URL="${E2E_BASE_URL:-http://127.0.0.1:8080}"
export E2E_ALLOWED_ORIGINS="${E2E_ALLOWED_ORIGINS:-${E2E_BASE_URL},http://127.0.0.1:9100}"
export DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:5432/edutalent_ci}"

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
export SUPABASE_JWT_ISSUER="http://127.0.0.1:9100/auth/v1"
export SUPABASE_PUBLISHABLE_KEY="e2e-publishable"
export SUPABASE_SECRET_KEY="e2e-server-only"
export IP="127.0.0.1"
export PORT="8080"
export RUN_MIGRATIONS="false"

# Discover the server binary the same way the release Dockerfile does.
bundle_dir="target/dx/web/debug/web"
server_bin=""
for candidate in "${bundle_dir}/server" "${bundle_dir}/web"; do
  if [[ -x "${candidate}" ]]; then
    server_bin="${candidate}"
    break
  fi
done
if [[ -z "${server_bin}" ]]; then
  server_bin="$(find "${bundle_dir}" -maxdepth 1 -type f -perm /111 | head -n 1 || true)"
fi
if [[ -z "${server_bin}" || ! -x "${server_bin}" ]]; then
  echo "no executable server binary found under ${bundle_dir}" >&2
  ls -la "${bundle_dir}" >&2 || true
  exit 1
fi
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

# Run from the harness directory so the local Playwright binary resolves.
cd "${ROOT}/tests/e2e"
npx playwright test --grep "${E2E_GREP}"

echo "browser evidence complete for head ${E2E_HEAD_SHA} (grep: ${E2E_GREP})"
