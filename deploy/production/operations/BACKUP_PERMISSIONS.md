# Encrypted backup permission invariant

EduTalent constructs every backup in a temporary payload tree before creating the
tamper-evident manifest and encrypted archive.

The backup copy is intentionally normalized before the manifest is written:

- payload directories use mode `0700`;
- every regular payload file uses mode `0600`;
- symbolic links and non-regular entries are rejected;
- the manifest records the normalized mode, size, and SHA-256 digest;
- verification rejects any later inventory, size, digest, or mode change.

This normalization applies only to the temporary backup copy. It does not modify
live PostgreSQL, Storage, Qdrant, configuration, or WAL source files.

Extraction runs under the operations tool's restrictive `umask 077`. It must not
use `tar --same-permissions`, because restoring arbitrary source permissions could
reintroduce group- or world-readable backup material. Normalized `0700`/`0600`
entries therefore round-trip deterministically while preserving least privilege.
