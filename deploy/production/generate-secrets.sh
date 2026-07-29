#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUPABASE_DIR="${SCRIPT_DIR}/runtime/supabase"
SUPABASE_ENV="${EDUTALENT_SUPABASE_ENV:-${SUPABASE_DIR}/.env}"
SUPABASE_COMPOSE="${SUPABASE_DIR}/docker-compose.yml"
SUPABASE_COMPOSE_BACKUP="${SUPABASE_DIR}/docker-compose.yml.edutalent-backup"
APP_ENV="${EDUTALENT_APP_ENV:-${SCRIPT_DIR}/.env.edutalent}"
APP_TEMPLATE="${SCRIPT_DIR}/.env.edutalent.example"
UPSTREAM_ENV_BACKUP="${SUPABASE_DIR}/.env.old"
UPSTREAM_COMPOSE_BACKUP="${SUPABASE_COMPOSE}.old"

usage() {
  cat <<'USAGE'
Usage: generate-secrets.sh

Creates fresh local production credentials. The command refuses to overwrite an
existing Supabase .env because key rotation is a separate staged operation.
External AI provider credentials remain operator-owned and are never generated.
USAGE
}

case "${1:-}" in
  "") ;;
  -h|--help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

for command in openssl sed grep awk node; do
  command -v "${command}" >/dev/null 2>&1 || {
    echo "Required command not found: ${command}" >&2
    exit 1
  }
done

node_major="$(node --version 2>/dev/null | sed 's/^v//' | awk -F. '{print $1}')"
[[ "${node_major}" =~ ^[0-9]+$ && "${node_major}" -ge 16 ]] || {
  echo "Node.js 16 or newer is required for offline-safe asymmetric key generation." >&2
  exit 1
}

[[ -f "${SUPABASE_COMPOSE}" ]] || {
  echo "Supabase runtime is missing. Run production-bootstrap first." >&2
  exit 1
}
[[ -f "${SUPABASE_DIR}/.env.example" ]] || { echo "Missing official Supabase .env.example" >&2; exit 1; }
[[ -f "${SUPABASE_DIR}/utils/generate-keys.sh" ]] || { echo "Missing official Supabase key generator" >&2; exit 1; }
[[ -f "${SUPABASE_DIR}/utils/add-new-auth-keys.sh" ]] || { echo "Missing official Supabase asymmetric key generator" >&2; exit 1; }
[[ -f "${APP_ENV}" ]] || cp "${APP_TEMPLATE}" "${APP_ENV}"
chmod 600 "${APP_ENV}"

if [[ -e "${SUPABASE_ENV}" ]]; then
  echo "${SUPABASE_ENV} already exists; refusing to rotate production secrets." >&2
  exit 1
fi

read_env() {
  local key="$1"
  awk -F= -v key="${key}" '$1 == key { sub(/^[^=]*=/, ""); print; exit }' "${APP_ENV}"
}

set_env() {
  local file="$1" key="$2" value="$3" escaped
  escaped="$(printf '%s' "${value}" | sed 's/[&|]/\\&/g')"
  if grep -q "^${key}=" "${file}"; then
    sed -i.bak "s|^${key}=.*$|${key}=${escaped}|" "${file}"
  else
    printf '%s=%s\n' "${key}" "${value}" >> "${file}"
  fi
  rm -f "${file}.bak"
}

app_domain="$(read_env APP_DOMAIN)"
supabase_domain="$(read_env SUPABASE_DOMAIN)"
admin_domain="$(read_env ADMIN_DOMAIN)"
for pair in "APP_DOMAIN:${app_domain}" "SUPABASE_DOMAIN:${supabase_domain}" "ADMIN_DOMAIN:${admin_domain}"; do
  key="${pair%%:*}"
  value="${pair#*:}"
  if [[ -z "${value}" || "${value}" == *example.invalid* ]]; then
    echo "Set ${key} in ${APP_ENV} before production-init." >&2
    exit 1
  fi
done

cp "${SUPABASE_COMPOSE}" "${SUPABASE_COMPOSE_BACKUP}"
initialization_complete=false
cleanup_upstream_backups() {
  rm -f -- "${UPSTREAM_ENV_BACKUP}" "${UPSTREAM_COMPOSE_BACKUP}"
}
cleanup_partial_initialization() {
  cleanup_upstream_backups
  if [[ "${initialization_complete}" != true ]]; then
    rm -f "${SUPABASE_ENV}"
    if [[ -f "${SUPABASE_COMPOSE_BACKUP}" ]]; then
      mv "${SUPABASE_COMPOSE_BACKUP}" "${SUPABASE_COMPOSE}"
    fi
  fi
}
trap cleanup_partial_initialization EXIT

cp "${SUPABASE_DIR}/.env.example" "${SUPABASE_ENV}"
chmod 600 "${SUPABASE_ENV}"

(
  cd "${SUPABASE_DIR}"
  sh utils/generate-keys.sh --update-env >/dev/null
  sh utils/add-new-auth-keys.sh --update-env >/dev/null
)
cleanup_upstream_backups

set_env "${SUPABASE_ENV}" SUPABASE_PUBLIC_URL "https://${supabase_domain}"
set_env "${SUPABASE_ENV}" API_EXTERNAL_URL "https://${supabase_domain}/auth/v1"
set_env "${SUPABASE_ENV}" SITE_URL "https://${app_domain}"
set_env "${SUPABASE_ENV}" ADDITIONAL_REDIRECT_URLS "https://${app_domain}"
set_env "${SUPABASE_ENV}" DISABLE_SIGNUP "true"
set_env "${SUPABASE_ENV}" ENABLE_EMAIL_SIGNUP "true"
set_env "${SUPABASE_ENV}" ENABLE_EMAIL_AUTOCONFIRM "false"
set_env "${SUPABASE_ENV}" ENABLE_ANONYMOUS_USERS "false"
set_env "${SUPABASE_ENV}" ENABLE_PHONE_SIGNUP "false"
set_env "${SUPABASE_ENV}" ENABLE_PHONE_AUTOCONFIRM "false"
set_env "${SUPABASE_ENV}" FUNCTIONS_VERIFY_JWT "true"
set_env "${SUPABASE_ENV}" OPENAI_API_KEY ""
set_env "${SUPABASE_ENV}" SMTP_HOST "disabled.invalid"
set_env "${SUPABASE_ENV}" SMTP_PORT "25"
set_env "${SUPABASE_ENV}" SMTP_USER "disabled"
set_env "${SUPABASE_ENV}" SMTP_PASS "disabled"
set_env "${SUPABASE_ENV}" SMTP_ADMIN_EMAIL "noreply@${app_domain}"
set_env "${SUPABASE_ENV}" SMTP_SENDER_NAME "EduTalent"
set_env "${SUPABASE_ENV}" DASHBOARD_USERNAME "edutalent-admin"
set_env "${SUPABASE_ENV}" STORAGE_TENANT_ID "$(openssl rand -hex 12)"
set_env "${SUPABASE_ENV}" POOLER_TENANT_ID "$(openssl rand -hex 12)"
set_env "${SUPABASE_ENV}" GLOBAL_S3_BUCKET "edutalent-storage"
set_env "${SUPABASE_ENV}" REGION "local"

set_env "${APP_ENV}" DATABASE_APP_PASSWORD "$(openssl rand -hex 32)"
set_env "${APP_ENV}" QDRANT_API_KEY "$(openssl rand -hex 32)"
set_env "${APP_ENV}" AI_GATEWAY_INTERNAL_TOKEN "$(openssl rand -hex 32)"
chmod 600 "${SUPABASE_ENV}" "${APP_ENV}"

cleanup_upstream_backups
initialization_complete=true
rm -f "${SUPABASE_COMPOSE_BACKUP}"
trap - EXIT

echo "Generated local production secrets without printing their values."
echo "Supabase environment: ${SUPABASE_ENV}"
echo "EduTalent environment: ${APP_ENV}"
echo "Dashboard host: https://${admin_domain}"
