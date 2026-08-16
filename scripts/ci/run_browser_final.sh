#!/usr/bin/env bash
# PR-12 Tier-2 final acceptance on an exact head (full-validation label only):
# complete critical journeys, desktop + mobile, English and Persian/RTL,
# accessibility scans, and the offline network policy.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

export E2E_GREP="@final|@smoke"
exec bash scripts/ci/run_browser_e2e.sh
