#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def replace_exact(path: Path, old: str, new: str, expected_count: int = 1) -> None:
    text = path.read_text(encoding="utf-8")
    actual = text.count(old)
    if actual != expected_count:
        raise RuntimeError(f"{path}: expected {expected_count} match(es), found {actual}")
    path.write_text(text.replace(old, new), encoding="utf-8")


offline = ROOT / "scripts/appliance/offline_smoke.sh"
offline_lines = offline.read_text(encoding="utf-8").splitlines()
keyless_start = offline_lines.index("    keyless)")
keyless_end = offline_lines.index("      ;;", keyless_start)
old_keyless = offline_lines[keyless_start:keyless_end + 1]
if not any("air-gapped-appliance" in line for line in old_keyless):
    raise RuntimeError("Unexpected offline smoke keyless identity block")
new_keyless = [
    "    keyless)",
    '      : "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required for CI keyless verification}"',
    '      : "${GITHUB_REF:?GITHUB_REF is required for CI keyless verification}"',
    '      expected_workflow_ref="${GITHUB_REPOSITORY}/.github/workflows/air-gapped-release.yml@${GITHUB_REF}"',
    '      [[ "${GITHUB_WORKFLOW_REF:-}" == "${expected_workflow_ref}" ]] || {',
    '        echo "Keyless appliance smoke verification is restricted to ${expected_workflow_ref}." >&2',
    "        exit 1",
    "      }",
    '      trusted_identity="https://github.com/${expected_workflow_ref}"',
    '      export EDUTALENT_APPLIANCE_TRUSTED_OIDC_ISSUER="https://token.actions.githubusercontent.com"',
    '      export EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP="$(' ,
    "        python3 -c 'import re, sys; print(\"^\" + re.escape(sys.argv[1]) + \"$\")' \\",
    '          "${trusted_identity}"',
    '      )"',
    "      ;;",
]
offline.write_text(
    "\n".join(offline_lines[:keyless_start] + new_keyless + offline_lines[keyless_end + 1:]) + "\n",
    encoding="utf-8",
)

readme = ROOT / "deploy/appliance/README.md"
replace_exact(
    readme,
    "export EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP='^https://github.com/<owner>/<repository>/\\.github/workflows/air-gapped-appliance\\.yml@refs/tags/v.*$'",
    "export EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP='^https://github.com/<owner>/<repository>/\\.github/workflows/air-gapped-release\\.yml@refs/tags/v[0-9A-Za-z._-]+$'",
)
replace_exact(
    readme,
    '''The target host requires Docker Engine with Compose v2, Python 3, GNU tar/gzip,
and a trusted `cosign` installation for signature verification. Copy the complete
bundle directory or every split archive part to the target host. Reassemble split
archives in lexical order when needed, then:

```bash
./edutalent-appliance verify
./edutalent-appliance load
./edutalent-appliance init
```
''',
    '''The target host requires Docker Engine with Compose v2, Python 3, GNU tar/gzip,
and a trusted `cosign` installation for signature verification. Copy the complete
bundle directory, or the release archive/checksum/signature objects, to the target
host through the controlled transfer process.

**Authenticate every transferred archive object before reassembly or extraction.**
Do not extract an archive and then rely only on the verifier contained inside it.
For a protected keyless release, obtain the issuer and exact release-workflow
identity from an independently authenticated release policy, then define a trusted
host-side verifier:

```bash
issuer='https://token.actions.githubusercontent.com'
identity='^https://github.com/<owner>/<repository>/\.github/workflows/air-gapped-release\.yml@refs/tags/v[0-9A-Za-z._-]+$'
verify_payload() {
  local payload="$1"
  cosign verify-blob \
    --certificate-oidc-issuer "${issuer}" \
    --certificate-identity-regexp "${identity}" \
    --bundle "${payload}.sigstore.json" \
    "${payload}"
}
```

For one unsplit archive, verify both the signed checksum manifest and the archive,
then check the archive bytes before extraction:

```bash
bundle_name='edutalent-appliance-v1.0.0-amd64'
archive="${bundle_name}.tar.gz"
checksum_file="${archive}.SHA256SUMS"
verify_payload "${checksum_file}"
verify_payload "${archive}"
sha256sum --check "${checksum_file}"
```

For split delivery, first verify the signed checksum manifest. Use only the exact
part names authenticated by that manifest, verify every part signature, verify every
part checksum, and only then concatenate them:

```bash
bundle_name='edutalent-appliance-v1.0.0-amd64'
archive="${bundle_name}.tar.gz"
checksum_file="${bundle_name}.parts.SHA256SUMS"
verify_payload "${checksum_file}"
mapfile -t parts < <(awk 'NF == 2 { print $2 }' "${checksum_file}")
((${#parts[@]} > 0))
for part in "${parts[@]}"; do
  [[ "${part}" == "${archive}.part-"[0-9][0-9][0-9] ]]
  verify_payload "${part}"
done
sha256sum --check "${checksum_file}"
cat -- "${parts[@]}" > "${archive}"
```

Extract the already authenticated archive into a new empty directory. Then run the
in-bundle verifier, which independently authenticates the signed immutable manifest
and payload inventory, before loading or initializing anything:

```bash
install_root="$(mktemp -d)"
tar --extract --gzip --file "${archive}" --directory "${install_root}" \
  --no-same-owner --no-same-permissions
cd "${install_root}/${bundle_name}"
./edutalent-appliance verify
./edutalent-appliance load
./edutalent-appliance init
```

The public mirror's ephemeral `.sig`/`.pub` objects are validation-only and are not a
production trust root. Production transfers must use the protected keyless release
objects and the independently obtained issuer/identity policy above.
''',
)

validate = ROOT / "scripts/appliance/validate.sh"
replace_exact(
    validate,
    '''if grep -Fq 'air-gapped-appliance\\.yml@${GITHUB_REF}' "${ROOT_DIR}/scripts/appliance/build.sh"; then
  echo "Keyless verification must trust the protected release workflow identity." >&2
  exit 1
fi
''',
    '''if grep -Fq 'air-gapped-appliance\\.yml@${GITHUB_REF}' "${ROOT_DIR}/scripts/appliance/build.sh"; then
  echo "Keyless verification must trust the protected release workflow identity." >&2
  exit 1
fi
grep -Fq 'GITHUB_WORKFLOW_REF' "${ROOT_DIR}/scripts/appliance/offline_smoke.sh"
grep -Fq 'air-gapped-release.yml@${GITHUB_REF}' "${ROOT_DIR}/scripts/appliance/offline_smoke.sh"
if grep -Fq 'air-gapped-appliance\\.yml@${GITHUB_REF}' "${ROOT_DIR}/scripts/appliance/offline_smoke.sh"; then
  echo "Offline smoke verification must trust the protected release workflow identity." >&2
  exit 1
fi
appliance_readme="${ROOT_DIR}/deploy/appliance/README.md"
grep -Fq 'air-gapped-release\\.yml@refs/tags/' "${appliance_readme}"
if grep -Fq 'air-gapped-appliance\\.yml@refs/tags/' "${appliance_readme}"; then
  echo "Operator documentation contains the obsolete appliance-workflow identity." >&2
  exit 1
fi
grep -Fq 'Authenticate every transferred archive object before reassembly or extraction.' "${appliance_readme}"
grep -Fq -- '--bundle "${payload}.sigstore.json"' "${appliance_readme}"
grep -Fq 'verify_payload "${checksum_file}"' "${appliance_readme}"
grep -Fq 'verify_payload "${part}"' "${appliance_readme}"
grep -Fq 'sha256sum --check "${checksum_file}"' "${appliance_readme}"
grep -Fq 'cat -- "${parts[@]}" > "${archive}"' "${appliance_readme}"
''',
)

print("Applied release smoke identity and pre-extraction verification fixes.")
