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

- Docker Engine, Compose v2, Buildx, and QEMU for cross-platform builds;
- Python 3 and `huggingface_hub`;
- `jq`, `openssl`, `syft`, and `cosign`;
- Node.js 16 or newer for the pinned official Supabase key generators.

```bash
EDUTALENT_APPLIANCE_PLATFORM=linux/amd64 \
EDUTALENT_APPLIANCE_SIGNING_MODE=ephemeral \
  bash scripts/appliance/build.sh v1.0.0
```

Tagged protected releases use keyless Sigstore signing and publish custom
multi-architecture images to GHCR. Pull-request validation uses an ephemeral key
only to prove the sign/verify mechanism; that key is not a production trust root.

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

Copy the complete bundle directory or every split archive part to the target host.
Reassemble split archives in lexical order when needed, then:

```bash
./edutalent-appliance verify
./edutalent-appliance load
./edutalent-appliance init
```

The first `init` creates `deploy/production/.env.edutalent`. Set the three domains,
restricted administrative CIDRs, and absolute operator-supplied TLS certificate
and key paths. Run `init` again to generate fresh installation-specific secrets
inside the packaged, network-disabled tools container.

```bash
./edutalent-appliance start
./edutalent-appliance checks
```

Startup uses `pull_policy: never`. A missing archive or image fails before a
registry request can be attempted. The optional local model is already present;
TEI receives a local filesystem path rather than a Hugging Face repository name.

## Updates and rollback

Each version installs beside the previous version. Stop the current appliance,
verify and load the new bundle, retain the existing production environment and
volumes, then start the new version. Rollback selects the previous verified bundle
and starts it against the unchanged data volumes. Database migration rollback and
full backup restoration remain part of Plan V1 Production Operations and must be
proven before a production rollout.

## Release verification

A release is acceptable only when its exact commit passes:

- AI Change Proof;
- Full Validation;
- Package image/archive and repeated migrations;
- complete Production Foundation;
- Air-gapped Appliance definition validation, complete image export, SBOM and
  signature verification, local model verification, and first startup with pulls
  disabled;
- multi-architecture GHCR publication and provenance for a protected release tag.
