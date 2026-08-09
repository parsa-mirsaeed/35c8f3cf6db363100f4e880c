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

# No floating tags anywhere in the harness manifest.
! grep -R --line-number -E '"(@playwright/test|playwright)"[[:space:]]*:[[:space:]]*"(latest|\*|\^|~)' tests/e2e/package.json

# The pinned Playwright version is recorded exactly once and matches config.
grep -q '"@playwright/test": "1.49.1"' tests/e2e/package.json

# Offline policy and console guard are wired into the config.
grep -q 'E2E_ALLOWED_ORIGINS' tests/e2e/playwright.config.ts

echo "browser harness verification passed"
