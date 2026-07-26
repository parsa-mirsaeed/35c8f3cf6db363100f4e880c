#!/usr/bin/env bash
set -euo pipefail

echo "build-for-render.sh is retained as a compatibility wrapper."
echo "The Dockerfile now builds directly from source; delegating to the unified image build."
exec bash "$(dirname "$0")/edutalent" build render
