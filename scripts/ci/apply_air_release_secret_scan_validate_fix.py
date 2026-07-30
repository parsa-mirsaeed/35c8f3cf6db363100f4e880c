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
    '''ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
''',
    '''ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
gitleaks_ignore="${ROOT_DIR}/.gitleaksignore"
python3 - "${gitleaks_ignore}" <<'PYIGNORE'
import sys
from pathlib import Path

expected = {
    "2c8b0895d182b4b02bb79c8cad54290a7c0bb75c:scripts/appliance/validate.sh:private-key:314",
    "2c8b0895d182b4b02bb79c8cad54290a7c0bb75c:scripts/appliance/validate.sh:private-key:317",
    "242aafbeddaa27df39f4b4b1b75e846f6ea5aed0:scripts/ci/oidc-fix-staging/validate.sh:private-key:747",
    "242aafbeddaa27df39f4b4b1b75e846f6ea5aed0:scripts/ci/oidc-fix-staging/validate.sh:private-key:750",
    "242aafbeddaa27df39f4b4b1b75e846f6ea5aed0:scripts/ci/oidc-fix-staging/validate.sh:private-key:753",
    "71b285a61d0969b3b939669b0e21489310c2f853:scripts/ci/air-release-staging/validate.sh:private-key:752",
    "71b285a61d0969b3b939669b0e21489310c2f853:scripts/ci/air-release-staging/validate.sh:private-key:755",
    "71b285a61d0969b3b939669b0e21489310c2f853:scripts/ci/air-release-staging/validate.sh:private-key:758",
    "70c08cd26b364931ba1b70358bb939de71b625ca:scripts/ci/mirror-air-input-staging/validate.sh:private-key:756",
    "70c08cd26b364931ba1b70358bb939de71b625ca:scripts/ci/mirror-air-input-staging/validate.sh:private-key:759",
    "70c08cd26b364931ba1b70358bb939de71b625ca:scripts/ci/mirror-air-input-staging/validate.sh:private-key:762",
    "49bcb0396bafdadc343fe264f22f1a7993460b5c:.github/workflows/fix-release-portability-secrets.yml:private-key:102",
    "49bcb0396bafdadc343fe264f22f1a7993460b5c:.github/workflows/fix-release-portability-secrets.yml:private-key:105",
    "da9595ffef90902ccbcd70262d1c4b580e46dadd:.github/workflows/fix-release-portability-secrets.yml:private-key:97",
    "da9595ffef90902ccbcd70262d1c4b580e46dadd:.github/workflows/fix-release-portability-secrets.yml:private-key:100",
    "ad71af2047fd77bbd7edaef7ae36c7e516155ee7:ci-transfer-mirror-air/validate.sh:private-key:756",
    "ad71af2047fd77bbd7edaef7ae36c7e516155ee7:ci-transfer-mirror-air/validate.sh:private-key:759",
    "ad71af2047fd77bbd7edaef7ae36c7e516155ee7:ci-transfer-mirror-air/validate.sh:private-key:762",
    "920ecd8bf53965cd9078ce162140df759da85786:deploy/production/edutalent-production:curl-auth-header:271",
    "c08d637fdb1330f674698f467729eb8c78ec1e87:deploy/production/edutalent-production:curl-auth-header:255",
}
lines = [line.strip() for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines() if line.strip()]
assert len(lines) == len(set(lines)), "duplicate Gitleaks fingerprints"
assert set(lines) == expected, "Gitleaks ignore baseline must contain only reviewed exact fingerprints"
PYIGNORE
''',
)
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
