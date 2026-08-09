#!/usr/bin/env bash
# PR-12 Tier-1 browser smoke on an exact head.
#
# Starts the production-like stack against local CI PostgreSQL and the mock IdP,
# runs only @smoke specs on one engine, and writes compact evidence. Never
# contacts a live external service. Fails closed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

: "${E2E_HEAD_SHA:=$(git rev-parse HEAD)}"
export E2E_HEAD_SHA
export E2E_BASE_URL="${E2E_BASE_URL:-http://127.0.0.1:8080}"
export E2E_ALLOWED_ORIGINS="${E2E_ALLOWED_ORIGINS:-${E2E_BASE_URL},http://127.0.0.1:9100}"
export DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:5432/edutalent_ci}"

bash scripts/ci/apply_migrations.sh
psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 -f tests/e2e/fixtures/seed.sql

node tests/e2e/fixtures/mock-idp.mjs &
MOCK_IDP_PID=$!
trap 'kill "${MOCK_IDP_PID}" 2>/dev/null || true' EXIT

# The production-like server is expected to be built and started by the caller
# (CI job) so this script stays fast and focused on the smoke evidence.
npm --prefix tests/e2e exec -- playwright test --grep @smoke --project=chromium

echo "browser smoke evidence complete for head ${E2E_HEAD_SHA}"
