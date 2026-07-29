#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PIN_FILE="${SCRIPT_DIR}/SUPABASE_UPSTREAM"
RUNTIME_DIR="${SCRIPT_DIR}/runtime/supabase"
PERMISSIONS_HELPER="${SCRIPT_DIR}/normalize-supabase-runtime-permissions.sh"
UPSTREAM_URL="https://github.com/supabase/supabase.git"

usage() {
  cat <<'USAGE'
Usage: bootstrap-supabase.sh

Fetches only the official Supabase Docker deployment at the commit recorded in
SUPABASE_UPSTREAM. Run this on a connected preparation host. Production startup
never fetches floating upstream content.

This command is deliberately non-destructive. It never replaces an existing
runtime because that directory can contain bind-mounted production state. A
version change requires the documented controlled-upgrade procedure.
USAGE
}

case "${1:-}" in
  "") ;;
  -h|--help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

for command in git awk mktemp python3 find chmod grep; do
  command -v "${command}" >/dev/null 2>&1 || {
    echo "Required command not found: ${command}" >&2
    exit 1
  }
done

[[ -f "${PIN_FILE}" ]] || { echo "Missing ${PIN_FILE}" >&2; exit 1; }
commit="$(awk 'NF { print $1; exit }' "${PIN_FILE}")"
[[ "${commit}" =~ ^[0-9a-f]{40}$ ]] || { echo "Invalid pinned Supabase commit: ${commit}" >&2; exit 1; }

configure_ci_database_volume() {
  [[ "${GITHUB_ACTIONS:-false}" == "true" ]] || return 0

  local compose_file="${RUNTIME_DIR}/docker-compose.yml"
  python3 - "${compose_file}" <<'PY'
from pathlib import Path
import sys

compose_file = Path(sys.argv[1])
text = compose_file.read_text(encoding="utf-8")
bind_mount = "      - ./volumes/db/data:/var/lib/postgresql/data:Z"
named_mount = "      - edutalent-ci-supabase-db-data:/var/lib/postgresql/data"

if text.count(bind_mount) == 1:
    text = text.replace(bind_mount, named_mount, 1)
elif text.count(named_mount) != 1:
    raise SystemExit(
        "Pinned Supabase Compose no longer contains the expected PostgreSQL data mount"
    )

volume_declaration = "  edutalent-ci-supabase-db-data:\n"
if volume_declaration not in text:
    marker = "\nvolumes:\n"
    marker_index = text.rfind(marker)
    if marker_index < 0:
        raise SystemExit("Pinned Supabase Compose is missing its top-level volumes section")
    insert_at = marker_index + len(marker)
    text = text[:insert_at] + volume_declaration + text[insert_at:]

compose_file.write_text(text, encoding="utf-8")
PY

  # The CI-only named volume is deleted by `docker compose down --volumes`.
  # Removing the unused bind directory prevents container-owned PostgreSQL files
  # from poisoning the self-hosted runner's next actions/checkout clean.
  rm -rf "${RUNTIME_DIR}/volumes/db/data"
  echo "Configured Docker-managed PostgreSQL storage for GitHub Actions."
}

if [[ -e "${RUNTIME_DIR}" ]]; then
  if [[ -f "${RUNTIME_DIR}/UPSTREAM_COMMIT" && -f "${RUNTIME_DIR}/docker-compose.yml" ]]; then
    actual="$(awk 'NF { print $1; exit }' "${RUNTIME_DIR}/UPSTREAM_COMMIT")"
    if [[ "${actual}" == "${commit}" ]]; then
      bash "${PERMISSIONS_HELPER}" "${RUNTIME_DIR}"
      configure_ci_database_volume
      echo "Official Supabase runtime is already prepared at ${RUNTIME_DIR}"
      echo "Pinned upstream commit: ${commit}"
      exit 0
    fi
    echo "Existing Supabase runtime uses commit ${actual}; repository pin is ${commit}." >&2
  else
    echo "Existing Supabase runtime at ${RUNTIME_DIR} is incomplete or unrecognized." >&2
  fi
  echo "Refusing to replace it automatically. Use a reviewed backup, upgrade, and rollback procedure." >&2
  exit 1
fi

mkdir -p "$(dirname "${RUNTIME_DIR}")"
temporary="$(mktemp -d)"
trap 'rm -rf "${temporary}"' EXIT

# Fetch the exact immutable commit, then materialize only the docker directory.
git -C "${temporary}" init --quiet
git -C "${temporary}" remote add origin "${UPSTREAM_URL}"
git -C "${temporary}" config advice.detachedHead false
git -C "${temporary}" fetch --quiet --depth 1 origin "${commit}"
git -C "${temporary}" sparse-checkout init --cone
git -C "${temporary}" sparse-checkout set docker
git -C "${temporary}" checkout --quiet --detach FETCH_HEAD

actual="$(git -C "${temporary}" rev-parse HEAD)"
[[ "${actual}" == "${commit}" ]] || {
  echo "Fetched Supabase commit ${actual}, expected ${commit}" >&2
  exit 1
}

cp -a "${temporary}/docker" "${RUNTIME_DIR}"
printf '%s\n' "${commit}" > "${RUNTIME_DIR}/UPSTREAM_COMMIT"
bash "${PERMISSIONS_HELPER}" "${RUNTIME_DIR}"
configure_ci_database_volume

echo "Prepared official Supabase Docker runtime at ${RUNTIME_DIR}"
echo "Pinned upstream commit: ${commit}"
