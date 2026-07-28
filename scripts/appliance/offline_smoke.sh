#!/usr/bin/env bash
set -euo pipefail
umask 077

bundle="${1:-}"
[[ -n "${bundle}" ]] || { echo "Usage: offline_smoke.sh <bundle-dir>" >&2; exit 2; }
bundle="$(cd "${bundle}" && pwd)"
appliance="${bundle}/edutalent-appliance"
production_dir="${bundle}/deploy/production"
app_env="${production_dir}/.env.edutalent"
supabase_env="${production_dir}/runtime/supabase/.env"
tls_dir="${production_dir}/.offline-smoke-tls"
started=false

cleanup() {
  if [[ "${started}" == true ]]; then
    "${appliance}" stop >/dev/null 2>&1 || true
  fi
  rm -rf "${tls_dir}"
  rm -f "${app_env}" "${supabase_env}" "${production_dir}/runtime/supabase/docker-compose.yml.edutalent-backup"
}
trap cleanup EXIT

"${appliance}" verify

mapfile -t local_tags < <(python3 - "${bundle}/manifests/release-manifest.json" <<'PY'
import json
import sys
for image in json.load(open(sys.argv[1], encoding="utf-8"))["images"]:
    print(image["local_tag"])
PY
)
for tag in "${local_tags[@]}"; do
  docker image rm --force "${tag}" >/dev/null 2>&1 || true
done
for tag in "${local_tags[@]}"; do
  if docker image inspect "${tag}" >/dev/null 2>&1; then
    echo "Local appliance tag survived pre-load removal: ${tag}" >&2
    exit 1
  fi
done

"${appliance}" load
for tag in "${local_tags[@]}"; do
  docker image inspect "${tag}" >/dev/null
done

mkdir -p "${tls_dir}"
openssl req -x509 -newkey rsa:3072 -sha256 -nodes -days 30 \
  -subj '/CN=app.offline.internal' \
  -addext 'subjectAltName=DNS:app.offline.internal,DNS:supabase.offline.internal,DNS:admin.offline.internal' \
  -keyout "${tls_dir}/privkey.pem" \
  -out "${tls_dir}/fullchain.pem" >/dev/null 2>&1
chmod 600 "${tls_dir}/privkey.pem"
cp "${production_dir}/.env.edutalent.example" "${app_env}"
chmod 600 "${app_env}"
python3 - "${app_env}" "${tls_dir}" <<'PY'
import sys
from pathlib import Path

env = Path(sys.argv[1])
tls = Path(sys.argv[2]).resolve()
values = {
    "APP_DOMAIN": "app.offline.internal",
    "SUPABASE_DOMAIN": "supabase.offline.internal",
    "ADMIN_DOMAIN": "admin.offline.internal",
    "ADMIN_ALLOWED_CIDRS": "127.0.0.1/32",
    "TLS_CERT_FILE": str(tls / "fullchain.pem"),
    "TLS_KEY_FILE": str(tls / "privkey.pem"),
    "EMBEDDING_PROFILE": "local-bge-v1",
    "AI_GATEWAY_MODE": "offline",
}
lines = []
seen = set()
for line in env.read_text(encoding="utf-8").splitlines():
    key = line.split("=", 1)[0]
    if key in values:
        lines.append(f"{key}={values[key]}")
        seen.add(key)
    else:
        lines.append(line)
for key, value in values.items():
    if key not in seen:
        lines.append(f"{key}={value}")
env.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
rm -f "${supabase_env}"
"${appliance}" init

started_at="$(date --iso-8601=seconds)"
"${appliance}" start
started=true
"${appliance}" checks

manifest_tags="$(printf '%s\n' "${local_tags[@]}" | sort -u)"
running_images="$(docker ps \
  --filter label=com.docker.compose.project=edutalent \
  --format '{{.Image}}' | sort -u)"
while IFS= read -r image; do
  [[ -n "${image}" ]] || continue
  grep -Fxq "${image}" <<<"${manifest_tags}" || {
    echo "Running container used an image outside the appliance manifest: ${image}" >&2
    exit 1
  }
done <<<"${running_images}"

ended_at="$(date --iso-8601=seconds)"
pull_events="$(docker events --since "${started_at}" --until "${ended_at}" \
  --filter type=image --filter event=pull --format '{{json .}}' 2>/dev/null || true)"
[[ -z "${pull_events}" ]] || {
  echo "Registry pull events occurred during offline startup:" >&2
  echo "${pull_events}" >&2
  exit 1
}

grep -Fq 'pull_policy: never' "${bundle}/manifests/compose.locked.yaml"
grep -Fq '/models/local-bge-v1' "${bundle}/manifests/compose.locked.yaml"
echo "Complete appliance started from loaded local images with registry pulls disabled."
