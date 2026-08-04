#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
real_rustfmt="$(rustup which rustfmt)"
lock_file="${RUNNER_TEMP:-/tmp}/edutalent-pr01-rustfmt.lock"
exec 9>"${lock_file}"
flock 9

check_args=()
for arg in "$@"; do
  if [[ "${arg}" == "--check" ]]; then
    check_args=(--check)
    break
  fi
done

exec "${real_rustfmt}" --edition 2021 "${check_args[@]}" \
  "${root}/packages/api/src/server_functions/dashboard_functions.rs" \
  "${root}/packages/api/src/repositories/authorized_assignment_repository_tests.rs"
