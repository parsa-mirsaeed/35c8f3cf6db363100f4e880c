#!/usr/bin/env python3
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


# Make the Air workflow reusable from the same pull-request commit and preserve
# the exact PR head SHA when the caller event is pull_request.
air_path = Path(".github/workflows/air-gapped-appliance.yml")
air = air_path.read_text(encoding="utf-8")
workflow_call = """  workflow_call:
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
"""
air = replace_once(air, "  workflow_dispatch:\n", workflow_call + "  workflow_dispatch:\n", "workflow_call insertion")
condition = "    if: github.event_name != 'pull_request'"
if air.count(condition) != 2:
    raise SystemExit(f"complete-job condition: expected two matches, found {air.count(condition)}")
air = air.replace(condition, "    if: github.event_name != 'pull_request' || inputs.complete")
air = replace_once(
    air,
    "    timeout-minutes: 180\n    outputs:\n",
    "    timeout-minutes: 180\n    env:\n      EDUTALENT_APPLIANCE_GIT_SHA: ${{ github.event.pull_request.head.sha || github.sha }}\n    outputs:\n",
    "exact-head build environment",
)
air = replace_once(
    air,
    "          python3 - \"${bundle}\" \"${GITHUB_SHA}\" > appliance-evidence.json <<'PY'\n",
    "          python3 - \"${bundle}\" \"${{ github.event.pull_request.head.sha || github.sha }}\" > appliance-evidence.json <<'PY'\n",
    "exact-head evidence argument",
)
air = replace_once(
    air,
    "          EVENT_NAME: ${{ github.event_name }}\n",
    "          EVENT_NAME: ${{ github.event_name }}\n          COMPLETE_REQUESTED: ${{ github.event_name != 'pull_request' || inputs.complete }}\n",
    "complete gate input",
)
air = replace_once(
    air,
    "          if [[ \"${EVENT_NAME}\" == pull_request ]]; then\n            test \"${OFFLINE_RESULT}\" = skipped\n            test \"${ARM64_RESULT}\" = skipped\n          else\n            test \"${OFFLINE_RESULT}\" = success\n            test \"${ARM64_RESULT}\" = success\n          fi\n",
    "          if [[ \"${COMPLETE_REQUESTED}\" == true ]]; then\n            test \"${OFFLINE_RESULT}\" = success\n            test \"${ARM64_RESULT}\" = success\n          else\n            test \"${OFFLINE_RESULT}\" = skipped\n            test \"${ARM64_RESULT}\" = skipped\n          fi\n",
    "complete gate enforcement",
)
air_path.write_text(air, encoding="utf-8")

# Serialize Production -> Package in the first Mirror job, call the newly added
# Air workflow locally from the same commit, and finalize evidence afterwards.
mirror_path = Path(".github/workflows/mirror-final-proof.yml")
mirror = mirror_path.read_text(encoding="utf-8")
mirror = replace_once(
    mirror,
    "permissions:\n  contents: read\n  actions: write\n",
    "permissions:\n  contents: read\n",
    "mirror top-level permissions",
)
mirror = replace_once(
    mirror,
    "  dispatch-and-verify:\n    name: Verify and serialize complete exact-head proof\n",
    "  dispatch-and-verify:\n    name: Verify and serialize complete exact-head proof\n    permissions:\n      contents: read\n      actions: write\n",
    "mirror dispatch permissions",
)
tail_start = mirror.index("      - name: Dispatch complete exact-head Air-gapped Appliance proof\n")
new_tail = """      - name: Upload pre-appliance exact-head evidence
        uses: actions/upload-artifact@v6
        with:
          name: mirror-pre-appliance-evidence
          path: |
            ai-run.json
            full-validation-run.json
            production-pr-run.json
            production-run.json
            package-run.json
          if-no-files-found: error
          retention-days: 7

  complete-appliance:
    name: Complete exact-head Air-gapped Appliance proof
    needs: dispatch-and-verify
    permissions:
      contents: read
      id-token: write
    uses: ./.github/workflows/air-gapped-appliance.yml
    with:
      complete: true
      publish: false
      create_archive: false

  final-proof:
    name: Finalize exact-head mirror evidence
    needs:
      - dispatch-and-verify
      - complete-appliance
    permissions:
      contents: read
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    steps:
      - name: Download serialized pre-appliance evidence
        uses: actions/download-artifact@v6
        with:
          name: mirror-pre-appliance-evidence
          path: mirror-evidence

      - name: Download complete appliance gate evidence
        uses: actions/download-artifact@v6
        with:
          name: air-gapped-appliance-gate
          path: mirror-evidence

      - name: Enforce exact-head appliance evidence
        shell: bash
        env:
          HEAD_SHA: ${{ github.event.pull_request.head.sha }}
        run: |
          set -euo pipefail
          python3 - "$HEAD_SHA" mirror-evidence/air-gapped-appliance-gate.json <<'PY'
          import json
          import sys

          expected_sha, path = sys.argv[1:]
          evidence = json.load(open(path, encoding="utf-8"))
          assert evidence["head_sha"] == expected_sha, (evidence["head_sha"], expected_sha)
          assert evidence["validation"] == "success", evidence
          assert evidence["offline_appliance"] == "success", evidence
          assert evidence["native_arm64"] == "success", evidence
          assert evidence["ghcr_platforms"] == "skipped", evidence
          assert evidence["ghcr_indexes"] == "skipped", evidence
          print("Complete serialized exact-head proof verified.")
          PY

      - name: Upload exact-head mirror evidence
        uses: actions/upload-artifact@v6
        with:
          name: mirror-final-proof
          path: mirror-evidence/**
          if-no-files-found: error
          retention-days: 7
"""
mirror_path.write_text(mirror[:tail_start] + new_tail, encoding="utf-8")

# Preserve the pull-request head SHA in release metadata when called from a
# reusable workflow whose GITHUB_SHA is the pull-request merge ref.
build_path = Path("scripts/appliance/build.sh")
build = build_path.read_text(encoding="utf-8")
build = replace_once(
    build,
    'GIT_SHA="${GITHUB_SHA:-}"\n',
    'GIT_SHA="${EDUTALENT_APPLIANCE_GIT_SHA:-${GITHUB_SHA:-}}"\n',
    "builder exact-head override",
)
build_path.write_text(build, encoding="utf-8")

# Use one stable per-operator installation state directory across side-by-side
# release directories, while retaining an explicit absolute override.
launcher_path = Path("deploy/appliance/edutalent-appliance")
launcher = launcher_path.read_text(encoding="utf-8")
launcher = replace_once(
    launcher,
    'STATE_DIR="${EDUTALENT_APPLIANCE_STATE_DIR:-${ROOT_DIR}.state}"\n',
    '''if [[ -n "${EDUTALENT_APPLIANCE_STATE_DIR:-}" ]]; then
  STATE_DIR="${EDUTALENT_APPLIANCE_STATE_DIR}"
else
  : "${HOME:?HOME is required when EDUTALENT_APPLIANCE_STATE_DIR is unset}"
  STATE_DIR="${XDG_STATE_HOME:-${HOME}/.local/state}/edutalent-appliance"
fi
''',
    "stable state default",
)
launcher = replace_once(
    launcher,
    "  checks                 Run database, gateway, AI outage/recovery, and Qdrant checks\n",
    "  checks                 Run database, gateway, AI outage/recovery, and Qdrant checks\n  state-dir              Print the persistent installation state directory\n",
    "state-dir usage",
)
old_state_function = '''initialize_state_dirs() {
    [[ "${STATE_DIR}" == /* ]] || {
        echo "EDUTALENT_APPLIANCE_STATE_DIR must be an absolute path: ${STATE_DIR}" >&2
        exit 1
    }
    case "${STATE_DIR}" in
      "${ROOT_DIR}"|"${ROOT_DIR}/"*)
        echo "Appliance state must remain outside the immutable release: ${STATE_DIR}" >&2
        exit 1
        ;;
    esac
    mkdir -p "${STATE_CONFIG_DIR}" "${STATE_DIR}/runtime"
    chmod 0700 "${STATE_DIR}" "${STATE_CONFIG_DIR}" "${STATE_DIR}/runtime"
}
'''
new_state_function = '''validate_state_dir() {
    [[ "${STATE_DIR}" == /* ]] || {
        echo "EDUTALENT_APPLIANCE_STATE_DIR and XDG_STATE_HOME must resolve to an absolute path: ${STATE_DIR}" >&2
        exit 1
    }
    case "${STATE_DIR}" in
      "${ROOT_DIR}"|"${ROOT_DIR}/"*)
        echo "Appliance state must remain outside the immutable release: ${STATE_DIR}" >&2
        exit 1
        ;;
    esac
}

initialize_state_dirs() {
    validate_state_dir
    mkdir -p "${STATE_CONFIG_DIR}" "${STATE_DIR}/runtime"
    chmod 0700 "${STATE_DIR}" "${STATE_CONFIG_DIR}" "${STATE_DIR}/runtime"
}
'''
launcher = replace_once(launcher, old_state_function, new_state_function, "state validation functions")
launcher = replace_once(
    launcher,
    '  status) production ps "$@" ;;\n',
    '''  status) production ps "$@" ;;
  state-dir)
    validate_state_dir
    printf '%s\\n' "${STATE_DIR}"
    ;;
''',
    "state-dir command",
)
launcher_path.write_text(launcher, encoding="utf-8")

# Align operator documentation with immutable release state and side-by-side
# upgrade reuse.
readme_path = Path("deploy/appliance/README.md")
readme = readme_path.read_text(encoding="utf-8")
old_init = '''The first `init` creates `deploy/production/.env.edutalent`. Set the three domains,
restricted administrative CIDRs, and absolute operator-supplied TLS certificate
and key paths. Run `init` again to generate fresh installation-specific secrets
inside the packaged, network-disabled tools container. Integrity verification
permits only that file and `deploy/production/runtime/supabase/.env` as mutable
installation state; every other appliance file remains exactly manifest-bound.
'''
new_init = '''The first `init` creates the external application environment under the stable
installation state directory. By default this is
`${XDG_STATE_HOME:-$HOME/.local/state}/edutalent-appliance`; use
`./edutalent-appliance state-dir` to print the exact path. An installation may set
an absolute `EDUTALENT_APPLIANCE_STATE_DIR` before every appliance command when a
different managed location is required.

Set the three domains, restricted administrative CIDRs, and absolute
operator-supplied TLS certificate and key paths in
`<state-dir>/config/.env.edutalent`. Run `init` again to generate fresh
installation-specific secrets inside the packaged, network-disabled tools
container. Generated application and Supabase environments remain outside the
signed release tree; every appliance file remains exactly manifest-bound.
'''
readme = replace_once(readme, old_init, new_init, "offline state documentation")
old_update = '''Each version installs beside the previous version. Stop the current appliance,
verify and load the new bundle, retain the existing production environment and
volumes, then start the new version. Rollback selects the previous verified bundle
and starts it against the unchanged data volumes. Database migration rollback and
full backup restoration remain part of Plan V1 Production Operations and must be
proven before a production rollout.
'''
new_update = '''Each version installs beside the previous version. Stop the current appliance,
verify and load the new bundle, and run the new launcher as the same operating-system
user with the same `XDG_STATE_HOME` or explicit `EDUTALENT_APPLIANCE_STATE_DIR`.
Confirm that `state-dir` prints the same path for both versions before startup. The
new version then reuses the existing application environment, Supabase secrets, and
named data volumes rather than generating replacement credentials. Rollback selects
the previous verified bundle with that same state directory and unchanged data
volumes. Database migration rollback and full backup restoration remain part of
Plan V1 Production Operations and must be proven before a production rollout.
'''
readme = replace_once(readme, old_update, new_update, "upgrade state documentation")
readme_path.write_text(readme, encoding="utf-8")

# Extend the focused validator for reusable orchestration, permission scoping,
# exact-head metadata, and state reuse across two side-by-side bundles.
validate_path = Path("scripts/appliance/validate.sh")
validate = validate_path.read_text(encoding="utf-8")
validate = replace_once(
    validate,
    "grep -Fq 'EDUTALENT_APPLIANCE_STATE_DIR' \"${ROOT_DIR}/deploy/appliance/edutalent-appliance\"\n",
    "grep -Fq 'EDUTALENT_APPLIANCE_STATE_DIR' \"${ROOT_DIR}/deploy/appliance/edutalent-appliance\"\ngrep -Fq 'XDG_STATE_HOME' \"${ROOT_DIR}/deploy/appliance/edutalent-appliance\"\ngrep -Fq 'state-dir)' \"${ROOT_DIR}/deploy/appliance/edutalent-appliance\"\ngrep -Fq 'EDUTALENT_APPLIANCE_GIT_SHA' \"${ROOT_DIR}/scripts/appliance/build.sh\"\n",
    "state and exact-head static checks",
)
workflow_start = validate.index("grep -Fq 'runs-on: ubuntu-24.04-arm' \"${air_workflow}\"\n")
workflow_end = validate.index("if grep -Fq 'setup-qemu-action'", workflow_start)
workflow_checks = '''grep -Fq 'runs-on: ubuntu-24.04-arm' "${air_workflow}"
grep -Fq 'platform: linux/arm64' "${air_workflow}"
grep -Fq 'workflow_call:' "${air_workflow}"
test "$(grep -Fc "if: github.event_name != 'pull_request' || inputs.complete" "${air_workflow}")" -eq 2
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
    "complete-appliance": ("contents: read", "id-token: write"),
}
for job, permissions in expected_mirror.items():
    index = mirror_lines.index(f"  {job}:")
    name_index = index + 1
    permission_index = name_index + 1
    assert mirror_lines[permission_index] == "    permissions:", (job, mirror_lines[index:index + 8])
    block = mirror_lines[permission_index + 1:permission_index + 1 + len(permissions)]
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
'''
validate = validate[:workflow_start] + workflow_checks + validate[workflow_end:]
fixture_marker = '''fixture="$(mktemp -d)"
trap 'rm -rf "${fixture}"' EXIT

'''
state_regression = r'''fixture="$(mktemp -d)"
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

'''
validate = replace_once(validate, fixture_marker, state_regression, "state regression insertion")
validate_path.write_text(validate, encoding="utf-8")
