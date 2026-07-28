#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
for file in \
  "${ROOT_DIR}/scripts/appliance/build.sh" \
  "${ROOT_DIR}/scripts/appliance/sign_release.sh" \
  "${ROOT_DIR}/scripts/appliance/offline_smoke.sh" \
  "${ROOT_DIR}/deploy/appliance/edutalent-appliance"; do
  bash -n "${file}"
done
python3 -m py_compile \
  "${ROOT_DIR}/scripts/appliance/fetch_model.py" \
  "${ROOT_DIR}/scripts/appliance/patch_production_command.py" \
  "${ROOT_DIR}/scripts/appliance/release_manifest.py"

python3 - "${ROOT_DIR}/deploy/appliance/model.lock.json" <<'PY'
import json
import re
import sys

lock = json.load(open(sys.argv[1], encoding="utf-8"))
assert lock["schema_version"] == 1
assert re.fullmatch(r"[0-9a-f]{40}", lock["revision"])
assert re.fullmatch(r"[0-9a-f]{64}", lock["primary_weight"]["sha256"])
assert lock["profile"] == "local-bge-v1"
assert lock["dimensions"] == 384
assert lock["primary_weight"]["path"] == "model.safetensors"
assert "pytorch_model.bin" not in lock["allow_patterns"]
PY

test "$(cat "${ROOT_DIR}/scripts/appliance/requirements.txt")" = "huggingface_hub==0.34.4"
grep -Fq 'pull_policy: never' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq '/models/local-bge-v1' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'EDUTALENT_COMPOSE_OVERRIDE' "${ROOT_DIR}/scripts/appliance/patch_production_command.py"
grep -Fq 'docker load' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'cosign sign-blob' "${ROOT_DIR}/scripts/appliance/sign_release.sh"
grep -Fq 'syft' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'linux/amd64,linux/arm64' "${ROOT_DIR}/.github/workflows/air-gapped-appliance.yml"
if grep -R --line-number --exclude=validate.sh -E '(^|[/:])latest([@:]|$)' \
  "${ROOT_DIR}/Dockerfile.appliance-tools" \
  "${ROOT_DIR}/deploy/appliance" \
  "${ROOT_DIR}/scripts/appliance"; then
  echo "Air-gapped release definitions contain a forbidden latest tag." >&2
  exit 1
fi

fixture="$(mktemp -d)"
trap 'rm -rf "${fixture}"' EXIT
cp "${ROOT_DIR}/deploy/production/edutalent-production" "${fixture}/edutalent-production"
python3 "${ROOT_DIR}/scripts/appliance/patch_production_command.py" \
  "${fixture}/edutalent-production"
grep -Fq 'EDUTALENT_COMPOSE_OVERRIDE' "${fixture}/edutalent-production"
bash -n "${fixture}/edutalent-production"

mkdir -p \
  "${fixture}/bundle/images" \
  "${fixture}/bundle/manifests" \
  "${fixture}/bundle/models/test-model" \
  "${fixture}/bundle/sbom/images" \
  "${fixture}/bundle/scripts/appliance"
printf 'fixture image\n' > "${fixture}/bundle/images/fixture.tar.gz"
printf '{}\n' > "${fixture}/bundle/sbom/images/fixture.spdx.json"
printf 'fixture weight\n' > "${fixture}/bundle/models/test-model/model.safetensors"
weight_sha="$(sha256sum "${fixture}/bundle/models/test-model/model.safetensors" | awk '{print $1}')"
cat > "${fixture}/model.lock.json" <<JSON
{
  "schema_version": 1,
  "profile": "test-model",
  "repository": "fixture/model",
  "revision": "0123456789abcdef0123456789abcdef01234567",
  "served_model_name": "fixture/model",
  "dimensions": 3,
  "license": "MIT",
  "primary_weight": {"path": "model.safetensors", "sha256": "${weight_sha}"}
}
JSON
cat > "${fixture}/bundle/models/test-model/MODEL_METADATA.json" <<JSON
{"revision":"0123456789abcdef0123456789abcdef01234567","dimensions":3}
JSON
sha256sum "${fixture}/bundle/models/test-model/model.safetensors" \
  | sed "s#${fixture}/bundle/models/test-model/##" \
  > "${fixture}/bundle/models/test-model/MODEL_SHA256SUMS"
cat > "${fixture}/bundle/manifests/images.json" <<'JSON'
{
  "schema_version": 1,
  "images": [{
    "component": "fixture",
    "services": ["fixture"],
    "source_ref": "registry.invalid/fixture:v1",
    "source_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "local_tag": "edutalent-offline/fixture:v1-amd64-aaaaaaaaaaaaaaaa",
    "archive": "images/fixture.tar.gz",
    "platform": "linux/amd64",
    "sbom": "sbom/images/fixture.spdx.json"
  }]
}
JSON
cp "${ROOT_DIR}/scripts/appliance/release_manifest.py" \
  "${fixture}/bundle/scripts/appliance/release_manifest.py"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" generate \
  --bundle "${fixture}/bundle" \
  --version fixture \
  --git-sha 0123456789abcdef0123456789abcdef01234567 \
  --platform linux/amd64 \
  --images "${fixture}/bundle/manifests/images.json" \
  --model-lock "${fixture}/model.lock.json"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle"
printf 'untracked\n' > "${fixture}/bundle/untracked.txt"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "untracked appliance payload was accepted" >&2
  exit 1
fi
rm -f "${fixture}/bundle/untracked.txt"
printf 'tampered\n' >> "${fixture}/bundle/images/fixture.tar.gz"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "tampered appliance fixture was accepted" >&2
  exit 1
fi

echo "Air-gapped appliance definitions and integrity regression tests passed."
