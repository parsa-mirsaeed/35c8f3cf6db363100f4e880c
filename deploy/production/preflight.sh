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
    raise SystemExit(f"Docker Compose 2.24.4 or newer is required; found {sys.argv[1]}")
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

for file in "${SUPABASE_ENV}" "${APP_ENV}"; do
  permissions="$(stat -c '%a' "${file}" 2>/dev/null || stat -f '%Lp' "${file}")"
  [[ "${permissions}" =~ ^[0-6]00$ ]] || {
    echo "${file} must not be group/world readable (current mode ${permissions})." >&2
    exit 1
  }
done

read_env() {
  local file="$1" key="$2"
  awk -F= -v key="${key}" '$1 == key { sub(/^[^=]*=/, ""); print; exit }' "${file}"
}

for key in APP_DOMAIN SUPABASE_DOMAIN ADMIN_DOMAIN ADMIN_ALLOWED_CIDRS TLS_CERT_FILE TLS_KEY_FILE DATABASE_APP_USER DATABASE_APP_PASSWORD QDRANT_API_KEY AI_GATEWAY_INTERNAL_TOKEN AI_GATEWAY_MODE EMBEDDING_PROFILE EMBEDDING_MODEL EMBEDDING_VECTOR_SIZE QDRANT_COLLECTION QDRANT_VECTOR_SIZE; do
  value="$(read_env "${APP_ENV}" "${key}")"
  if [[ -z "${value}" || "${value}" == *example.invalid* || "${value}" == *replace* ]]; then
    echo "${key} is missing or contains a placeholder in ${APP_ENV}." >&2
    exit 1
  fi
done

if grep -q '^AI_GATEWAY_DEFAULT_SCHOOL_ID=' "${APP_ENV}"; then
  echo "AI_GATEWAY_DEFAULT_SCHOOL_ID is forbidden; AI requests must carry the authoritative school ID." >&2
  exit 1
fi
if grep -Eq '^AI_ALLOWED_(EMBEDDING|LLM)_BASE_URLS=' "${APP_ENV}"; then
  echo "Operator-configurable AI allowlists are forbidden; provider origins are fixed in code." >&2
  exit 1
fi

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
        raise SystemExit(f"ADMIN_ALLOWED_CIDRS contains invalid IP/CIDR {entry!r}: {error}") from error
    if network.prefixlen == 0:
        raise SystemExit(f"ADMIN_ALLOWED_CIDRS must not permit the entire internet: {entry}")
PY

database_app_user="$(read_env "${APP_ENV}" DATABASE_APP_USER)"
database_app_password="$(read_env "${APP_ENV}" DATABASE_APP_PASSWORD)"
[[ "${database_app_user}" =~ ^[a-z_][a-z0-9_]{0,62}$ ]] || { echo "DATABASE_APP_USER must be a lowercase PostgreSQL identifier." >&2; exit 1; }
[[ "${database_app_user}" != "postgres" ]] || { echo "DATABASE_APP_USER must not be postgres." >&2; exit 1; }
[[ "${database_app_password}" =~ ^[A-Za-z0-9._~-]{32,128}$ ]] || { echo "DATABASE_APP_PASSWORD must be 32-128 URL-safe characters." >&2; exit 1; }

ai_gateway_token="$(read_env "${APP_ENV}" AI_GATEWAY_INTERNAL_TOKEN)"
[[ "${ai_gateway_token}" =~ ^[A-Za-z0-9._~-]{32,128}$ ]] || { echo "AI_GATEWAY_INTERNAL_TOKEN must be 32-128 URL-safe characters." >&2; exit 1; }

ai_mode="$(read_env "${APP_ENV}" AI_GATEWAY_MODE)"
embedding_profile="$(read_env "${APP_ENV}" EMBEDDING_PROFILE)"
embedding_model="$(read_env "${APP_ENV}" EMBEDDING_MODEL)"
embedding_size="$(read_env "${APP_ENV}" EMBEDDING_VECTOR_SIZE)"
qdrant_collection="$(read_env "${APP_ENV}" QDRANT_COLLECTION)"
qdrant_size="$(read_env "${APP_ENV}" QDRANT_VECTOR_SIZE)"
case "${embedding_profile}" in
  local-bge-v1)
    [[ "${ai_mode}|${embedding_model}|${embedding_size}|${qdrant_collection}|${qdrant_size}" == "offline|BAAI/bge-small-en-v1.5|384|edutalent_materials_local_v1|384" ]] || {
      echo "local-bge-v1 profile values do not match the registry." >&2
      exit 1
    }
    [[ "$(read_env "${APP_ENV}" AI_EMBEDDING_BASE_URL)" == "http://embedding:80/v1/" ]] || {
      echo "Offline mode must use internal TEI through the AI gateway." >&2
      exit 1
    }
    ;;
  openai-v1)
    [[ "${ai_mode}|${embedding_model}|${embedding_size}|${qdrant_collection}|${qdrant_size}" == "connected|text-embedding-3-small|1536|edutalent_openai_v1|1536" ]] || {
      echo "openai-v1 profile values do not match the registry." >&2
      exit 1
    }
    [[ "$(read_env "${APP_ENV}" AI_EMBEDDING_BASE_URL)" == "https://api.openai.com/v1/" ]] || {
      echo "Connected embeddings must use the approved OpenAI origin." >&2
      exit 1
    }
    [[ "$(read_env "${APP_ENV}" AI_LLM_BASE_URL)" == "https://api.deepseek.com/v1/" ]] || {
      echo "Connected LLM requests must use the approved LLM origin." >&2
      exit 1
    }
    for key in OPENAI_API_KEY LLM_API_KEY; do
      value="$(read_env "${APP_ENV}" "${key}")"
      [[ "${#value}" -ge 24 && "${value}" != *replace* ]] || { echo "Connected mode requires ${key}." >&2; exit 1; }
    done
    ;;
  *) echo "Unsupported EMBEDDING_PROFILE=${embedding_profile}" >&2; exit 1 ;;
esac

for key in POSTGRES_PASSWORD JWT_SECRET SUPABASE_PUBLISHABLE_KEY SUPABASE_SECRET_KEY DASHBOARD_PASSWORD SECRET_KEY_BASE REALTIME_DB_ENC_KEY VAULT_ENC_KEY PG_META_CRYPTO_KEY; do
  value="$(read_env "${SUPABASE_ENV}" "${key}")"
  if [[ -z "${value}" || "${value}" == *your-* || "${value}" == *insecure* ]]; then
    echo "${key} is missing or contains an upstream placeholder." >&2
    exit 1
  fi
done
postgres_password="$(read_env "${SUPABASE_ENV}" POSTGRES_PASSWORD)"
[[ "${database_app_password}" != "${postgres_password}" ]] || { echo "The application and PostgreSQL bootstrap credentials must differ." >&2; exit 1; }

for key in DISABLE_SIGNUP ENABLE_EMAIL_SIGNUP ENABLE_ANONYMOUS_USERS ENABLE_PHONE_SIGNUP FUNCTIONS_VERIFY_JWT; do
  value="$(read_env "${SUPABASE_ENV}" "${key}")"
  case "${key}:${value}" in
    DISABLE_SIGNUP:true|ENABLE_EMAIL_SIGNUP:true|ENABLE_ANONYMOUS_USERS:false|ENABLE_PHONE_SIGNUP:false|FUNCTIONS_VERIFY_JWT:true) ;;
    *) echo "Unsafe Supabase setting ${key}=${value}" >&2; exit 1 ;;
  esac
done

cert_file="$(read_env "${APP_ENV}" TLS_CERT_FILE)"
key_file="$(read_env "${APP_ENV}" TLS_KEY_FILE)"
[[ "${cert_file}" == /* && "${key_file}" == /* ]] || { echo "TLS_CERT_FILE and TLS_KEY_FILE must be absolute paths." >&2; exit 1; }
[[ -r "${cert_file}" && -r "${key_file}" ]] || { echo "TLS certificate or private key is not readable." >&2; exit 1; }
key_permissions="$(stat -c '%a' "${key_file}" 2>/dev/null || stat -f '%Lp' "${key_file}")"
[[ "${key_permissions}" =~ ^[0-6]00$ ]] || { echo "TLS private key must not be group/world readable (current mode ${key_permissions})." >&2; exit 1; }

openssl x509 -in "${cert_file}" -noout >/dev/null
openssl pkey -in "${key_file}" -noout >/dev/null
openssl x509 -in "${cert_file}" -checkend 1209600 -noout >/dev/null || { echo "TLS certificate expires in less than 14 days." >&2; exit 1; }
for domain in "${app_domain}" "${supabase_domain}" "${admin_domain}"; do
  openssl x509 -in "${cert_file}" -checkhost "${domain}" -noout >/dev/null || { echo "TLS certificate does not cover ${domain}." >&2; exit 1; }
done
cert_public="$(openssl x509 -in "${cert_file}" -pubkey -noout | openssl pkey -pubin -outform der | openssl dgst -sha256)"
key_public="$(openssl pkey -in "${key_file}" -pubout -outform der | openssl dgst -sha256)"
[[ "${cert_public}" == "${key_public}" ]] || { echo "TLS certificate and private key do not match." >&2; exit 1; }

rendered="$(mktemp)"
trap 'rm -f "${rendered}"' EXIT
export EDUTALENT_PRODUCTION_DIR="${SCRIPT_DIR}"
profile_args=()
if [[ "${embedding_profile}" == "local-bge-v1" ]]; then
  profile_args=(--profile local-embedding)
fi
docker compose \
  --project-name edutalent \
  --project-directory "${SUPABASE_DIR}" \
  --env-file "${SUPABASE_ENV}" \
  --env-file "${APP_ENV}" \
  -f "${SUPABASE_DIR}/docker-compose.yml" \
  -f "${OVERLAY}" \
  "${profile_args[@]}" \
  config --format json > "${rendered}"
python3 "${SCRIPT_DIR}/validate-rendered-compose.py" "${rendered}" "${host_cpus}"

echo "Production preflight passed for Docker host capacity ${host_cpus} CPUs."
