# ADR 0003: Signed air-gapped appliance and GHCR delivery

- Status: Accepted
- Date: 2026-07-28
- Supersedes: the application-image-only release as the production delivery unit
- Related: ADR 0001 and ADR 0002

## Context

The production topology is an offline-first school appliance. The existing
`make package` archive contains only the EduTalent application image and a
lightweight Compose file. A target host still needs registry access for Caddy,
Supabase, PostgreSQL, Qdrant, TEI, and supporting services, and TEI downloads its
model at runtime. That package therefore cannot establish the approved
strict-network production boundary.

The release also needs verifiable answers to four questions:

1. Which exact source revision produced the appliance?
2. Which exact image and model bytes are allowed to run?
3. Can the target install and start without registry or model access?
4. Can an operator verify integrity, authorship, software inventory, and build
   provenance before introducing the appliance to a school network?

## Decision

EduTalent keeps two explicit release editions.

### Thin application bundle

The existing `make package` path remains for connected development and migration
verification. It is not represented as an air-gapped production appliance.

### Full offline appliance

A trusted connected builder renders the complete pinned production topology,
including optional and profile-gated services selected for delivery. For each
unique image it:

- pulls or builds one platform-specific image;
- records the source registry digest or custom image content digest;
- assigns a deterministic local tag containing the release, architecture, and
  digest prefix;
- exports the locally tagged image as a compressed archive;
- creates an SPDX image SBOM.

The builder downloads the optional local BGE model only at the full locked Hugging
Face commit, allows only safe model files, verifies the safetensors checksum, and
writes model metadata and a complete checksum set. Pickle weights are excluded.

The appliance contains the pinned official Supabase runtime, production overlay,
environment templates, offline installer, all image archives, model artifacts,
image and filesystem SBOMs, licence notices, and a generated Compose override.
Every service in that override uses a manifest-owned local tag and
`pull_policy: never`. TEI receives a read-only local model path.

One immutable JSON manifest binds:

- release version, exact Git SHA, and platform;
- every source image reference and digest;
- every local tag, service mapping, archive checksum, and image SBOM checksum;
- model repository, commit, dimensions, and primary weight checksum;
- every non-signature bundle file, size, mode, and checksum.

`SHA256SUMS` covers the exact manifest inventory plus the manifest itself. The
verifier rejects missing, modified, duplicate, forbidden, symlinked, or untracked
payloads. Signatures are intentionally outside the signed inventory to avoid a
recursive signature dependency.

Pull-request proof uses a one-time ephemeral cosign key and packages only the
public key. This proves the sign/verify mechanism but is not a production trust
root. Protected release tags use keyless Sigstore signing through GitHub OIDC.
Published custom GHCR image indexes are also keylessly signed.

The custom EduTalent runtime image and the offline installer-tools image are built
for `linux/amd64` and `linux/arm64`. Protected version tags publish only versioned
and exact-commit tags to GHCR; no `latest` tag is produced. Buildx emits SBOM and
provenance attestations, and GitHub produces build-provenance attestations tied to
the registry digest.

Installation secrets are generated only on the target host through the packaged
tools image with `--network none`. Operator TLS keys remain outside the release.
The installer refuses a wrong-platform appliance, loads every image archive
locally, and starts through the locked Compose override.

## Acceptance proof

Final review must build the complete amd64 appliance, remove its local image tags,
load them from the archives, generate fresh non-production secrets through the
offline tools image, and start the complete production topology. The proof must
show:

- every running container uses a manifest-owned local tag;
- `pull_policy: never` applies to every selected service;
- no Docker image-pull event occurs during startup;
- TEI uses only the packaged model directory;
- production preflight, migrations, constrained database role, gateway, Auth,
  Qdrant, AI outage, and AI recovery checks still pass;
- both custom images compile for amd64 and arm64.

GHCR publication and permanent release signing occur only for a protected tag or
an explicitly approved dispatch from `main`. Pull-request validation never
publishes mutable product images.

## Consequences

- The complete appliance is substantially larger than the thin bundle and may
  require split archive parts for GitHub delivery limits.
- A connected, trusted release builder is still required to materialize upstream
  images and the pinned model before transfer to the air gap.
- Third-party registry signatures are not reissued as upstream signatures. The
  EduTalent release manifest records the resolved source digest, while the
  appliance signature attests to the assembled release.
- A release remains platform-specific because image archives contain one selected
  architecture. Multi-architecture GHCR indexes are a separate connected delivery
  channel.
- Database rollback, backup restoration, and operational lifecycle validation are
  intentionally deferred to Plan V1 Production Operations; the installer does not
  pretend that reverting binaries can reverse a migration.
