#!/usr/bin/env bash
set -euo pipefail
umask 077

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${1:-}"
PLATFORM="${EDUTALENT_APPLIANCE_PLATFORM:-linux/amd64}"
SIGNING_MODE="${EDUTALENT_APPLIANCE_SIGNING_MODE:-ephemeral}"
CREATE_ARCHIVE="${EDUTALENT_APPLIANCE_CREATE_ARCHIVE:-false}"
GIT_SHA="${EDUTALENT_APPLIANCE_GIT_SHA:-${GITHUB_SHA:-}}"

if [[ "${GITHUB_REF_TYPE:-}" == "tag" && "${GITHUB_REF_NAME:-}" == v* ]]; then
  SIGNING_MODE=keyless
  CREATE_ARCHIVE=true
fi

[[ -n "${VERSION}" ]] || { echo "Usage: build.sh <version>" >&2; exit 2; }
[[ "${VERSION}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$ ]] || {
  echo "Version must be a safe tag and path component." >&2
  exit 2
}
if [[ -z "${GIT_SHA}" ]]; then
  GIT_SHA="$(git -C "${ROOT_DIR}" rev-parse HEAD)"
fi
[[ "${GIT_SHA}" =~ ^[0-9a-f]{40}$ ]] || { echo "A full git SHA is required." >&2; exit 1; }
case "${PLATFORM}" in
  linux/amd64) ARCH=amd64 ;;
  linux/arm64) ARCH=arm64 ;;
  *) echo "Unsupported platform: ${PLATFORM}" >&2; exit 2 ;;
esac

for command in docker jq python3 gzip tar sha256sum openssl syft cosign node; do
  command -v "${command}" >/dev/null 2>&1 || {
    echo "Required connected-builder command not found: ${command}" >&2
    exit 1
  }
done
docker compose version >/dev/null
docker buildx version >/dev/null

SOURCE_PRODUCTION_DIR="${ROOT_DIR}/deploy/production"
TLS_DIR="$(mktemp -d)"
TEMP_DIR="$(mktemp -d)"
PRODUCTION_DIR="${TEMP_DIR}/production"
SUPABASE_DIR="${PRODUCTION_DIR}/runtime/supabase"
APP_ENV="${PRODUCTION_DIR}/.env.edutalent"
SUPABASE_ENV="${SUPABASE_DIR}/.env"
BUNDLE_NAME="edutalent-appliance-${VERSION}-${ARCH}"
BUNDLE_DIR="${ROOT_DIR}/dist/${BUNDLE_NAME}"
APP_BUILD_TAG="edutalent-appliance-build/app:${VERSION}-${ARCH}"
TOOLS_BUILD_TAG="edutalent-appliance-build/tools:${VERSION}-${ARCH}"
MODEL_LOCK="${ROOT_DIR}/deploy/appliance/model.lock.json"
MODEL_OUTPUT="${BUNDLE_DIR}/models/local-bge-v1"

cleanup() {
  rm -rf "${TLS_DIR}" "${TEMP_DIR}"
}
trap cleanup EXIT

rm -rf "${BUNDLE_DIR}"
mkdir -p \
  "${BUNDLE_DIR}/images" \
  "${BUNDLE_DIR}/manifests" \
  "${BUNDLE_DIR}/models" \
  "${BUNDLE_DIR}/sbom/images" \
  "${BUNDLE_DIR}/signatures" \
  "${BUNDLE_DIR}/provenance"

python3 "${ROOT_DIR}/scripts/appliance/stage_production.py" \
  --source "${SOURCE_PRODUCTION_DIR}" \
  --destination "${PRODUCTION_DIR}"
bash "${PRODUCTION_DIR}/edutalent-production" bootstrap
unreadable_runtime_file="$(find "${SUPABASE_DIR}" -type f ! -perm -004 -print -quit)"
unsearchable_runtime_directory="$(find "${SUPABASE_DIR}" -type d ! -perm -005 -print -quit)"
[[ -z "${unreadable_runtime_file}" ]] || {
  echo "Pinned Supabase runtime file is not container-readable: ${unreadable_runtime_file}" >&2
  exit 1
}
[[ -z "${unsearchable_runtime_directory}" ]] || {
  echo "Pinned Supabase runtime directory is not container-searchable: ${unsearchable_runtime_directory}" >&2
  exit 1
}
cp "${PRODUCTION_DIR}/.env.edutalent.example" "${APP_ENV}"
chmod 600 "${APP_ENV}"
openssl req -x509 -newkey rsa:3072 -sha256 -nodes -days 30 \
  -subj '/CN=app.appliance.invalid' \
  -addext 'subjectAltName=DNS:app.appliance.invalid,DNS:supabase.appliance.invalid,DNS:admin.appliance.invalid' \
  -keyout "${TLS_DIR}/privkey.pem" \
  -out "${TLS_DIR}/fullchain.pem" >/dev/null 2>&1
python3 - "${APP_ENV}" "${TLS_DIR}" <<'PY'
import sys
from pathlib import Path

env_path = Path(sys.argv[1])
tls_dir = Path(sys.argv[2]).resolve()
replacements = {
    "APP_DOMAIN": "app.appliance.invalid",
    "SUPABASE_DOMAIN": "supabase.appliance.invalid",
    "ADMIN_DOMAIN": "admin.appliance.invalid",
    "ADMIN_ALLOWED_CIDRS": "127.0.0.1/32",
    "TLS_CERT_FILE": str(tls_dir / "fullchain.pem"),
    "TLS_KEY_FILE": str(tls_dir / "privkey.pem"),
    "EMBEDDING_PROFILE": "local-bge-v1",
    "AI_GATEWAY_MODE": "offline",
}
lines = []
seen = set()
for line in env_path.read_text(encoding="utf-8").splitlines():
    key = line.split("=", 1)[0]
    if key in replacements:
        lines.append(f"{key}={replacements[key]}")
        seen.add(key)
    else:
        lines.append(line)
for key, value in replacements.items():
    if key not in seen:
        lines.append(f"{key}={value}")
env_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
bash "${PRODUCTION_DIR}/edutalent-production" init

cache_args=()
if [[ -n "${ACTIONS_RUNTIME_TOKEN:-}" && -n "${ACTIONS_CACHE_URL:-}" ]]; then
  cache_scope="${EDUTALENT_BUILD_CACHE_SCOPE:-edutalent-appliance-${ARCH}}"
  cache_args=(
    --cache-from "type=gha,scope=${cache_scope}"
    --cache-to "type=gha,mode=max,scope=${cache_scope}"
  )
fi

docker buildx build \
  --load \
  --platform "${PLATFORM}" \
  --progress=plain \
  --target runtime \
  "${cache_args[@]}" \
  --tag "${APP_BUILD_TAG}" \
  "${ROOT_DIR}"
docker buildx build \
  --load \
  --platform "${PLATFORM}" \
  --progress=plain \
  --file "${ROOT_DIR}/Dockerfile.appliance-tools" \
  --tag "${TOOLS_BUILD_TAG}" \
  "${ROOT_DIR}"

python3 "${ROOT_DIR}/scripts/appliance/fetch_model.py" \
  --lock "${MODEL_LOCK}" \
  --output "${MODEL_OUTPUT}" \
  --cache "${TEMP_DIR}/huggingface"

export EDUTALENT_IMAGE="edutalent-appliance-build/app"
export EDUTALENT_TAG="${VERSION}-${ARCH}"
export EDUTALENT_PRODUCTION_DIR="${PRODUCTION_DIR}"
COMPOSE_JSON="${TEMP_DIR}/compose.json"
docker compose \
  --project-name edutalent \
  --project-directory "${SUPABASE_DIR}" \
  --env-file "${SUPABASE_ENV}" \
  --env-file "${APP_ENV}" \
  -f "${SUPABASE_DIR}/docker-compose.yml" \
  -f "${PRODUCTION_DIR}/compose.production.yaml" \
  --profile local-embedding \
  --profile edge-functions \
  config --format json > "${COMPOSE_JSON}"

jq -r '.services | to_entries[] | select(.value.image != null) | [.key, .value.image] | @tsv' \
  "${COMPOSE_JSON}" | sort -u > "${TEMP_DIR}/service-images.tsv"
[[ -s "${TEMP_DIR}/service-images.tsv" ]] || { echo "Rendered production image inventory is empty." >&2; exit 1; }

safe_name() {
  printf '%s' "$1" | sed -E 's#^[^/]+/##; s#[^A-Za-z0-9._-]+#-#g; s#^-+|-+$##g' | tr '[:upper:]' '[:lower:]'
}

source_digest() {
  local image="$1" value
  value="$(docker image inspect --format '{{if .RepoDigests}}{{index .RepoDigests 0}}{{else}}{{.Id}}{{end}}' "${image}")"
  if [[ "${value}" == *@sha256:* ]]; then
    printf '%s\n' "${value##*@}"
  else
    printf '%s\n' "${value}"
  fi
}

save_component() {
  local component="$1" source_ref="$2" services_csv="$3"
  local digest digest_short local_tag archive_rel archive_path sbom_rel sbom_path
  if [[ "${source_ref}" != "${APP_BUILD_TAG}" && "${source_ref}" != "${TOOLS_BUILD_TAG}" ]]; then
    docker pull --platform "${PLATFORM}" "${source_ref}" >/dev/null
  fi
  digest="$(source_digest "${source_ref}")"
  [[ "${digest}" =~ ^sha256:[0-9a-f]{64}$ ]] || { echo "Invalid digest for ${source_ref}: ${digest}" >&2; exit 1; }
  digest_short="${digest#sha256:}"
  digest_short="${digest_short:0:16}"
  local_tag="edutalent-offline/${component}:${VERSION}-${ARCH}-${digest_short}"
  docker tag "${source_ref}" "${local_tag}"
  archive_rel="images/${component}.tar.gz"
  archive_path="${BUNDLE_DIR}/${archive_rel}"
  docker save "${local_tag}" | gzip -1 > "${archive_path}"
  sbom_rel="sbom/images/${component}.spdx.json"
  sbom_path="${BUNDLE_DIR}/${sbom_rel}"
  syft "${local_tag}" -o "spdx-json=${sbom_path}" >/dev/null
  python3 - "${TEMP_DIR}/image-records.jsonl" \
    "${component}" "${services_csv}" "${source_ref}" "${digest}" \
    "${local_tag}" "${archive_rel}" "${PLATFORM}" "${sbom_rel}" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
record = {
    "component": sys.argv[2],
    "services": [item for item in sys.argv[3].split(",") if item],
    "source_ref": sys.argv[4],
    "source_digest": sys.argv[5],
    "local_tag": sys.argv[6],
    "archive": sys.argv[7],
    "platform": sys.argv[8],
    "sbom": sys.argv[9],
}
with path.open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True) + "\n")
PY
}

: > "${TEMP_DIR}/image-records.jsonl"
mapfile -t UNIQUE_IMAGES < <(cut -f2 "${TEMP_DIR}/service-images.tsv" | sort -u)
for source_ref in "${UNIQUE_IMAGES[@]}"; do
  services_csv="$(awk -F '\t' -v image="${source_ref}" '$2 == image { print $1 }' "${TEMP_DIR}/service-images.tsv" | sort | paste -sd, -)"
  if [[ "${source_ref}" == "${APP_BUILD_TAG}" ]]; then
    component="edutalent-runtime"
  else
    component="$(safe_name "${source_ref%%:*}")"
  fi
  save_component "${component}" "${source_ref}" "${services_csv}"
done
save_component "appliance-tools" "${TOOLS_BUILD_TAG}" ""

python3 - "${TEMP_DIR}/image-records.jsonl" "${BUNDLE_DIR}/manifests/images.json" <<'PY'
import json
import sys
from pathlib import Path

records = [json.loads(line) for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines() if line]
components = [record["component"] for record in records]
if len(components) != len(set(components)):
    raise SystemExit(f"duplicate component names: {components}")
Path(sys.argv[2]).write_text(
    json.dumps({"schema_version": 1, "images": sorted(records, key=lambda row: row["component"])}, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
PY

mkdir -p "${BUNDLE_DIR}/deploy"
python3 "${ROOT_DIR}/scripts/appliance/stage_production.py" \
  --source "${PRODUCTION_DIR}" \
  --destination "${BUNDLE_DIR}/deploy/production" \
  --mode release
python3 "${ROOT_DIR}/scripts/appliance/patch_production_command.py" \
  "${BUNDLE_DIR}/deploy/production/edutalent-production"
mkdir -p "${BUNDLE_DIR}/scripts/appliance"
cp "${ROOT_DIR}/scripts/appliance/release_manifest.py" "${BUNDLE_DIR}/scripts/appliance/release_manifest.py"
cp "${ROOT_DIR}/scripts/appliance/patch_production_command.py" "${BUNDLE_DIR}/scripts/appliance/patch_production_command.py"
cp "${ROOT_DIR}/deploy/appliance/edutalent-appliance" "${BUNDLE_DIR}/edutalent-appliance"
cp "${ROOT_DIR}/deploy/appliance/README.md" "${BUNDLE_DIR}/README.md"
cp "${ROOT_DIR}/deploy/appliance/THIRD_PARTY_NOTICES.md" "${BUNDLE_DIR}/THIRD_PARTY_NOTICES.md"
cp "${MODEL_LOCK}" "${BUNDLE_DIR}/models/model.lock.json"
if [[ -f "${ROOT_DIR}/LICENSE" ]]; then
  cp "${ROOT_DIR}/LICENSE" "${BUNDLE_DIR}/LICENSE"
fi
chmod 0755 "${BUNDLE_DIR}/edutalent-appliance" "${BUNDLE_DIR}/deploy/production/edutalent-production"

python3 "${ROOT_DIR}/scripts/appliance/locked_compose.py" \
  --service-images "${TEMP_DIR}/service-images.tsv" \
  --images "${BUNDLE_DIR}/manifests/images.json" \
  --output "${BUNDLE_DIR}/manifests/compose.locked.yaml"

python3 - "${BUNDLE_DIR}/provenance/release-builder.json" "${GIT_SHA}" "${PLATFORM}" \
  "$(docker version --format '{{.Server.Version}}')" \
  "$(docker buildx version | awk '{print $2}')" \
  "$(syft version -o json | jq -r '.version')" \
  "$(cosign version --json | jq -r '.gitVersion // .git_version // "unknown"')" <<'PY'
import json
import sys
from pathlib import Path

output, git_sha, platform, docker_version, buildx_version, syft_version, cosign_version = sys.argv[1:]
Path(output).write_text(
    json.dumps(
        {
            "schema_version": 1,
            "builder": "scripts/appliance/build.sh",
            "git_sha": git_sha,
            "platform": platform,
            "tools": {
                "docker": docker_version,
                "buildx": buildx_version,
                "syft": syft_version,
                "cosign": cosign_version,
            },
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY

syft "dir:${BUNDLE_DIR}" --exclude './images/**' --exclude './signatures/**' \
  -o "spdx-json=${BUNDLE_DIR}/sbom/release-files.spdx.json" >/dev/null
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" generate \
  --bundle "${BUNDLE_DIR}" \
  --version "${VERSION}" \
  --git-sha "${GIT_SHA}" \
  --platform "${PLATFORM}" \
  --signing-mode "${SIGNING_MODE}" \
  --images "${BUNDLE_DIR}/manifests/images.json" \
  --model-lock "${MODEL_LOCK}"
bash "${ROOT_DIR}/scripts/appliance/sign_release.sh" "${BUNDLE_DIR}" "${SIGNING_MODE}"
case "${SIGNING_MODE}" in
  ephemeral)
    export EDUTALENT_APPLIANCE_ALLOW_EPHEMERAL_SIGNATURES=true
    ;;
  keyless)
    if [[ "${GITHUB_ACTIONS:-false}" == "true" ]]; then
      : "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required for CI keyless verification}"
      : "${GITHUB_REF:?GITHUB_REF is required for CI keyless verification}"
      export EDUTALENT_APPLIANCE_TRUSTED_OIDC_ISSUER="https://token.actions.githubusercontent.com"
      export EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP="^https://github.com/${GITHUB_REPOSITORY}/\\.github/workflows/air-gapped-appliance\\.yml@${GITHUB_REF}$"
    fi
    ;;
esac
bash "${BUNDLE_DIR}/edutalent-appliance" verify

sign_external_payload() {
  local payload="$1" temp_signing
  case "${SIGNING_MODE}" in
    ephemeral)
      temp_signing="$(mktemp -d)"
      COSIGN_PASSWORD='' cosign generate-key-pair --output-key-prefix "${temp_signing}/verification" >/dev/null
      COSIGN_PASSWORD='' cosign sign-blob --yes \
        --key "${temp_signing}/verification.key" \
        --output-signature "${payload}.sig" \
        "${payload}" >/dev/null
      cp "${temp_signing}/verification.pub" "${payload}.pub"
      cosign verify-blob --key "${payload}.pub" --signature "${payload}.sig" "${payload}" >/dev/null
      rm -rf "${temp_signing}"
      ;;
    keyless)
      cosign sign-blob --yes --bundle "${payload}.sigstore.json" "${payload}" >/dev/null
      ;;
    *) echo "Unsupported signing mode: ${SIGNING_MODE}" >&2; exit 2 ;;
  esac
}

if [[ "${CREATE_ARCHIVE}" == "true" ]]; then
  archive="${ROOT_DIR}/dist/${BUNDLE_NAME}.tar.gz"
  rm -f "${archive}" "${archive}.part-"* "${archive}.sig" "${archive}.pub" "${archive}.sigstore.json"
  tar --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner \
    -C "${ROOT_DIR}/dist" -cf - "${BUNDLE_NAME}" | gzip -n -9 > "${archive}"
  if (( $(stat -c %s "${archive}") > 1900000000 )); then
    split -b 1800m -d -a 3 "${archive}" "${archive}.part-"
    rm -f "${archive}"
    checksum_file="${ROOT_DIR}/dist/${BUNDLE_NAME}.parts.SHA256SUMS"
    (
      cd "${ROOT_DIR}/dist"
      sha256sum "${BUNDLE_NAME}.tar.gz.part-"*
    ) > "${checksum_file}"
    for part in "${archive}.part-"*; do
      sign_external_payload "${part}"
    done
    sign_external_payload "${checksum_file}"
  else
    checksum_file="${archive}.SHA256SUMS"
    (
      cd "${ROOT_DIR}/dist"
      sha256sum "${BUNDLE_NAME}.tar.gz"
    ) > "${checksum_file}"
    sign_external_payload "${archive}"
    sign_external_payload "${checksum_file}"
  fi
fi

echo "Air-gapped appliance prepared at ${BUNDLE_DIR}"
