#!/usr/bin/env bash
set -euo pipefail

runtime_dir="${1:-}"
[[ -n "${runtime_dir}" && -d "${runtime_dir}" ]] || {
  echo "Usage: normalize-supabase-runtime-permissions.sh <runtime-directory>" >&2
  exit 2
}

if find "${runtime_dir}" -type l -print -quit | grep -q .; then
  echo "Supabase runtime contains a symlink; refusing permission normalization." >&2
  exit 1
fi
if find "${runtime_dir}" ! -type d ! -type f -print -quit | grep -q .; then
  echo "Supabase runtime contains a non-regular entry; refusing permission normalization." >&2
  exit 1
fi

# The pinned runtime contains immutable public definitions only. Secrets are
# generated later and retain mode 0600. Restore read/search access that a
# strict caller umask may remove while preserving upstream executable bits.
chmod -R a+rX,go-w "${runtime_dir}"

unreadable_file="$(find "${runtime_dir}" -type f ! -perm -004 -print -quit)"
unsearchable_directory="$(find "${runtime_dir}" -type d ! -perm -005 -print -quit)"
[[ -z "${unreadable_file}" ]] || {
  echo "Supabase runtime file is not container-readable: ${unreadable_file}" >&2
  exit 1
}
[[ -z "${unsearchable_directory}" ]] || {
  echo "Supabase runtime directory is not container-searchable: ${unsearchable_directory}" >&2
  exit 1
}
