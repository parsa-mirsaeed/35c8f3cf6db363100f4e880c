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
  "${ROOT_DIR}/scripts/appliance/stage_production.py" \
  "${ROOT_DIR}/scripts/appliance/locked_compose.py"

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
grep -Fq 'pull_policy: never' "${ROOT_DIR}/scripts/appliance/locked_compose.py"
grep -Fq '/models/local-bge-v1' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'locked_compose.py' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'SOURCE_PRODUCTION_DIR=' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'PRODUCTION_DIR="${TEMP_DIR}/production"' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'stage_production.py' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq -- '--mode release' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'cleanup_upstream_backups' "${ROOT_DIR}/deploy/production/generate-secrets.sh"
grep -Fq 'UPSTREAM_ENV_BACKUP="${SUPABASE_DIR}/.env.old"' "${ROOT_DIR}/deploy/production/generate-secrets.sh"
grep -Fq 'UPSTREAM_COMPOSE_BACKUP="${SUPABASE_COMPOSE}.old"' "${ROOT_DIR}/deploy/production/generate-secrets.sh"
if grep -Fq 'rm -f "${APP_ENV}"' "${ROOT_DIR}/scripts/appliance/build.sh"; then
  echo "Appliance builder must not delete source-tree production environments." >&2
  exit 1
fi
grep -Fq -- '--signing-mode "${SIGNING_MODE}"' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'EDUTALENT_COMPOSE_OVERRIDE' "${ROOT_DIR}/scripts/appliance/patch_production_command.py"
grep -Fq 'docker load' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq "'{{.Id}}'" "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'install destination must be empty' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'bash "${destination}/edutalent-appliance" verify' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'release.signing_mode' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'EDUTALENT_APPLIANCE_TRUSTED_OIDC_ISSUER' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'EDUTALENT_APPLIANCE_ALLOW_EPHEMERAL_SIGNATURES' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
! grep -Fq 'signatures/policy.json' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
! grep -Fq 'MUTABLE_INSTALLATION_FILES' "${ROOT_DIR}/scripts/appliance/release_manifest.py"
grep -Fq 'SCAN_CHUNK_SIZE' "${ROOT_DIR}/scripts/appliance/release_manifest.py"
grep -Fq 'actual_mode' "${ROOT_DIR}/scripts/appliance/release_manifest.py"
grep -Fq 'EDUTALENT_APPLIANCE_STATE_DIR' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'XDG_STATE_HOME' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'state-dir)' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'EDUTALENT_APPLIANCE_GIT_SHA' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'EDUTALENT_APP_ENV' "${ROOT_DIR}/deploy/production/edutalent-production"
grep -Fq 'EDUTALENT_SUPABASE_ENV' "${ROOT_DIR}/deploy/production/generate-secrets.sh"
grep -Fq 'cosign sign-blob' "${ROOT_DIR}/scripts/appliance/sign_release.sh"
! grep -Fq 'policy.json' "${ROOT_DIR}/scripts/appliance/sign_release.sh"
grep -Fq 'syft' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'EDUTALENT_APPLIANCE_FUNCTIONS_DIR' "${ROOT_DIR}/deploy/appliance/edutalent-appliance"
grep -Fq 'edutalent-appliance-db-data' "${ROOT_DIR}/scripts/appliance/locked_compose.py"
grep -Fq 'edutalent-appliance-storage-data' "${ROOT_DIR}/scripts/appliance/locked_compose.py"
grep -Fq 'edutalent-appliance-studio-snippets' "${ROOT_DIR}/scripts/appliance/locked_compose.py"
grep -Fxq 'deploy/appliance' "${ROOT_DIR}/.dockerignore"
grep -Fxq 'scripts/appliance' "${ROOT_DIR}/.dockerignore"
grep -Fxq 'docs' "${ROOT_DIR}/.dockerignore"

air_workflow="${ROOT_DIR}/.github/workflows/air-gapped-appliance.yml"
package_workflow="${ROOT_DIR}/.github/workflows/package.yml"
mirror_workflow="${ROOT_DIR}/.github/workflows/mirror-final-proof.yml"
grep -Fq 'runs-on: ubuntu-24.04-arm' "${air_workflow}"
grep -Fq 'platform: linux/arm64' "${air_workflow}"
grep -Fq 'workflow_call:' "${air_workflow}"
test "$(grep -Fc "if: github.event_name != 'pull_request' || inputs.complete" "${air_workflow}")" -eq 2
test "$(grep -Fc 'ref: ${{ github.event.pull_request.head.sha || github.sha }}' "${air_workflow}")" -eq 5
if grep -Eq '^[[:space:]]+ref: [0-9a-f]{40}$' "${air_workflow}"; then
  echo "Air-gapped workflow contains a hard-coded checkout SHA." >&2
  exit 1
fi
grep -Fq "inputs.publish && github.ref == 'refs/heads/main'" "${air_workflow}"
grep -Fq 'Build custom images natively for arm64' "${air_workflow}"
python3 - "${air_workflow}" "${mirror_workflow}" <<'PYWORKFLOW'
import sys
from pathlib import Path

air_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
air_jobs = air_lines.index("jobs:")
air_top = air_lines[:air_jobs]
for forbidden in ("  packages: write", "  id-token: write", "  attestations: write"):
    assert forbidden not in air_top, forbidden
expected_air = {
    "build-offline": ("contents: read", "id-token: write"),
    "publish-platforms": ("contents: read", "packages: write"),
    "publish-indexes": (
        "contents: read",
        "packages: write",
        "id-token: write",
        "attestations: write",
    ),
}
for job, permissions in expected_air.items():
    index = air_lines.index(f"  {job}:")
    block = air_lines[index + 1:index + 2 + len(permissions)]
    assert block[0] == "    permissions:", (job, block)
    assert tuple(line.strip() for line in block[1:]) == permissions, (job, block)

mirror_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
mirror_jobs = mirror_lines.index("jobs:")
mirror_top = mirror_lines[:mirror_jobs]
assert "  actions: write" not in mirror_top, mirror_top
assert "  id-token: write" not in mirror_top, mirror_top
expected_mirror = {
    "dispatch-and-verify": ("contents: read", "actions: write"),
    "complete-appliance": (
    "contents: read",
    "packages: write",
    "id-token: write",
    "attestations: write",
),
}
for job, permissions in expected_mirror.items():
    index = mirror_lines.index(f"  {job}:")
    end = next(
        (candidate for candidate in range(index + 1, len(mirror_lines))
         if mirror_lines[candidate].startswith("  ")
         and not mirror_lines[candidate].startswith("    ")
         and mirror_lines[candidate].endswith(":")),
        len(mirror_lines),
    )
    job_block = mirror_lines[index + 1:end]
    permission_line = "    permissions:"
    assert permission_line in job_block, (job, job_block)
    permission_index = job_block.index(permission_line)
    block = job_block[permission_index + 1:permission_index + 1 + len(permissions)]
    assert tuple(line.strip() for line in block) == permissions, (job, block)
PYWORKFLOW
grep -Fq "if: github.event_name != 'pull_request'" "${package_workflow}"
grep -Fq 'Verify and serialize complete exact-head proof' "${mirror_workflow}"
grep -Fq 'Finalize exact-head mirror evidence' "${mirror_workflow}"
grep -Fq "github.event.pull_request.draft == false" "${mirror_workflow}"
grep -Fq -- "--event pull_request" "${mirror_workflow}"
grep -Fq "gh workflow run production-foundation.yml" "${mirror_workflow}"
grep -Fq "gh workflow run package.yml" "${mirror_workflow}"
grep -Fq 'uses: ./.github/workflows/air-gapped-appliance.yml' "${mirror_workflow}"
grep -Fq 'complete: true' "${mirror_workflow}"
if grep -Fq "gh workflow run air-gapped-appliance.yml" "${mirror_workflow}"; then
  echo "A newly introduced Air workflow cannot be bootstrapped through workflow_dispatch before merge." >&2
  exit 1
fi
production_line="$(grep -n "gh workflow run production-foundation.yml" "${mirror_workflow}" | cut -d: -f1)"
package_line="$(grep -n "gh workflow run package.yml" "${mirror_workflow}" | cut -d: -f1)"
appliance_line="$(grep -n 'uses: ./.github/workflows/air-gapped-appliance.yml' "${mirror_workflow}" | cut -d: -f1)"
test "${production_line}" -lt "${package_line}"
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

mkdir -p \
  "${fixture}/home" \
  "${fixture}/xdg-state" \
  "${fixture}/upgrades/v1" \
  "${fixture}/upgrades/v2"
cp "${ROOT_DIR}/deploy/appliance/edutalent-appliance" \
  "${fixture}/upgrades/v1/edutalent-appliance"
cp "${ROOT_DIR}/deploy/appliance/edutalent-appliance" \
  "${fixture}/upgrades/v2/edutalent-appliance"
chmod 0755 \
  "${fixture}/upgrades/v1/edutalent-appliance" \
  "${fixture}/upgrades/v2/edutalent-appliance"
expected_state="${fixture}/home/.local/state/edutalent-appliance"
state_v1="$(HOME="${fixture}/home" "${fixture}/upgrades/v1/edutalent-appliance" state-dir)"
state_v2="$(HOME="${fixture}/home" "${fixture}/upgrades/v2/edutalent-appliance" state-dir)"
test "${state_v1}" = "${expected_state}"
test "${state_v2}" = "${expected_state}"
test "${state_v1}" = "${state_v2}"
xdg_state="$(HOME="${fixture}/home" XDG_STATE_HOME="${fixture}/xdg-state" \
  "${fixture}/upgrades/v2/edutalent-appliance" state-dir)"
test "${xdg_state}" = "${fixture}/xdg-state/edutalent-appliance"
custom_state="$(HOME="${fixture}/home" EDUTALENT_APPLIANCE_STATE_DIR="${fixture}/managed-state" \
  "${fixture}/upgrades/v2/edutalent-appliance" state-dir)"
test "${custom_state}" = "${fixture}/managed-state"
if HOME="${fixture}/home" EDUTALENT_APPLIANCE_STATE_DIR="${fixture}/upgrades/v1/state" \
  "${fixture}/upgrades/v1/edutalent-appliance" state-dir >/dev/null 2>&1; then
  echo "Appliance accepted mutable state inside the immutable release." >&2
  exit 1
fi

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
grep -Fxq 'live app state' "${fixture}/production-source/.env.edutalent"
grep -Fxq 'live supabase state' "${fixture}/production-source/runtime/supabase/.env"

mkdir -p \
  "${fixture}/release-source/runtime/supabase/.git" \
  "${fixture}/release-source/runtime/keep"
printf 'release definition\n' > "${fixture}/release-source/keep.txt"
printf 'generated app secret\n' > "${fixture}/release-source/.env.edutalent"
printf 'APP_DOMAIN=\n' > "${fixture}/release-source/.env.edutalent.example"
printf 'generated Supabase secret\n' > "${fixture}/release-source/runtime/supabase/.env"
printf 'JWT_SECRET=\n' > "${fixture}/release-source/runtime/supabase/.env.example"
printf 'services: {}\n' > "${fixture}/release-source/runtime/supabase/docker-compose.yml"
printf 'upstream metadata\n' > "${fixture}/release-source/runtime/supabase/.git/config"
python3 "${ROOT_DIR}/scripts/appliance/stage_production.py" \
  --source "${fixture}/release-source" \
  --destination "${fixture}/release-staged" \
  --mode release
test -f "${fixture}/release-staged/keep.txt"
test -f "${fixture}/release-staged/.env.edutalent.example"
test -f "${fixture}/release-staged/runtime/supabase/.env.example"
test ! -e "${fixture}/release-staged/.env.edutalent"
test ! -e "${fixture}/release-staged/runtime/supabase/.env"
test ! -e "${fixture}/release-staged/runtime/supabase/.git"

printf 'stale secret backup\n' > "${fixture}/release-source/runtime/supabase/.env.old"
if python3 "${ROOT_DIR}/scripts/appliance/stage_production.py" \
  --source "${fixture}/release-source" \
  --destination "${fixture}/release-rejected-env-old" \
  --mode release >/dev/null 2>&1; then
  echo "Release staging accepted a generated Supabase .env.old backup." >&2
  exit 1
fi
rm -f "${fixture}/release-source/runtime/supabase/.env.old"
printf 'stale compose backup\n' > "${fixture}/release-source/runtime/supabase/docker-compose.yml.old"
if python3 "${ROOT_DIR}/scripts/appliance/stage_production.py" \
  --source "${fixture}/release-source" \
  --destination "${fixture}/release-rejected-compose-old" \
  --mode release >/dev/null 2>&1; then
  echo "Release staging accepted a generated Supabase Compose backup." >&2
  exit 1
fi
rm -f "${fixture}/release-source/runtime/supabase/docker-compose.yml.old"

secret_fixture="${fixture}/secret-generation"
mkdir -p "${secret_fixture}/runtime/supabase/utils"
cp "${ROOT_DIR}/deploy/production/generate-secrets.sh" "${secret_fixture}/generate-secrets.sh"
cat > "${secret_fixture}/.env.edutalent" <<'ENV'
APP_DOMAIN=app.fixture.invalid
SUPABASE_DOMAIN=supabase.fixture.invalid
ADMIN_DOMAIN=admin.fixture.invalid
ENV
cp "${secret_fixture}/.env.edutalent" "${secret_fixture}/.env.edutalent.example"
printf 'JWT_SECRET=\n' > "${secret_fixture}/runtime/supabase/.env.example"
printf 'services: {}\n' > "${secret_fixture}/runtime/supabase/docker-compose.yml"
cat > "${secret_fixture}/runtime/supabase/utils/generate-keys.sh" <<'SH'
#!/bin/sh
set -e
cp .env .env.old
SH
cat > "${secret_fixture}/runtime/supabase/utils/add-new-auth-keys.sh" <<'SH'
#!/bin/sh
set -e
cp .env .env.old
cp docker-compose.yml docker-compose.yml.old
SH
bash "${secret_fixture}/generate-secrets.sh" >/dev/null
test -f "${secret_fixture}/runtime/supabase/.env"
test ! -e "${secret_fixture}/runtime/supabase/.env.old"
test ! -e "${secret_fixture}/runtime/supabase/docker-compose.yml.old"
test ! -e "${secret_fixture}/runtime/supabase/docker-compose.yml.edutalent-backup"

mkdir -p "${fixture}/locked-compose"
cat > "${fixture}/locked-compose/services.tsv" <<'TSV'
app	registry.invalid/runtime:v1
db	registry.invalid/postgres:v1
embedding	registry.invalid/tei:v1
functions	registry.invalid/functions:v1
imgproxy	registry.invalid/imgproxy:v1
storage	registry.invalid/storage:v1
studio	registry.invalid/studio:v1
TSV
python3 - "${fixture}/locked-compose/images.json" <<'PYLOCK'
import json
import sys
from pathlib import Path
sources = (
    "registry.invalid/runtime:v1", "registry.invalid/postgres:v1",
    "registry.invalid/tei:v1", "registry.invalid/functions:v1",
    "registry.invalid/imgproxy:v1", "registry.invalid/storage:v1",
    "registry.invalid/studio:v1",
)
rows = [
    {"source_ref": source, "local_tag": f"edutalent-offline/component-{index}:fixture"}
    for index, source in enumerate(sources)
]
Path(sys.argv[1]).write_text(json.dumps({"images": rows}), encoding="utf-8")
PYLOCK
python3 "${ROOT_DIR}/scripts/appliance/locked_compose.py" \
  --service-images "${fixture}/locked-compose/services.tsv" \
  --images "${fixture}/locked-compose/images.json" \
  --output "${fixture}/locked-compose/compose.yaml"
for expected in \
  'pull_policy: never' \
  'edutalent-appliance-db-data' \
  'edutalent-appliance-storage-data' \
  'edutalent-appliance-studio-snippets' \
  'EDUTALENT_APPLIANCE_FUNCTIONS_DIR' \
  'read_only: true'; do
  grep -Fq "${expected}" "${fixture}/locked-compose/compose.yaml"
done

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
  "${fixture}/bundle/scripts/appliance" \
  "${fixture}/bundle/signatures"
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
            "Config": f"blobs/sha256/{digest}",
            "Layers": [],
            "RepoTags": ["edutalent-offline/fixture:v1-amd64-aaaaaaaaaaaaaaaa"],
        }
    ],
    separators=(",", ":"),
).encode()
with tarfile.open(Path(sys.argv[1]), "w:gz") as archive:
    for name, data in ((f"blobs/sha256/{digest}", config), ("manifest.json", manifest)):
        info = tarfile.TarInfo(name)
        info.mode = 0o644
        info.mtime = 0
        info.size = len(data)
        archive.addfile(info, io.BytesIO(data))
PY
python3 - "${ROOT_DIR}/scripts/appliance/release_manifest.py" "${fixture}/docker-archive-layouts" <<'PYARCHIVE'
import hashlib
import importlib.util
import io
import json
import sys
import tarfile
from pathlib import Path

source = Path(sys.argv[1])
root = Path(sys.argv[2])
root.mkdir(parents=True, exist_ok=True)
spec = importlib.util.spec_from_file_location("appliance_release_archive", source)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)

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
tag = "edutalent-offline/archive-layout:v1-amd64-aaaaaaaaaaaaaaaa"

def write_archive(path: Path, config_name: str, payload: bytes = config) -> None:
    manifest = json.dumps(
        [{"Config": config_name, "Layers": [], "RepoTags": [tag]}],
        separators=(",", ":"),
    ).encode()
    with tarfile.open(path, "w:gz") as archive:
        for name, data in ((config_name, payload), ("manifest.json", manifest)):
            info = tarfile.TarInfo(name)
            info.mode = 0o644
            info.mtime = 0
            info.size = len(data)
            archive.addfile(info, io.BytesIO(data))

for label, config_name in (
    ("legacy", f"{digest}.json"),
    ("containerd", f"blobs/sha256/{digest}"),
):
    archive_path = root / f"{label}.tar.gz"
    write_archive(archive_path, config_name)
    image_id, tags = module.docker_archive_identity(archive_path)
    assert image_id == f"sha256:{digest}", (label, image_id)
    assert tags == {tag}, (label, tags)

mismatch = root / "digest-mismatch.tar.gz"
write_archive(mismatch, "blobs/sha256/" + "0" * 64)
try:
    module.docker_archive_identity(mismatch)
except RuntimeError as error:
    assert "content digest" in str(error)
else:
    raise AssertionError("Docker archive config digest mismatch was accepted")

unsupported = root / "unsupported-config-path.tar.gz"
write_archive(unsupported, f"../{digest}.json")
try:
    module.docker_archive_identity(unsupported)
except RuntimeError as error:
    assert "config is invalid" in str(error)
else:
    raise AssertionError("unsafe Docker archive config path was accepted")
PYARCHIVE
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
cp "${ROOT_DIR}/deploy/appliance/edutalent-appliance" \
  "${fixture}/bundle/edutalent-appliance"
chmod 0755 "${fixture}/bundle/edutalent-appliance"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" generate \
  --bundle "${fixture}/bundle" \
  --version fixture \
  --git-sha 0123456789abcdef0123456789abcdef01234567 \
  --platform linux/amd64 \
  --signing-mode keyless \
  --images "${fixture}/bundle/manifests/images.json" \
  --model-lock "${fixture}/model.lock.json"
touch \
  "${fixture}/bundle/signatures/release-manifest.sigstore.json" \
  "${fixture}/bundle/signatures/SHA256SUMS.sigstore.json"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle"
cat > "${fixture}/bundle/signatures/policy.json" <<'JSON'
{"certificate_oidc_issuer":"https://attacker.invalid","certificate_identity_regexp":".*"}
JSON
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "Keyless release accepted an unsigned extra signature file." >&2
  exit 1
fi
rm -f "${fixture}/bundle/signatures/policy.json"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle"
mkdir -p "${fixture}/fake-bin"
cat > "${fixture}/fake-bin/cosign" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "${COSIGN_LOG:?}"
SH
chmod 0755 "${fixture}/fake-bin/cosign"
if PATH="${fixture}/fake-bin:${PATH}" COSIGN_LOG="${fixture}/cosign.log" \
  bash "${fixture}/bundle/edutalent-appliance" verify >/dev/null 2>&1; then
  echo "keyless appliance accepted a bundle-controlled trust policy" >&2
  exit 1
fi
PATH="${fixture}/fake-bin:${PATH}" \
  COSIGN_LOG="${fixture}/cosign.log" \
  EDUTALENT_APPLIANCE_TRUSTED_OIDC_ISSUER="https://issuer.example" \
  EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP='^https://identity.example/workflow$' \
  bash "${fixture}/bundle/edutalent-appliance" verify >/dev/null
grep -Fq -- '--certificate-oidc-issuer https://issuer.example' "${fixture}/cosign.log"
grep -Fq -- '--certificate-identity-regexp ^https://identity.example/workflow$' "${fixture}/cosign.log"
if grep -Fq 'attacker.invalid' "${fixture}/cosign.log"; then
  echo "bundle-controlled trust policy reached cosign" >&2
  exit 1
fi

rm -f "${fixture}/bundle/signatures/"*
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" generate \
  --bundle "${fixture}/bundle" \
  --version fixture \
  --git-sha 0123456789abcdef0123456789abcdef01234567 \
  --platform linux/amd64 \
  --signing-mode ephemeral \
  --images "${fixture}/bundle/manifests/images.json" \
  --model-lock "${fixture}/model.lock.json"
touch \
  "${fixture}/bundle/signatures/verification.pub" \
  "${fixture}/bundle/signatures/release-manifest.sig" \
  "${fixture}/bundle/signatures/SHA256SUMS.sig"
printf 'unsigned payload\n' > "${fixture}/bundle/signatures/extra.bin"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "Ephemeral release accepted an unsigned extra signature file." >&2
  exit 1
fi
rm -f "${fixture}/bundle/signatures/extra.bin"
mkdir "${fixture}/bundle/signatures/nested"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "Release signature inventory accepted an unexpected directory." >&2
  exit 1
fi
rmdir "${fixture}/bundle/signatures/nested"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle"
: > "${fixture}/cosign.log"
if PATH="${fixture}/fake-bin:${PATH}" COSIGN_LOG="${fixture}/cosign.log" \
  bash "${fixture}/bundle/edutalent-appliance" verify >/dev/null 2>&1; then
  echo "ephemeral appliance verified without explicit external opt-in" >&2
  exit 1
fi
PATH="${fixture}/fake-bin:${PATH}" \
  COSIGN_LOG="${fixture}/cosign.log" \
  EDUTALENT_APPLIANCE_ALLOW_EPHEMERAL_SIGNATURES=true \
  bash "${fixture}/bundle/edutalent-appliance" verify >/dev/null
grep -Fq -- '--key' "${fixture}/cosign.log"

mkdir -p "${fixture}/bundle/deploy/production/runtime/supabase"
printf 'attacker app state
' > "${fixture}/bundle/deploy/production/.env.edutalent"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify   --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "unsigned application environment was accepted" >&2
  exit 1
fi
rm -f "${fixture}/bundle/deploy/production/.env.edutalent"
printf 'attacker supabase state
' > "${fixture}/bundle/deploy/production/runtime/supabase/.env"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify   --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "unsigned Supabase environment was accepted" >&2
  exit 1
fi
rm -f "${fixture}/bundle/deploy/production/runtime/supabase/.env"
python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify   --bundle "${fixture}/bundle"
chmod 0755 "${fixture}/bundle/sbom/images/fixture.spdx.json"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "signed file mode change was accepted" >&2
  exit 1
fi
chmod 0644 "${fixture}/bundle/sbom/images/fixture.spdx.json"

mkfifo "${fixture}/bundle/unsigned-pipe"
if python3 "${ROOT_DIR}/scripts/appliance/release_manifest.py" verify \
  --bundle "${fixture}/bundle" >/dev/null 2>&1; then
  echo "unsigned FIFO was accepted" >&2
  exit 1
fi
rm -f "${fixture}/bundle/unsigned-pipe"

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

python3 - "${ROOT_DIR}/scripts/appliance/release_manifest.py" "${fixture}/private-key-scan" <<'PYTEST'
import importlib.util
import sys
from pathlib import Path

source = Path(sys.argv[1])
root = Path(sys.argv[2])
root.mkdir(parents=True, exist_ok=True)
spec = importlib.util.spec_from_file_location("appliance_release_manifest", source)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)
markers = (
    b"-----BEGIN PRIVATE KEY-----",
    b"-----BEGIN RSA PRIVATE KEY-----",
    b"-----BEGIN EC PRIVATE KEY-----",
    b"-----BEGIN DSA PRIVATE KEY-----",
    b"-----BEGIN OPENSSH PRIVATE KEY-----",
    b"-----BEGIN ENCRYPTED PRIVATE KEY-----",
    b"-----BEGIN PGP PRIVATE KEY BLOCK-----",
)
candidate = root / "arbitrary-release-note.txt"
for marker in markers:
    candidate.write_bytes(marker + b"\nfixture\n")
    try:
        module.reject_forbidden_file(root, candidate)
    except RuntimeError:
        continue
    raise AssertionError(f"private-key marker was accepted: {marker!r}")
PYTEST

python3 - "${ROOT_DIR}/scripts/appliance/release_manifest.py" "${fixture}/scanner-tests" <<'PYSCAN'
import importlib.util
import sys
from pathlib import Path

source = Path(sys.argv[1])
root = Path(sys.argv[2])
root.mkdir(parents=True, exist_ok=True)
spec = importlib.util.spec_from_file_location("appliance_release_scanner", source)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)

for name in (".env.local", ".env.production", ".env.test.secret"):
    candidate = root / name
    candidate.write_text("TOKEN=fixture\n", encoding="utf-8")
    try:
        module.reject_forbidden_file(root, candidate)
    except RuntimeError:
        continue
    raise AssertionError(f"dotenv variant was accepted: {name}")

example = root / ".env.example"
example.write_text("TOKEN=\n", encoding="utf-8")
module.reject_forbidden_file(root, example)

production_template = root / "deploy/production/.env.edutalent.example"
production_template.parent.mkdir(parents=True, exist_ok=True)
production_template.write_text("APP_PUBLIC_BASE_URL=\n", encoding="utf-8")
module.reject_forbidden_file(root, production_template)

for relative in (
    "deploy/production/.env.edutalent",
    "deploy/production/.env.edutalent.example.local",
    "other/.env.edutalent.example",
):
    candidate = root / relative
    candidate.parent.mkdir(parents=True, exist_ok=True)
    candidate.write_text("TOKEN=fixture\n", encoding="utf-8")
    try:
        module.reject_forbidden_file(root, candidate)
    except RuntimeError:
        continue
    raise AssertionError(f"non-allowlisted dotenv path was accepted: {relative}")

large = root / "large-release-note.txt"
large.write_bytes(b"x" * (5 * 1024 * 1024) + b"-----BEGIN OPENSSH PRIVATE KEY-----\n")
try:
    module.reject_forbidden_file(root, large)
except RuntimeError:
    pass
else:
    raise AssertionError("private key marker beyond 4 MiB was accepted")

large.write_bytes(b"x" * (5 * 1024 * 1024) + b"postgresql://user:password@db.internal/app\n")
try:
    module.reject_forbidden_file(root, large)
except RuntimeError:
    pass
else:
    raise AssertionError("credentialed database URL beyond 4 MiB was accepted")
PYSCAN

portable_checksums="${fixture}/portable-checksums"
mkdir -p "${portable_checksums}/source" "${portable_checksums}/moved"
printf 'portable archive\n' > "${portable_checksums}/source/release.tar.gz"
(cd "${portable_checksums}/source" && sha256sum release.tar.gz > release.tar.gz.SHA256SUMS)
cp "${portable_checksums}/source/release.tar.gz"   "${portable_checksums}/source/release.tar.gz.SHA256SUMS"   "${portable_checksums}/moved/"
(cd "${portable_checksums}/moved" && sha256sum --check release.tar.gz.SHA256SUMS)
test "$(awk '{print $2}' "${portable_checksums}/moved/release.tar.gz.SHA256SUMS")" = 'release.tar.gz'
grep -Fq 'sha256sum "${BUNDLE_NAME}.tar.gz.part-"*' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'sha256sum "${BUNDLE_NAME}.tar.gz"' "${ROOT_DIR}/scripts/appliance/build.sh"

echo "Air-gapped appliance definitions and integrity regression tests passed."
