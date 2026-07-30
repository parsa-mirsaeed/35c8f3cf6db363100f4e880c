#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
validate = ROOT / "scripts/appliance/validate.sh"


def replace_exact(old: str, new: str, expected_count: int = 1) -> None:
    text = validate.read_text(encoding="utf-8")
    actual = text.count(old)
    if actual != expected_count:
        raise RuntimeError(f"{validate}: expected {expected_count} match(es), found {actual}")
    validate.write_text(text.replace(old, new), encoding="utf-8")


replace_exact(
    '''grep -Fq 'attestations: write' "${release_workflow}"
''',
    '''grep -Fq 'attestations: write' "${release_workflow}"
grep -Fq 'GITLEAKS_VERSION: "8.30.1"' "${release_workflow}"
grep -Fq 'GITLEAKS_LINUX_X64_SHA256: "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb"' "${release_workflow}"
grep -Fq 'gitleaks detect --source . --redact --no-banner --log-opts="--all"' "${release_workflow}"
''',
)
replace_exact(
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

print("Applied protected release secret-scan regression checks.")
