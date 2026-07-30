# EduTalent air-gapped appliance

The appliance is built on a connected, trusted release runner and installed on a
supported Linux host without registry or model access. It contains every selected
production image, the pinned local embedding model, the pinned Supabase runtime,
production configuration templates, image and filesystem SBOMs, an immutable
release manifest, checksums, signatures, and the installation command.

## Trust boundary

The public mirror is a validation environment. A real product release must be
reproduced from the authoritative private repository and signed by its protected
release workflow. The appliance never contains generated installation secrets,
TLS private keys, provider credentials, database dumps, PDFs, or personal data.

## Connected build

Required tools on the release runner:

- Docker Engine, Compose v2, and Buildx;
- a native Linux runner for each released architecture;
- Python 3 and `huggingface_hub`;
- `jq`, `openssl`, `syft`, and `cosign`;
- Node.js 16 or newer for the pinned official Supabase key generators.

QEMU is not used for release architecture proof. The public validation workflow
builds amd64 on `ubuntu-24.04` and arm64 on GitHub's native
`ubuntu-24.04-arm` runner. This avoids emulating the Rust/Dioxus build and keeps
architecture failures separate from emulation performance.

```bash
EDUTALENT_APPLIANCE_PLATFORM=linux/amd64 \
EDUTALENT_APPLIANCE_SIGNING_MODE=ephemeral \
  bash scripts/appliance/build.sh v1.0.0
```

Tagged protected releases use keyless Sigstore signing and publish custom
multi-architecture images to GHCR. Pull-request validation uses an ephemeral key
only to prove the sign/verify mechanism; that key is not a production trust root.
The signed manifest binds the accepted signing mode, and installed appliances
reject ephemeral signatures unless the operator independently sets
`EDUTALENT_APPLIANCE_ALLOW_EPHEMERAL_SIGNATURES=true`. Never enable that flag for a
production release.

Keyless verification never accepts issuer or workflow identity from the bundle.
Obtain the approved values independently from the authoritative release policy and
export them before verifying a tagged release:

```bash
export EDUTALENT_APPLIANCE_TRUSTED_OIDC_ISSUER=https://token.actions.githubusercontent.com
export EDUTALENT_APPLIANCE_TRUSTED_IDENTITY_REGEXP='^https://github.com/<owner>/<repository>/\.github/workflows/air-gapped-release\.yml@refs/tags/v[0-9A-Za-z._-]+$'
./edutalent-appliance verify
```

Treat the verifier command and expected policy as trust-bootstrap material: obtain
them through an independently authenticated channel rather than learning either
value from the appliance being checked.

The connected builder stages production definitions in a temporary directory,
bootstraps Supabase there, and removes the staging tree on exit. Existing production
configuration and generated secrets in the checkout are never overwritten or
deleted by an appliance build.

## Staged validation

Draft pull-request commits run only the fast definition, syntax, lock-file,
integrity-fixture, package, and production-topology gates. They do not rebuild the
complete appliance.

When the implementation is stable, apply `full-validation` and mark the pull
request ready. The mirror proof then enforces one exact-head sequence:

1. AI Change Proof, Full Validation, and complete Production Foundation;
2. complete Package image/archive and repeated migrations;
3. complete amd64 offline appliance plus native arm64 custom-image proof.

Package runs before the appliance so the amd64 runtime build can reuse the same
GitHub Actions BuildKit cache. GHCR publication is not performed on pull requests;
it is restricted to protected `v*` tags or an explicitly approved dispatch from
`main`.

## Bundle layout

```text
edutalent-appliance-<version>-<arch>/
├── edutalent-appliance
├── deploy/production/
├── images/*.tar.gz
├── manifests/images.json
├── manifests/compose.locked.yaml
├── manifests/release-manifest.json
├── models/local-bge-v1/
├── sbom/images/*.spdx.json
├── sbom/release-files.spdx.json
├── signatures/
├── THIRD_PARTY_NOTICES.md
└── SHA256SUMS
```

Every runtime service in the selected production and optional profiles receives a
local immutable tag in `compose.locked.yaml` and `pull_policy: never`. The manifest
binds each local tag and archive checksum to the source registry digest. The TEI
service reads the model only from the packaged read-only directory.

## Offline installation

The target host requires Docker Engine with Compose v2, Python 3, GNU tar/gzip,
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
  cosign verify-blob     --certificate-oidc-issuer "${issuer}"     --certificate-identity-regexp "${identity}"     --bundle "${payload}.sigstore.json"     "${payload}"
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
tar --extract --gzip --file "${archive}" --directory "${install_root}"   --no-same-owner --no-same-permissions
cd "${install_root}/${bundle_name}"
./edutalent-appliance verify
./edutalent-appliance load
./edutalent-appliance init
```

The public mirror's ephemeral `.sig`/`.pub` objects are validation-only and are not a
production trust root. Production transfers must use the protected keyless release
objects and the independently obtained issuer/identity policy above.

The first `init` creates the external application environment under the stable
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

```bash
./edutalent-appliance start
./edutalent-appliance checks
```

Startup uses `pull_policy: never`. A missing archive or image fails before a
registry request can be attempted. The optional local model is already present;
TEI receives a local filesystem path rather than a Hugging Face repository name.

`install <directory>` accepts only a new or empty destination and verifies the
copied installation before reporting success. This prevents stale files from a
previous release remaining outside the signed manifest.

## Updates and rollback

Each version installs beside the previous version. Stop the current appliance,
verify and load the new bundle, and run the new launcher as the same operating-system
user with the same `XDG_STATE_HOME` or explicit `EDUTALENT_APPLIANCE_STATE_DIR`.
Confirm that `state-dir` prints the same path for both versions before startup. The
new version then reuses the existing application environment, Supabase secrets, and
named data volumes rather than generating replacement credentials. Rollback selects
the previous verified bundle with that same state directory and unchanged data
volumes. Database migration rollback and full backup restoration remain part of
Plan V1 Production Operations and must be proven before a production rollout.

## Release verification

A release is acceptable only when its exact commit passes:

- AI Change Proof;
- Full Validation;
- Package image/archive and repeated migrations;
- complete Production Foundation;
- Air-gapped Appliance definition validation, complete image export, SBOM and
  signature verification, local model verification, and first startup with pulls
  disabled;
- native amd64 and arm64 custom-image proof;
- multi-architecture GHCR publication, signatures, SBOMs, and provenance for a
  protected release tag.
