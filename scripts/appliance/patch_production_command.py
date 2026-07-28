#!/usr/bin/env python3
"""Patch a copied production command to accept the appliance lock override."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

OLD = '''  local -a profile_args=()
  local profile
  profile="$(read_env "${APP_ENV}" EMBEDDING_PROFILE)"
  case "${profile}" in
    local-bge-v1) profile_args=(--profile local-embedding) ;;
    openai-v1) ;;
    *) echo "Unsupported EMBEDDING_PROFILE=${profile}" >&2; exit 1 ;;
  esac

  docker compose \\
    --project-name edutalent \\
    --project-directory "${SUPABASE_DIR}" \\
    --env-file "${SUPABASE_ENV}" \\
    --env-file "${APP_ENV}" \\
    -f "${SUPABASE_DIR}/docker-compose.yml" \\
    -f "${OVERLAY}" \\
    "${profile_args[@]}" \\
    "$@"
'''

NEW = '''  local -a profile_args=()
  local -a override_args=()
  local profile override
  profile="$(read_env "${APP_ENV}" EMBEDDING_PROFILE)"
  case "${profile}" in
    local-bge-v1) profile_args=(--profile local-embedding) ;;
    openai-v1) ;;
    *) echo "Unsupported EMBEDDING_PROFILE=${profile}" >&2; exit 1 ;;
  esac

  override="${EDUTALENT_COMPOSE_OVERRIDE:-}"
  if [[ -n "${override}" ]]; then
    [[ -f "${override}" ]] || {
      echo "Appliance Compose override is missing: ${override}" >&2
      exit 1
    }
    override="$(cd "$(dirname "${override}")" && pwd)/$(basename "${override}")"
    override_args=(-f "${override}")
  fi

  docker compose \\
    --project-name edutalent \\
    --project-directory "${SUPABASE_DIR}" \\
    --env-file "${SUPABASE_ENV}" \\
    --env-file "${APP_ENV}" \\
    -f "${SUPABASE_DIR}/docker-compose.yml" \\
    -f "${OVERLAY}" \\
    "${override_args[@]}" \\
    "${profile_args[@]}" \\
    "$@"
'''


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    content = args.path.read_text(encoding="utf-8")
    if content.count(OLD) != 1:
        raise RuntimeError("production compose block did not match exactly once")
    patched = content.replace(OLD, NEW)
    if patched.count("EDUTALENT_COMPOSE_OVERRIDE") != 1:
        raise RuntimeError("appliance override patch was not applied exactly once")
    args.path.write_text(patched, encoding="utf-8")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError) as error:
        print(f"production command patch failed: {error}", file=sys.stderr)
        raise SystemExit(1)
