#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def replace_exact(path: Path, old: str, new: str, expected_count: int = 1) -> None:
    text = path.read_text(encoding="utf-8")
    actual = text.count(old)
    if actual != expected_count:
        raise RuntimeError(f"{path}: expected {expected_count} match(es), found {actual}")
    path.write_text(text.replace(old, new), encoding="utf-8")


release = ROOT / ".github/workflows/air-gapped-release.yml"
validate = ROOT / "scripts/appliance/validate.sh"

secret_scan_job = '''
  secret-scan:
    name: Scan complete repository history for secrets
    needs: release-policy
    permissions:
      contents: read
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ github.event.pull_request.head.sha || github.sha }}
          fetch-depth: 0
          show-progress: false

      - name: Install verified Gitleaks
        shell: bash
        env:
          GITLEAKS_VERSION: "8.30.1"
          GITLEAKS_LINUX_X64_SHA256: "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb"
        run: |
          set -euo pipefail
          archive="${RUNNER_TEMP}/gitleaks_${GITLEAKS_VERSION}_linux_x64.tar.gz"
          install_dir="${RUNNER_TEMP}/gitleaks-bin"
          curl --fail --show-error --silent --location \\
            --proto '=https' --tlsv1.2 \\
            "https://github.com/gitleaks/gitleaks/releases/download/v${GITLEAKS_VERSION}/gitleaks_${GITLEAKS_VERSION}_linux_x64.tar.gz" \\
            --output "${archive}"
          printf '%s  %s\\n' "${GITLEAKS_LINUX_X64_SHA256}" "${archive}" | sha256sum --check -
          mkdir -p "${install_dir}"
          tar --extract --gzip --file "${archive}" --directory "${install_dir}" gitleaks
          chmod 0755 "${install_dir}/gitleaks"
          echo "${install_dir}" >> "${GITHUB_PATH}"

      - name: Scan all Git history with redacted output
        shell: bash
        run: |
          set -euo pipefail
          gitleaks detect --source . --redact --no-banner --log-opts="--all" \\
            2>&1 | tee gitleaks-all-history.log

      - name: Upload redacted secret-scan diagnostics
        if: always()
        uses: actions/upload-artifact@v6
        with:
          name: air-gapped-release-secret-scan
          path: gitleaks-all-history.log
          if-no-files-found: error
          retention-days: 7
'''

replace_exact(release, "\n  validate:\n", secret_scan_job + "\n  validate:\n")
replace_exact(
    release,
    '''    needs:
      - validate
      - release-policy
''',
    '''    needs:
      - validate
      - release-policy
      - secret-scan
''',
    expected_count=2,
)
replace_exact(
    release,
    '''    needs:
      - release-policy
      - validate
''',
    '''    needs:
      - release-policy
      - secret-scan
      - validate
''',
)
replace_exact(
    release,
    '''          POLICY_RESULT: ${{ needs.release-policy.result }}
          VALIDATE_RESULT: ${{ needs.validate.result }}
''',
    '''          POLICY_RESULT: ${{ needs.release-policy.result }}
          SECRET_SCAN_RESULT: ${{ needs.secret-scan.result }}
          VALIDATE_RESULT: ${{ needs.validate.result }}
''',
)
replace_exact(
    release,
    '''          test "${POLICY_RESULT}" = success
          test "${VALIDATE_RESULT}" = success
''',
    '''          test "${POLICY_RESULT}" = success
          test "${SECRET_SCAN_RESULT}" = success
          test "${VALIDATE_RESULT}" = success
''',
)

replace_exact(
    validate,
    '''grep -Fq 'attestations: write' "${release_workflow}"
''',
    '''grep -Fq 'attestations: write' "${release_workflow}"
grep -Fq 'GITLEAKS_VERSION: "8.30.1"' "${release_workflow}"
grep -Fq 'GITLEAKS_LINUX_X64_SHA256: "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb"' "${release_workflow}"
grep -Fq 'gitleaks detect --source . --redact --no-banner --log-opts="--all"' "${release_workflow}"
''',
)
replace_exact(
    validate,
    '''release_build = job_block(release_lines, "build-release")
assert "      id-token: write" in release_build
assert "          EDUTALENT_APPLIANCE_SIGNING_MODE: keyless" in release_build
publish_platforms = job_block(release_lines, "publish-platforms")
''',
    '''secret_scan = job_block(release_lines, "secret-scan")
assert "    needs: release-policy" in secret_scan
assert "      contents: read" in secret_scan
assert "          fetch-depth: 0" in secret_scan
assert '          GITLEAKS_VERSION: "8.30.1"' in secret_scan
assert '          GITLEAKS_LINUX_X64_SHA256: "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb"' in secret_scan
assert any('gitleaks detect --source . --redact --no-banner --log-opts="--all"' in line for line in secret_scan)
assert "        if: always()" in secret_scan
assert "          name: air-gapped-release-secret-scan" in secret_scan
assert not any("continue-on-error" in line for line in secret_scan)

release_build = job_block(release_lines, "build-release")
assert "      id-token: write" in release_build
assert "          EDUTALENT_APPLIANCE_SIGNING_MODE: keyless" in release_build
assert "      - secret-scan" in release_build
native_arm64 = job_block(release_lines, "native-arm64")
assert "      - secret-scan" in native_arm64
publish_platforms = job_block(release_lines, "publish-platforms")
''',
)
replace_exact(
    validate,
    '''assert "  pull_request:" not in release_lines
assert "  workflow_call:" not in release_lines
''',
    '''release_gate = job_block(release_lines, "gate")
assert "      - secret-scan" in release_gate
assert "          SECRET_SCAN_RESULT: ${{ needs.secret-scan.result }}" in release_gate
assert '          test "${SECRET_SCAN_RESULT}" = success' in release_gate
assert "  pull_request:" not in release_lines
assert "  workflow_call:" not in release_lines
''',
)

print("Applied protected all-history secret-scan gate and regression checks.")
