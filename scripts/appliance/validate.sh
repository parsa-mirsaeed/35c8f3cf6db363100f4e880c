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
  "${ROOT_DIR}/scripts/appliance/release_manifest.py" \
  "${ROOT_DIR}/scripts/appliance/stage_production.py"

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
grep -Fq 'if service == "embedding"' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'SOURCE_PRODUCTION_DIR=' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'PRODUCTION_DIR="${TEMP_DIR}/production"' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'stage_production.py' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'EDUTALENT_COMPOSE_OVERRIDE' "${ROOT_DIR}/scripts/appliance/patch_production_command.py"
grep -Fq 'docker load' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq "'{{.Id}}'" "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'install destination must be empty' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'bash "${destination}/edutalent-appliance" verify' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'release.signing_mode' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'EDUTALENT_APPLIANCE_TRUSTED_OIDC_ISSUER' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
! grep -Fq 'signatures/policy.json' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'cosign sign-blob' "${ROOT_DIR}/scripts/appliance/sign_release.sh"
! grep -Fq 'policy.json' "${ROOT_DIR}/scripts/appliance/sign_release.sh"
grep -Fq 'syft' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fxq 'deploy/appliance' "${ROOT_DIR}/.dockerignore"
grep -Fxq 'scripts/appliance' "${ROOT_DIR}/.dockerignore"
grep -Fxq 'docs' "${ROOT_DIR}/.dockerignore"

air_workflow="${ROOT_DIR}/.github/workflows/air-gapped-appliance.yml"
package_workflow="${ROOT_DIR}/.github/workflows/package.yml"
mirror_workflow="${ROOT_DIR}/.github/workflows/mirror-final-proof.yml"
grep -Fq 'runs-on: ubuntu-24.04-arm' "${air_workflow}"
grep -Fq 'platform: linux/arm64' "${air_workflow}"
grep -Fq "if: github.event_name != 'pull_request'" "${air_workflow}"
grep -Fq "inputs.publish && github.ref == 'refs/heads/main'" "${air_workflow}"
grep -Fq 'Build custom images natively for arm64' "${air_workflow}"
grep -Fq "if: github.event_name != 'pull_request'" "${package_workflow}"
grep -Fq 'Verify and serialize complete exact-head proof' "${mirror_workflow}"
grep -Fq "github.event.pull_request.draft == false" "${mirror_workflow}"
grep -Fq -- "--event pull_request" "${mirror_workflow}"
grep -Fq "gh workflow run package.yml" "${mirror_workflow}"
grep -Fq "gh workflow run air-gapped-appliance.yml" "${mirror_workflow}"
if grep -Fq "gh workflow run production-foundation.yml" "${mirror_workflow}"; then
  echo "Production Foundation must be an exact-head prerequisite, not a duplicate dispatch." >&2
  exit 1
fi
package_line="$(grep -n "gh workflow run package.yml" "${mirror_workflow}" | cut -d: -f1)"
appliance_line="$(grep -n "gh workflow run air-gapped-appliance.yml" "${mirror_workflow}" | cut -d: -f1)"
test "${package_line}" -lt "${appliance_line}"
if grep -Fq 'setup-qemu-action' "${air_workflow}"; then
  echo "Air-gapped workflow must use native architecture runners, not QEMU." >&2
  exit 1
fi
if grep -R --line-number --exclude=validate.sh -E '(^|[/:])latest([@:]|$)' \
  "${ROOT_DIR}/Dockerfile.appliance-tools" \
  "${ROOT_DIR}/deploy/appliance" \
  "${ROOT_DIR}/scripts/appliance"; then
  echo "Air-gapped release definitions contain a forbidden latest tag." >&2
  exit 1
fi

fixture="$(mktemp -d)"
trap 'rm -rf "${fixture}"' EXIT

mkdir -p "${fixture}/production-source/runtime/supabase" "${fixture}/production-source/runtime/keep"
printf 'definition\n' > "${fixture}/production-source/keep.txt"
printf 'live app state\n' > "${fixture}/production-source/.env.edutalent"
printf 'live supabase state\n' > "${fixture}/production-source/runtime/supabase/.env"
printf 'other runtime definition\n' > "${fixture}/production-source/runtime/keep/definition.txt"
python3 "${ROOT_DIR}/scripts/appliance/stage_production.py" \
  --source "${fixture}/production-source" \
  --destination "${fixture}/production-staged"
test -f "${fixture}/production-staged/keep.txt"
test -f "${fixture}/production-staged/runtime/keep/definition.txt"
test ! -e "${fixture}/production-staged/.env.edutalent"
test ! -e "${fixture}/production-staged/runtime/supabase"

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
python3 - "${fixture}/bundle/images/fixture.tar.gz" <<'PY'
import hashlib
import io
import json
import sys
import tarfile
from pathlib import Path

config = json.dumps(
    {
        "architecture": "amd64",
        "config": {},
        "os": "linux",
        "rootfs": {"diff_ids": [], "type": "layers"},
    },
    sort_keys=True,
    separators=(",", ":"),
).encode()
digest = hashlib.sha256(config).hexdigest()
manifest = json.dumps(
    [
        {
            "Config": f"{digest}.json",
            "Layers": [],
            "RepoTags": ["edutalent-offline/fixture:v1-amd64-aaaaaaaaaaaaaaaa"],
        }
    ],
    separators=(",", ":"),
).encode()
with tarfile.open(Path(sys.argv[1]), "w:gz") as archive:
    for name, data in ((f"{digest}.json", config), ("manifest.json", manifest)):
        info = tarfile.TarInfo(name)
        info.mode = 0o644
        info.mtime = 0
        info.size = len(data)
        archive.addfile(info, io.BytesIO(data))
PY
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
  --signing-mode ephemeral \
  --images "${fixture}/bundle/manifests/images.json" \
  --model-lock "${fixture}/model.lock.json"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle"

mkdir -p "${fixture}/bundle/deploy/production/runtime/supabase"
printf 'generated app state\n' > "${fixture}/bundle/deploy/production/.env.edutalent"
printf 'generated supabase state\n' > "${fixture}/bundle/deploy/production/runtime/supabase/.env"
chmod 0600 \
  "${fixture}/bundle/deploy/production/.env.edutalent" \
  "${fixture}/bundle/deploy/production/runtime/supabase/.env"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle"

chmod 0755 "${fixture}/bundle/sbom/images/fixture.spdx.json"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "signed file mode change was accepted" >&2
  exit 1
fi
chmod 0644 "${fixture}/bundle/sbom/images/fixture.spdx.json"

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
