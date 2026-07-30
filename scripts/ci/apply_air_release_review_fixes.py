#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def replace_exact(path: Path, old: str, new: str, expected_count: int = 1) -> None:
    text = path.read_text(encoding="utf-8")
    actual_count = text.count(old)
    if actual_count != expected_count:
        raise RuntimeError(
            f"{path}: expected {expected_count} occurrence(s), found {actual_count}"
        )
    path.write_text(text.replace(old, new), encoding="utf-8")


build = ROOT / "scripts/appliance/build.sh"
replace_exact(
    build,
    '''      export EDUTALENT_APPLIANCE_TRUSTED_OIDC_ISSUER="https://token.actions.githubusercontent.com"
      export EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP="^https://github.com/${GITHUB_REPOSITORY}/\\.github/workflows/air-gapped-appliance\\.yml@${GITHUB_REF}$"
''',
    '''      expected_workflow_ref="${GITHUB_REPOSITORY}/.github/workflows/air-gapped-release.yml@${GITHUB_REF}"
      [[ "${GITHUB_WORKFLOW_REF:-}" == "${expected_workflow_ref}" ]] || {
        echo "Keyless appliance signing is restricted to ${expected_workflow_ref}." >&2
        exit 1
      }
      trusted_identity="https://github.com/${expected_workflow_ref}"
      export EDUTALENT_APPLIANCE_TRUSTED_OIDC_ISSUER="https://token.actions.githubusercontent.com"
      export EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP="$(
        python3 -c 'import re, sys; print("^" + re.escape(sys.argv[1]) + "$")' \
          "${trusted_identity}"
      )"
''',
)

release = ROOT / ".github/workflows/air-gapped-release.yml"
replace_exact(
    release,
    '''      - id: version
        name: Resolve exact appliance version
        shell: bash
        run: |
          set -euo pipefail
          requested='${{ inputs.version }}'
          if [[ -n "${requested}" ]]; then
            version="${requested}"
          elif [[ "${GITHUB_REF_TYPE}" == "tag" ]]; then
            version="${GITHUB_REF_NAME}"
          else
            version="${GITHUB_SHA::12}"
          fi
          test -n "${version}"
          echo "version=${version}" >> "${GITHUB_OUTPUT}"
          echo "bundle_dir=dist/edutalent-appliance-${version}-amd64" >> "${GITHUB_OUTPUT}"
''',
    '''      - id: version
        name: Resolve exact appliance version
        shell: bash
        env:
          REQUESTED_VERSION: ${{ inputs.version }}
        run: |
          set -euo pipefail
          requested="${REQUESTED_VERSION:-}"
          if [[ -n "${requested}" ]]; then
            version="${requested}"
          elif [[ "${GITHUB_REF_TYPE}" == "tag" ]]; then
            version="${GITHUB_REF_NAME}"
          else
            version="${GITHUB_SHA::12}"
          fi
          [[ "${version}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$ ]] || {
            echo "Version must be a safe tag and path component." >&2
            exit 2
          }
          {
            echo "version=${version}"
            echo "bundle_dir=dist/edutalent-appliance-${version}-amd64"
          } >> "${GITHUB_OUTPUT}"
''',
)
replace_exact(
    release,
    '''      - id: version
        name: Resolve publish version
        shell: bash
        run: |
          set -euo pipefail
          requested='${{ inputs.version }}'
          if [[ -n "${requested}" ]]; then
            version="${requested}"
          elif [[ "${GITHUB_REF_TYPE}" == "tag" ]]; then
            version="${GITHUB_REF_NAME}"
          else
            version="${GITHUB_SHA::12}"
          fi
          echo "version=${version}" >> "${GITHUB_OUTPUT}"
''',
    '''      - id: version
        name: Resolve publish version
        shell: bash
        env:
          REQUESTED_VERSION: ${{ inputs.version }}
        run: |
          set -euo pipefail
          requested="${REQUESTED_VERSION:-}"
          if [[ -n "${requested}" ]]; then
            version="${requested}"
          elif [[ "${GITHUB_REF_TYPE}" == "tag" ]]; then
            version="${GITHUB_REF_NAME}"
          else
            version="${GITHUB_SHA::12}"
          fi
          [[ "${version}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$ ]] || {
            echo "Version must be a safe tag and path component." >&2
            exit 2
          }
          echo "version=${version}" >> "${GITHUB_OUTPUT}"
''',
    expected_count=2,
)
replace_exact(
    release,
    '''        env:
          EDUTALENT_APPLIANCE_PLATFORM: linux/amd64
          EDUTALENT_APPLIANCE_SIGNING_MODE: keyless
          EDUTALENT_APPLIANCE_CREATE_ARCHIVE: ${{ startsWith(github.ref, 'refs/tags/v') || inputs.create_archive || false }}
          EDUTALENT_BUILD_CACHE_SCOPE: edutalent-runtime
        run: |
          set -o pipefail
          bash scripts/appliance/build.sh '${{ steps.version.outputs.version }}' \\
            2>&1 | tee appliance-build.log
''',
    '''        env:
          APPLIANCE_VERSION: ${{ steps.version.outputs.version }}
          EDUTALENT_APPLIANCE_PLATFORM: linux/amd64
          EDUTALENT_APPLIANCE_SIGNING_MODE: keyless
          EDUTALENT_APPLIANCE_CREATE_ARCHIVE: ${{ startsWith(github.ref, 'refs/tags/v') || inputs.create_archive || false }}
          EDUTALENT_BUILD_CACHE_SCOPE: edutalent-runtime
        run: |
          set -euo pipefail
          bash scripts/appliance/build.sh "${APPLIANCE_VERSION}" \\
            2>&1 | tee appliance-build.log
''',
)
replace_exact(
    release,
    '''      - name: Prove first startup with all registry pulls disabled
        shell: bash
        run: |
          set -o pipefail
          bash scripts/appliance/offline_smoke.sh '${{ steps.version.outputs.bundle_dir }}' \\
            2>&1 | tee appliance-offline-smoke.log
''',
    '''      - name: Prove first startup with all registry pulls disabled
        shell: bash
        env:
          BUNDLE_DIR: ${{ steps.version.outputs.bundle_dir }}
        run: |
          set -euo pipefail
          bash scripts/appliance/offline_smoke.sh "${BUNDLE_DIR}" \\
            2>&1 | tee appliance-offline-smoke.log
''',
)
replace_exact(
    release,
    '''      - name: Write exact-head appliance evidence
        shell: bash
        run: |
          set -euo pipefail
          bundle='${{ steps.version.outputs.bundle_dir }}'
          python3 - "${bundle}" "${{ github.event.pull_request.head.sha || github.sha }}" > appliance-evidence.json <<'PY'
''',
    '''      - name: Write exact-head appliance evidence
        shell: bash
        env:
          BUNDLE_DIR: ${{ steps.version.outputs.bundle_dir }}
          EXPECTED_SHA: ${{ github.event.pull_request.head.sha || github.sha }}
        run: |
          set -euo pipefail
          python3 - "${BUNDLE_DIR}" "${EXPECTED_SHA}" > appliance-evidence.json <<'PY'
''',
)
replace_exact(
    release,
    '''      - name: Stop and remove appliance smoke data
        if: always()
        shell: bash
        run: |
          bundle='${{ steps.version.outputs.bundle_dir }}'
          if [[ -x "${bundle}/edutalent-appliance" ]]; then
            "${bundle}/edutalent-appliance" stop >/dev/null 2>&1 || true
          fi
''',
    '''      - name: Stop and remove appliance smoke data
        if: always()
        shell: bash
        env:
          BUNDLE_DIR: ${{ steps.version.outputs.bundle_dir }}
        run: |
          if [[ -x "${BUNDLE_DIR}/edutalent-appliance" ]]; then
            "${BUNDLE_DIR}/edutalent-appliance" stop >/dev/null 2>&1 || true
          fi
''',
)
replace_exact(
    release,
    '''      - id: indexes
        name: Assemble immutable multi-architecture indexes
        shell: bash
        run: |
          set -euo pipefail
          version='${{ steps.version.outputs.version }}'
''',
    '''      - id: indexes
        name: Assemble immutable multi-architecture indexes
        shell: bash
        env:
          VERSION: ${{ steps.version.outputs.version }}
        run: |
          set -euo pipefail
          version="${VERSION}"
''',
)
replace_exact(
    release,
    '''      - name: Write publication evidence
        shell: bash
        env:
          APP_DIGEST: ${{ steps.indexes.outputs.app_digest }}
''',
    '''      - name: Write publication evidence
        shell: bash
        env:
          VERSION: ${{ steps.version.outputs.version }}
          APP_DIGEST: ${{ steps.indexes.outputs.app_digest }}
''',
)
replace_exact(
    release,
    '''            "version": "${{ steps.version.outputs.version }}",
''',
    '''            "version": "${VERSION}",
''',
)

package = ROOT / ".github/workflows/package.yml"
replace_exact(
    package,
    '''      - name: Resolve package version
        id: version
        shell: bash
        run: |
          requested='${{ inputs.version }}'
          if [[ -n "$requested" ]]; then
            version="$requested"
          elif [[ "$GITHUB_REF_TYPE" == "tag" ]]; then
            version="$GITHUB_REF_NAME"
          else
            version="${GITHUB_SHA::12}"
          fi
          echo "version=$version" >> "$GITHUB_OUTPUT"
''',
    '''      - name: Resolve package version
        id: version
        shell: bash
        env:
          REQUESTED_VERSION: ${{ inputs.version }}
        run: |
          set -euo pipefail
          requested="${REQUESTED_VERSION:-}"
          if [[ -n "${requested}" ]]; then
            version="${requested}"
          elif [[ "${GITHUB_REF_TYPE}" == "tag" ]]; then
            version="${GITHUB_REF_NAME}"
          else
            version="${GITHUB_SHA::12}"
          fi
          [[ "${version}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$ ]] || {
            echo "Version must be a safe tag and path component." >&2
            exit 2
          }
          echo "version=${version}" >> "${GITHUB_OUTPUT}"
''',
)
replace_exact(
    package,
    '''      - name: Build release bundle through unified command
        shell: bash
        run: |
          set -o pipefail
          bash edutalent package '${{ steps.version.outputs.version }}' 2>&1 | tee package-build.log
''',
    '''      - name: Build release bundle through unified command
        shell: bash
        env:
          PACKAGE_VERSION: ${{ steps.version.outputs.version }}
        run: |
          set -euo pipefail
          bash edutalent package "${PACKAGE_VERSION}" 2>&1 | tee package-build.log
''',
)

validate = ROOT / "scripts/appliance/validate.sh"
replace_exact(
    validate,
    '''grep -Fq 'EDUTALENT_APPLIANCE_GIT_SHA' "${ROOT_DIR}/scripts/appliance/build.sh"
''',
    '''grep -Fq 'EDUTALENT_APPLIANCE_GIT_SHA' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'GITHUB_WORKFLOW_REF' "${ROOT_DIR}/scripts/appliance/build.sh"
grep -Fq 'air-gapped-release.yml@${GITHUB_REF}' "${ROOT_DIR}/scripts/appliance/build.sh"
if grep -Fq 'air-gapped-appliance\\.yml@${GITHUB_REF}' "${ROOT_DIR}/scripts/appliance/build.sh"; then
  echo "Keyless verification must trust the protected release workflow identity." >&2
  exit 1
fi
''',
)
replace_exact(
    validate,
    '''python3 - "${air_workflow}" "${release_workflow}" "${mirror_workflow}" <<'PYWORKFLOW'
''',
    '''python3 - "${air_workflow}" "${release_workflow}" "${package_workflow}" "${mirror_workflow}" <<'PYWORKFLOW'
''',
)
replace_exact(
    validate,
    '''release_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
release_build = job_block(release_lines, "build-release")
''',
    '''release_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
package_lines = Path(sys.argv[3]).read_text(encoding="utf-8").splitlines()
release_build = job_block(release_lines, "build-release")
''',
)
replace_exact(
    validate,
    '''assert "  pull_request:" not in release_lines
assert "  workflow_call:" not in release_lines

mirror_lines = Path(sys.argv[3]).read_text(encoding="utf-8").splitlines()
''',
    '''assert "  pull_request:" not in release_lines
assert "  workflow_call:" not in release_lines


def run_blocks(lines):
    blocks = []
    for index, line in enumerate(lines):
        if line.strip() != "run: |":
            continue
        indent = len(line) - len(line.lstrip())
        block = []
        for candidate in lines[index + 1:]:
            candidate_indent = len(candidate) - len(candidate.lstrip())
            if candidate.strip() and candidate_indent <= indent:
                break
            block.append(candidate)
        blocks.append("\\n".join(block))
    return blocks


for workflow_name, lines, expected_resolvers in (
    ("release", release_lines, 3),
    ("package", package_lines, 1),
):
    input_lines = [line.strip() for line in lines if "${{ inputs.version }}" in line]
    assert input_lines == ["REQUESTED_VERSION: ${{ inputs.version }}"] * expected_resolvers, (
        workflow_name,
        input_lines,
    )
    assert lines.count('          requested="${REQUESTED_VERSION:-}"') == expected_resolvers
    assert lines.count('          [[ "${version}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$ ]] || {') == expected_resolvers
    for block in run_blocks(lines):
        assert "${{ inputs.version }}" not in block, (workflow_name, block)
        assert "${{ steps.version.outputs.version }}" not in block, (workflow_name, block)
        assert "${{ steps.version.outputs.bundle_dir }}" not in block, (workflow_name, block)

assert "          APPLIANCE_VERSION: ${{ steps.version.outputs.version }}" in release_lines
assert '          bash scripts/appliance/build.sh "${APPLIANCE_VERSION}" \\' in release_lines
assert "          PACKAGE_VERSION: ${{ steps.version.outputs.version }}" in package_lines
assert '          bash edutalent package "${PACKAGE_VERSION}" 2>&1 | tee package-build.log' in package_lines

mirror_lines = Path(sys.argv[4]).read_text(encoding="utf-8").splitlines()
''',
)

print("Applied protected release identity and workflow-input hardening fixes.")
