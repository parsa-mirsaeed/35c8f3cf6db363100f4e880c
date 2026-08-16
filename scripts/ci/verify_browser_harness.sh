#!/usr/bin/env bash
# PR-12 harness self-check: the harness must be pinned, present, and internally
# consistent before any browser journey is trusted. Fails closed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

test -f tests/e2e/package.json
test -f tests/e2e/playwright.config.ts
test -f tests/e2e/fixtures/seed.sql
test -f tests/e2e/fixtures/mock-idp.mjs
test -f tests/e2e/fixtures/network-policy.ts
test -f tests/e2e/fixtures/console-guard.ts
test -d tests/e2e/specs

! grep -R --line-number -E '"(@playwright/test|@axe-core/playwright|playwright)"[[:space:]]*:[[:space:]]*"(latest|\*|\^|~)' tests/e2e/package.json
grep -q '"@playwright/test": "1.49.1"' tests/e2e/package.json
grep -q '"@axe-core/playwright": "4.10.2"' tests/e2e/package.json
grep -q 'E2E_ALLOWED_ORIGINS' tests/e2e/playwright.config.ts

grep -q "alg: 'ES256'" tests/e2e/fixtures/mock-idp.mjs
grep -q "kid: KID" tests/e2e/fixtures/mock-idp.mjs
grep -q "email," tests/e2e/fixtures/mock-idp.mjs
grep -q 'SUPABASE_JWT_ISSUER="http://127.0.0.1:9100/auth/v1"' scripts/ci/run_browser_e2e.sh

grep -q 'dx bundle --web --release --package web' scripts/ci/run_browser_e2e.sh
grep -q 'target/dx/web/release/web' scripts/ci/run_browser_e2e.sh

echo "browser harness verification passed"
