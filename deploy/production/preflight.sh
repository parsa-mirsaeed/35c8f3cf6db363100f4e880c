#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUPABASE_DIR="${SCRIPT_DIR}/runtime/supabase"
SUPABASE_ENV="${SUPABASE_DIR}/.env"
APP_ENV="${SCRIPT_DIR}/.env.edutalent"
OVERLAY="${SCRIPT_DIR}/compose.production.yaml"
PIN_FILE="${SCRIPT_DIR}/SUPABASE_UPSTREAM"

for command in docker python3 openssl awk grep mktemp sed stat; do
  command -v "${command}" >/dev/null 2>&1 || {
    echo "Required command not found: ${command}" >&2
    exit 1
  }
done

docker compose version >/dev/null
compose_version="$(docker compose version --short | sed -E 's/^v//; s/[^0-9.].*$//')"
python3 - "${compose_version}" <<'PY'
import sys

minimum = (2, 24, 4)
try:
    current = tuple(int(part) for part in sys.argv[1].split(".")[:3])
except ValueError as error:
    raise SystemExit(f"Unable to parse Docker Compose version: {sys.argv[1]}") from error
current = current + (0,) * (3 - len(current))
if current < minimum:
    raise SystemExit(
        f"Docker Compose 2.24.4 or newer is required; found {sys.argv[1]}"
    )
PY
host_cpus="$(docker info --format '{{.NCPU}}')"
[[ "${host_cpus}" =~ ^[0-9]+([.][0-9]+)?$ ]] || {
  echo "Unable to determine Docker host CPU capacity: ${host_cpus}" >&2
  exit 1
}

[[ -f "${SUPABASE_DIR}/docker-compose.yml" ]] || { echo "Run production-bootstrap first." >&2; exit 1; }
[[ -f "${SUPABASE_ENV}" ]] || { echo "Run production-init first." >&2; exit 1; }
[[ -f "${APP_ENV}" ]] || { echo "Missing ${APP_ENV}" >&2; exit 1; }

expected="$(awk 'NF { print $1; exit }' "${PIN_FILE}")"
actual="$(awk 'NF { print $1; exit }' "${SUPABASE_DIR}/UPSTREAM_COMMIT")"
[[ "${actual}" == "${expected}" ]] || {
  echo "Supabase runtime commit ${actual} does not match pinned commit ${expected}." >&2
  exit 1
}

permissions="$(stat -c '%a' "${SUPABASE_ENV}" 2>/dev/null || stat -f '%Lp' "${SUPABASE_ENV}")"
[[ "${permissions}" =~ ^[0-6]00$ ]] || {
  echo "${SUPABASE_ENV} must not be group/world readable (current mode ${permissions})." >&2
  exit 1
}
permissions="$(stat -c '%a' "${APP_ENV}" 2>/dev/null || stat -f '%Lp' "${APP_ENV}")"
[[ "${permissions}" =~ ^[0-6]00$ ]] || {
  echo "${APP_ENV} must not be group/world readable (current mode ${permissions})." >&2
  exit 1
}

read_env() {
  local file="$1" key="$2"
  awk -F= -v key="${key}" '$1 == key { sub(/^[^=]*=/, ""); print; exit }' "${file}"
}

for key in APP_DOMAIN SUPABASE_DOMAIN ADMIN_DOMAIN ADMIN_ALLOWED_CIDRS TLS_CERT_FILE TLS_KEY_FILE DATABASE_APP_USER DATABASE_APP_PASSWORD QDRANT_API_KEY; do
  value="$(read_env "${APP_ENV}" "${key}")"
  if [[ -z "${value}" || "${value}" == *example.invalid* || "${value}" == *replace* ]]; then
    echo "${key} is missing or contains a placeholder in ${APP_ENV}." >&2
    exit 1
  fi
done

app_domain="$(read_env "${APP_ENV}" APP_DOMAIN)"
supabase_domain="$(read_env "${APP_ENV}" SUPABASE_DOMAIN)"
admin_domain="$(read_env "${APP_ENV}" ADMIN_DOMAIN)"
if [[ "${app_domain}" == "${supabase_domain}" || "${app_domain}" == "${admin_domain}" || "${supabase_domain}" == "${admin_domain}" ]]; then
  echo "APP_DOMAIN, SUPABASE_DOMAIN, and ADMIN_DOMAIN must be distinct." >&2
  exit 1
fi

admin_cidrs="$(read_env "${APP_ENV}" ADMIN_ALLOWED_CIDRS)"
python3 - "${admin_cidrs}" <<'PY'
import ipaddress
import sys

entries = sys.argv[1].split()
if not entries:
    raise SystemExit("ADMIN_ALLOWED_CIDRS must contain at least one IP address or CIDR")

for entry in entries:
    try:
        network = ipaddress.ip_network(entry, strict=False)
    except ValueError as error:
        raise SystemExit(
            f"ADMIN_ALLOWED_CIDRS contains invalid IP/CIDR {entry!r}: {error}"
        ) from error
    if network.prefixlen == 0:
        raise SystemExit(
            f"ADMIN_ALLOWED_CIDRS must not permit the entire internet: {entry}"
        )
PY

database_app_user="$(read_env "${APP_ENV}" DATABASE_APP_USER)"
database_app_password="$(read_env "${APP_ENV}" DATABASE_APP_PASSWORD)"
[[ "${database_app_user}" =~ ^[a-z_][a-z0-9_]{0,62}$ ]] || {
  echo "DATABASE_APP_USER must be a lowercase PostgreSQL identifier." >&2
  exit 1
}
[[ "${database_app_user}" != "postgres" ]] || {
  echo "DATABASE_APP_USER must not be postgres." >&2
  exit 1
}
[[ "${database_app_password}" =~ ^[A-Za-z0-9._~-]{32,128}$ ]] || {
  echo "DATABASE_APP_PASSWORD must be 32-128 URL-safe characters." >&2
  exit 1
}

for key in POSTGRES_PASSWORD JWT_SECRET SUPABASE_PUBLISHABLE_KEY SUPABASE_SECRET_KEY DASHBOARD_PASSWORD SECRET_KEY_BASE REALTIME_DB_ENC_KEY VAULT_ENC_KEY PG_META_CRYPTO_KEY; do
  value="$(read_env "${SUPABASE_ENV}" "${key}")"
  if [[ -z "${value}" || "${value}" == *your-* || "${value}" == *insecure* ]]; then
    echo "${key} is missing or contains an upstream placeholder." >&2
    exit 1
  fi
done
postgres_password="$(read_env "${SUPABASE_ENV}" POSTGRES_PASSWORD)"
[[ "${database_app_password}" != "${postgres_password}" ]] || {
  echo "The application and PostgreSQL bootstrap credentials must differ." >&2
  exit 1
}

for key in DISABLE_SIGNUP ENABLE_EMAIL_SIGNUP ENABLE_ANONYMOUS_USERS ENABLE_PHONE_SIGNUP FUNCTIONS_VERIFY_JWT; do
  value="$(read_env "${SUPABASE_ENV}" "${key}")"
  case "${key}:${value}" in
    DISABLE_SIGNUP:true|ENABLE_EMAIL_SIGNUP:true|ENABLE_ANONYMOUS_USERS:false|ENABLE_PHONE_SIGNUP:false|FUNCTIONS_VERIFY_JWT:true) ;;
    *) echo "Unsafe Supabase setting ${key}=${value}" >&2; exit 1 ;;
  esac
done

cert_file="$(read_env "${APP_ENV}" TLS_CERT_FILE)"
key_file="$(read_env "${APP_ENV}" TLS_KEY_FILE)"
[[ "${cert_file}" == /* && "${key_file}" == /* ]] || {
  echo "TLS_CERT_FILE and TLS_KEY_FILE must be absolute paths." >&2
  exit 1
}
[[ -r "${cert_file}" && -r "${key_file}" ]] || {
  echo "TLS certificate or private key is not readable." >&2
  exit 1
}
key_permissions="$(stat -c '%a' "${key_file}" 2>/dev/null || stat -f '%Lp' "${key_file}")"
[[ "${key_permissions}" =~ ^[0-6]00$ ]] || {
  echo "TLS private key must not be group/world readable (current mode ${key_permissions})." >&2
  exit 1
}

openssl x509 -in "${cert_file}" -noout >/dev/null
openssl pkey -in "${key_file}" -noout >/dev/null
openssl x509 -in "${cert_file}" -checkend 1209600 -noout >/dev/null || {
  echo "TLS certificate expires in less than 14 days." >&2
  exit 1
}
for domain in "${app_domain}" "${supabase_domain}" "${admin_domain}"; do
  openssl x509 -in "${cert_file}" -checkhost "${domain}" -noout >/dev/null || {
    echo "TLS certificate does not cover ${domain}." >&2
    exit 1
  }
done
cert_public="$(openssl x509 -in "${cert_file}" -pubkey -noout | openssl pkey -pubin -outform der | openssl dgst -sha256)"
key_public="$(openssl pkey -in "${key_file}" -pubout -outform der | openssl dgst -sha256)"
[[ "${cert_public}" == "${key_public}" ]] || { echo "TLS certificate and private key do not match." >&2; exit 1; }

rendered="$(mktemp)"
trap 'rm -f "${rendered}"' EXIT
export EDUTALENT_PRODUCTION_DIR="${SCRIPT_DIR}"
docker compose \
  --project-name edutalent \
  --project-directory "${SUPABASE_DIR}" \
  --env-file "${SUPABASE_ENV}" \
  --env-file "${APP_ENV}" \
  -f "${SUPABASE_DIR}/docker-compose.yml" \
  -f "${OVERLAY}" \
  config --format json > "${rendered}"
python3 "${SCRIPT_DIR}/validate-rendered-compose.py" "${rendered}" "${host_cpus}"

echo "Production preflight passed for Docker host capacity ${host_cpus} CPUs."
