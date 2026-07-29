from pathlib import Path


def job_block(text: str, start_name: str, end_name: str | None) -> str:
    start = text.index(f"  {start_name}:\n")
    end = text.index(f"  {end_name}:\n", start) if end_name else len(text)
    return text[start:end]


air_path = Path(".github/workflows/air-gapped-appliance.yml")
original = air_path.read_text(encoding="utf-8")
validate_block = job_block(original, "validate", "build-offline")
pr_build = job_block(original, "build-offline", "build-release")
release_build = job_block(original, "build-release", "native-arm64")
native_block = job_block(original, "native-arm64", "publish-platforms")
publish_platforms = job_block(original, "publish-platforms", "publish-indexes")
publish_indexes = job_block(original, "publish-indexes", "gate")

pr_build = pr_build.replace(
    "    if: github.event_name == 'pull_request' && inputs.complete\n",
    "    if: inputs.complete\n",
    1,
)
native_proof = native_block.replace(
    "    if: github.event_name != 'pull_request' || inputs.complete\n",
    "    if: inputs.complete\n",
    1,
)

proof_header = """name: Air-gapped Appliance

on:
  pull_request:
    types:
      - opened
      - synchronize
      - reopened
      - labeled
    paths:
      - Dockerfile
      - Dockerfile.appliance-tools
      - compose.yaml
      - compose.release.yaml
      - edutalent
      - Makefile
      - deploy/appliance/**
      - deploy/production/**
      - scripts/appliance/**
      - scripts/ci/**
      - migrations/**
      - packages/**
      - .github/workflows/air-gapped-appliance.yml
      - .github/workflows/air-gapped-release.yml
      - .github/workflows/package.yml
      - .github/workflows/mirror-final-proof.yml
  workflow_call:
    inputs:
      version:
        description: Appliance version; defaults to the exact commit prefix
        required: false
        type: string
      complete:
        description: Run the complete offline and native architecture proof
        required: false
        default: false
        type: boolean
      create_archive:
        description: Create an ephemeral validation archive
        required: false
        default: false
        type: boolean

permissions:
  contents: read

concurrency:
  group: air-gapped-appliance-${{ github.head_ref || github.ref_name }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: never
  APP_IMAGE: ghcr.io/${{ github.repository_owner }}/edutalent-app
  AI_GATEWAY_IMAGE: ghcr.io/${{ github.repository_owner }}/edutalent-ai-gateway
  MIGRATION_IMAGE: ghcr.io/${{ github.repository_owner }}/edutalent-migration
  TOOLS_IMAGE: ghcr.io/${{ github.repository_owner }}/edutalent-appliance-tools

jobs:
"""

proof_gate = """  gate:
    name: Air-gapped appliance gate
    if: always()
    needs:
      - validate
      - build-offline
      - native-arm64
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - name: Enforce exact required outcomes
        shell: bash
        env:
          COMPLETE_REQUESTED: ${{ inputs.complete }}
          VALIDATE_RESULT: ${{ needs.validate.result }}
          OFFLINE_RESULT: ${{ needs.build-offline.result }}
          ARM64_RESULT: ${{ needs.native-arm64.result }}
          HEAD_SHA: ${{ github.event.pull_request.head.sha || github.sha }}
        run: |
          set -euo pipefail
          test "${VALIDATE_RESULT}" = success
          if [[ "${COMPLETE_REQUESTED}" == true ]]; then
            test "${OFFLINE_RESULT}" = success
            test "${ARM64_RESULT}" = success
          else
            test "${OFFLINE_RESULT}" = skipped
            test "${ARM64_RESULT}" = skipped
          fi
          cat > air-gapped-appliance-gate.json <<EOF
          {
            "head_sha": "${HEAD_SHA}",
            "validation": "${VALIDATE_RESULT}",
            "offline_appliance": "${OFFLINE_RESULT}",
            "native_arm64": "${ARM64_RESULT}",
            "ghcr_platforms": "skipped",
            "ghcr_indexes": "skipped"
          }
          EOF

      - uses: actions/upload-artifact@v6
        with:
          name: air-gapped-appliance-gate
          path: air-gapped-appliance-gate.json
          if-no-files-found: error
          retention-days: 7
"""

proof = proof_header + validate_block + pr_build + native_proof + proof_gate
for forbidden in ("id-token: write", "packages: write", "attestations: write"):
    if forbidden in proof:
        raise SystemExit(f"PR proof retained privileged permission: {forbidden}")
if "EDUTALENT_APPLIANCE_SIGNING_MODE: ephemeral" not in proof:
    raise SystemExit("PR proof lost ephemeral signing mode")
air_path.write_text(proof, encoding="utf-8")

release_build = release_build.replace(
    "    if: github.event_name != 'pull_request'\n    needs: validate\n",
    "    needs:\n      - validate\n      - release-policy\n",
    1,
)
native_release = native_block.replace(
    "    if: github.event_name != 'pull_request' || inputs.complete\n    needs: validate\n",
    "    needs:\n      - validate\n      - release-policy\n",
    1,
).replace(
    "    name: Build custom images natively for arm64\n",
    "    name: Build protected release images natively for arm64\n",
    1,
)

release_header = """name: Air-gapped Release

on:
  workflow_dispatch:
    inputs:
      version:
        description: Appliance version; defaults to the exact commit prefix
        required: false
        type: string
      publish:
        description: Publish signed multi-architecture custom images to GHCR; main only
        required: false
        default: false
        type: boolean
      create_archive:
        description: Create and retain the signed offline release archive
        required: false
        default: false
        type: boolean
  push:
    tags:
      - "v*"

permissions:
  contents: read

concurrency:
  group: air-gapped-release-${{ github.ref_name }}
  cancel-in-progress: false

env:
  CARGO_TERM_COLOR: never
  APP_IMAGE: ghcr.io/${{ github.repository_owner }}/edutalent-app
  AI_GATEWAY_IMAGE: ghcr.io/${{ github.repository_owner }}/edutalent-ai-gateway
  MIGRATION_IMAGE: ghcr.io/${{ github.repository_owner }}/edutalent-migration
  TOOLS_IMAGE: ghcr.io/${{ github.repository_owner }}/edutalent-appliance-tools

jobs:
  release-policy:
    name: Enforce protected release source
    permissions:
      contents: read
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - name: Require a version tag or main-branch dispatch
        env:
          ALLOWED: ${{ startsWith(github.ref, 'refs/tags/v') || (github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main') }}
        run: test "${ALLOWED}" = true

"""

release_gate = """  gate:
    name: Air-gapped release gate
    if: always()
    needs:
      - release-policy
      - validate
      - build-release
      - native-arm64
      - publish-platforms
      - publish-indexes
    permissions:
      contents: read
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - name: Enforce exact protected release outcomes
        env:
          POLICY_RESULT: ${{ needs.release-policy.result }}
          VALIDATE_RESULT: ${{ needs.validate.result }}
          RELEASE_RESULT: ${{ needs.build-release.result }}
          ARM64_RESULT: ${{ needs.native-arm64.result }}
          PUBLISH_REQUESTED: ${{ startsWith(github.ref, 'refs/tags/v') || (github.event_name == 'workflow_dispatch' && inputs.publish && github.ref == 'refs/heads/main') }}
          PUBLISH_PLATFORMS_RESULT: ${{ needs.publish-platforms.result }}
          PUBLISH_INDEXES_RESULT: ${{ needs.publish-indexes.result }}
        run: |
          set -euo pipefail
          test "${POLICY_RESULT}" = success
          test "${VALIDATE_RESULT}" = success
          test "${RELEASE_RESULT}" = success
          test "${ARM64_RESULT}" = success
          if [[ "${PUBLISH_REQUESTED}" == true ]]; then
            test "${PUBLISH_PLATFORMS_RESULT}" = success
            test "${PUBLISH_INDEXES_RESULT}" = success
          else
            test "${PUBLISH_PLATFORMS_RESULT}" = skipped
            test "${PUBLISH_INDEXES_RESULT}" = skipped
          fi
"""

release = (
    release_header
    + validate_block
    + release_build
    + native_release
    + publish_platforms
    + publish_indexes
    + release_gate
)
if release.count("id-token: write") != 2:
    raise SystemExit("protected release workflow has unexpected OIDC grant count")
if "EDUTALENT_APPLIANCE_SIGNING_MODE: keyless" not in release:
    raise SystemExit("protected release workflow lost keyless signing mode")
Path(".github/workflows/air-gapped-release.yml").write_text(release, encoding="utf-8")

validate_path = Path("scripts/appliance/validate.sh")
validate = validate_path.read_text(encoding="utf-8")
start_marker = 'air_workflow="${ROOT_DIR}/.github/workflows/air-gapped-appliance.yml"\n'
end_marker = 'grep -Fq "if: github.event_name != \'pull_request\'" "${package_workflow}"\n'
start = validate.index(start_marker)
end = validate.index(end_marker, start)
replacement = r'''air_workflow="${ROOT_DIR}/.github/workflows/air-gapped-appliance.yml"
release_workflow="${ROOT_DIR}/.github/workflows/air-gapped-release.yml"
package_workflow="${ROOT_DIR}/.github/workflows/package.yml"
mirror_workflow="${ROOT_DIR}/.github/workflows/mirror-final-proof.yml"
grep -Fq 'runs-on: ubuntu-24.04-arm' "${air_workflow}"
grep -Fq 'platforms: linux/arm64' "${air_workflow}"
grep -Fq 'workflow_call:' "${air_workflow}"
grep -Fq 'if: inputs.complete' "${air_workflow}"
grep -Fq 'EDUTALENT_APPLIANCE_SIGNING_MODE: ephemeral' "${air_workflow}"
if grep -Eq 'packages: write|id-token: write|attestations: write' "${air_workflow}"; then
  echo "Pull-request appliance proof must remain read-only." >&2
  exit 1
fi
if grep -Fq 'workflow_dispatch:' "${air_workflow}" || grep -Fq 'push:' "${air_workflow}"; then
  echo "Protected release triggers must not exist in the PR proof workflow." >&2
  exit 1
fi
grep -Fq 'workflow_dispatch:' "${release_workflow}"
grep -Fq 'refs/heads/main' "${release_workflow}"
grep -Fq 'tags:' "${release_workflow}"
grep -Fq 'EDUTALENT_APPLIANCE_SIGNING_MODE: keyless' "${release_workflow}"
grep -Fq 'id-token: write' "${release_workflow}"
grep -Fq 'packages: write' "${release_workflow}"
grep -Fq 'attestations: write' "${release_workflow}"
if grep -Fq 'pull_request:' "${release_workflow}" || grep -Fq 'workflow_call:' "${release_workflow}"; then
  echo "Protected release workflow must not be callable from pull requests." >&2
  exit 1
fi
if grep -Eq '^[[:space:]]+ref: [0-9a-f]{40}$' "${air_workflow}" "${release_workflow}"; then
  echo "Air-gapped workflows contain a hard-coded checkout SHA." >&2
  exit 1
fi
if grep -Fq 'setup-qemu-action' "${air_workflow}" "${release_workflow}"; then
  echo "Air-gapped workflows must use native architecture runners, not QEMU." >&2
  exit 1
fi
python3 - "${air_workflow}" "${release_workflow}" "${mirror_workflow}" <<'PYWORKFLOW'
import sys
from pathlib import Path


def job_block(lines, job):
    index = lines.index(f"  {job}:")
    end = next(
        (candidate for candidate in range(index + 1, len(lines))
         if lines[candidate].startswith("  ")
         and not lines[candidate].startswith("    ")
         and lines[candidate].endswith(":")),
        len(lines),
    )
    return lines[index + 1:end]


air_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
assert "  workflow_dispatch:" not in air_lines
assert "  push:" not in air_lines
pr_build = job_block(air_lines, "build-offline")
assert pr_build[0] == "    permissions:"
assert pr_build[1].strip() == "contents: read"
assert "      id-token: write" not in pr_build
assert "    if: inputs.complete" in pr_build
assert "          EDUTALENT_APPLIANCE_SIGNING_MODE: ephemeral" in pr_build
assert not any(line.strip() in {"packages: write", "id-token: write", "attestations: write"} for line in air_lines)

release_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
release_build = job_block(release_lines, "build-release")
assert "      id-token: write" in release_build
assert "          EDUTALENT_APPLIANCE_SIGNING_MODE: keyless" in release_build
publish_platforms = job_block(release_lines, "publish-platforms")
assert "      packages: write" in publish_platforms
publish_indexes = job_block(release_lines, "publish-indexes")
for permission in ("      packages: write", "      id-token: write", "      attestations: write"):
    assert permission in publish_indexes, permission
assert "  pull_request:" not in release_lines
assert "  workflow_call:" not in release_lines

mirror_lines = Path(sys.argv[3]).read_text(encoding="utf-8").splitlines()
mirror_jobs = mirror_lines.index("jobs:")
mirror_top = mirror_lines[:mirror_jobs]
assert "  actions: write" not in mirror_top, mirror_top
assert "  id-token: write" not in mirror_top, mirror_top
expected_mirror = {
    "dispatch-and-verify": ("contents: read", "actions: write"),
    "complete-appliance": ("contents: read",),
}
for job, permissions in expected_mirror.items():
    block = job_block(mirror_lines, job)
    permission_index = block.index("    permissions:")
    actual = tuple(line.strip() for line in block[permission_index + 1:permission_index + 1 + len(permissions)])
    assert actual == permissions, (job, actual)
PYWORKFLOW
'''
validate = validate[:start] + replacement + validate[end:]
validate_path.write_text(validate, encoding="utf-8")
