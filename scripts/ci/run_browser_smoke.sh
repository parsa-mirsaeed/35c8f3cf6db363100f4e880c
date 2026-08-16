#!/usr/bin/env bash
# PR-12 Tier-1 browser smoke on an exact head: login, role landing, one
# critical changed-feature path, authorization denial, logout. One engine.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

export E2E_GREP="@smoke"
exec bash scripts/ci/run_browser_e2e.sh
