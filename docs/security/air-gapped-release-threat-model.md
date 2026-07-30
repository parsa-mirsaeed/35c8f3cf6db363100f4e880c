# Air-gapped release threat model

## Assets

The release process protects:

- EduTalent proprietary source and custom image contents;
- the exact production service and image inventory;
- the pinned local embedding model and vector-space contract;
- the appliance manifest, SBOMs, signatures, and provenance;
- installation secrets and operator TLS keys;
- school data and persistent Docker volumes during install, update, and rollback.

## Trust boundaries

1. **Source repository to connected builder.** GitHub Actions checks out one exact
   commit. A release claim is valid only for that SHA.
2. **Connected builder to upstream registries/model host.** The builder may fetch
   only during assembly. Resolved registry digests and the immutable model commit
   become release inputs.
3. **Builder to release media.** The generated manifest, checksums, SBOMs, and
   signatures bind the payload copied to offline media.
4. **Release media to target host.** The installer verifies the complete payload
   before loading images or generating secrets.
5. **Installer tools container to host files.** The container has no network and
   writes only the mounted appliance configuration paths as the invoking UID/GID.
6. **Locked Compose topology to runtime.** Every service uses an already loaded
   local tag and `pull_policy: never`; only the established AI Gateway egress
   boundary remains non-internal at runtime.

## Threats and controls

### Repository or workflow substitution

An attacker could build an older or different commit and present its artifacts as
the reviewed release.

Controls:

- exact full Git SHA in the release manifest and workflow evidence;
- exact-head AI Change Proof, Full Validation, Package, Production Foundation, and
  Air-gapped Appliance gates;
- protected release tag or explicit `main` dispatch for publication;
- expected-head checks before merging.

### Mutable or substituted container image

A tag can move between assembly and deployment.

Controls:

- source registry digest recorded after the selected platform is pulled;
- deterministic manifest-owned local tag containing a digest prefix;
- archive checksum and size bound into the release manifest;
- locked Compose override uses only local tags and never pulls;
- every running container is compared with the manifest during offline proof.

### Model substitution or unsafe serialization

A model repository can move or include executable/pickle content.

Controls:

- full immutable model commit and safetensors SHA-256 lock;
- allowlist excludes `pytorch_model.bin` and arbitrary files;
- symlinks and executable files are rejected;
- model metadata and complete checksum set are included;
- TEI receives only the packaged read-only local path.

### Extra, missing, or tampered release payload

An attacker could append an unreviewed executable or replace one archive while
leaving a valid signature for another manifest.

Controls:

- manifest inventory includes every non-signature file, mode, size, and checksum;
- verifier requires the actual file set to exactly equal the inventory;
- `SHA256SUMS` must exactly cover that inventory and the manifest;
- duplicate entries, symlinks, private keys, real environment files, credentialed
  database URLs, PDFs, dumps, and build outputs are rejected;
- regression tests prove modified and untracked files fail verification.

### Signature or trust-root confusion

A pull-request proof key could be mistaken for the production release identity.

Controls:

- ephemeral signing is documented and limited to CI mechanism proof;
- protected releases use GitHub OIDC keyless Sigstore identity;
- verification policy pins the GitHub repository workflow identity and issuer;
- GHCR image indexes are signed by digest;
- provenance subjects are registry digests, not tags.

### Secret leakage into Git, images, artifacts, or logs

Build-time test configuration can accidentally enter the bundle.

Controls:

- generated Supabase and EduTalent environments are deleted before copying;
- TLS test material lives in a temporary directory and is never copied;
- release scanning rejects real `.env`, key, certificate, database dump, PDF, and
  credentialed URL payloads;
- the installer creates target secrets only after verification;
- external provider keys are operator-owned and never generated or packaged;
- diagnostics contain manifest metadata and bounded operational logs, not secret
  values.

### Installer network access

A compromised or incomplete bundle could cause the target to contact a registry,
model host, or package server.

Controls:

- installer tools run with `--network none`;
- every image is loaded from a local archive before startup;
- every service uses `pull_policy: never`;
- missing images fail startup rather than invoking a pull;
- model path is a local read-only bind mount;
- offline proof inspects Docker pull events during first startup.

### Cross-platform confusion

An amd64 archive could be installed on arm64 or vice versa.

Controls:

- platform is in every image record and the release manifest;
- installer maps host architecture and rejects mismatch before loading;
- CI separately builds custom images for both supported architectures.

### Compromised upstream image or vulnerable package

Digest pinning guarantees identity, not safety.

Controls:

- SPDX SBOM per image and for the release filesystem;
- immutable source digest and release manifest enable later revocation;
- custom GHCR images carry SBOM and provenance attestations;
- vulnerability policy, scanner definitions, and signed maintenance updates are
  completed in Plan V1 Production Operations.

### Destructive update or false rollback claim

Starting an older binary after a migration may corrupt or fail against newer data.

Controls:

- installer never deletes production volumes;
- releases install side by side;
- documentation explicitly separates image rollback from database rollback;
- full backup, PITR, Qdrant restore, migration rollback, and update drills remain
  mandatory Production Operations acceptance work.

## Residual risks

- A compromised connected builder can still assemble malicious bytes unless
  protected-runner, workflow-review, exact-head, signature, and provenance controls
  all remain effective.
- Third-party images and the model carry their own vulnerability and licensing
  risks even when their bytes are correctly identified.
- Offline media confidentiality is an operator responsibility; the release contains
  proprietary binaries even though it contains no installation secrets or school
  data.
- The complete Supabase and TEI inventory is large. Transfer interruption must be
  handled through checksummed split parts or another controlled local-media process.
