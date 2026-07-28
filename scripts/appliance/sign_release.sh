#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: sign_release.sh <bundle-dir> <ephemeral|keyless>

Ephemeral mode proves signing and verification during pull-request validation and
packages only the generated public key. Keyless mode uses GitHub OIDC and emits a
Sigstore bundle suitable for tagged releases. The accepted keyless issuer and
workflow identity are intentionally not packaged; verifiers must obtain that trust
policy independently.
USAGE
}

bundle_dir="${1:-}"
mode="${2:-}"
[[ -n "${bundle_dir}" && -n "${mode}" ]] || { usage >&2; exit 2; }
command -v cosign >/dev/null 2>&1 || { echo "cosign is required" >&2; exit 1; }
bundle_dir="$(cd "${bundle_dir}" && pwd)"
manifest="${bundle_dir}/manifests/release-manifest.json"
sums="${bundle_dir}/SHA256SUMS"
signatures="${bundle_dir}/signatures"
[[ -f "${manifest}" && -f "${sums}" ]] || { echo "manifest and checksums are required" >&2; exit 1; }
rm -rf "${signatures}"
mkdir -p "${signatures}"

case "${mode}" in
  ephemeral)
    temp_dir="$(mktemp -d)"
    trap 'rm -rf "${temp_dir}"' EXIT
    COSIGN_PASSWORD='' cosign generate-key-pair --output-key-prefix "${temp_dir}/verification" >/dev/null
    COSIGN_PASSWORD='' cosign sign-blob --yes \
      --key "${temp_dir}/verification.key" \
      --output-signature "${signatures}/release-manifest.sig" \
      "${manifest}" >/dev/null
    COSIGN_PASSWORD='' cosign sign-blob --yes \
      --key "${temp_dir}/verification.key" \
      --output-signature "${signatures}/SHA256SUMS.sig" \
      "${sums}" >/dev/null
    cp "${temp_dir}/verification.pub" "${signatures}/verification.pub"
    cosign verify-blob --key "${signatures}/verification.pub" \
      --signature "${signatures}/release-manifest.sig" "${manifest}" >/dev/null
    cosign verify-blob --key "${signatures}/verification.pub" \
      --signature "${signatures}/SHA256SUMS.sig" "${sums}" >/dev/null
    ;;
  keyless)
    cosign sign-blob --yes \
      --bundle "${signatures}/release-manifest.sigstore.json" \
      "${manifest}" >/dev/null
    cosign sign-blob --yes \
      --bundle "${signatures}/SHA256SUMS.sigstore.json" \
      "${sums}" >/dev/null
    ;;
  *) usage >&2; exit 2 ;;
esac

find "${signatures}" -type f -exec chmod 0644 {} +
echo "Signed release manifest and checksum set using ${mode} mode."
