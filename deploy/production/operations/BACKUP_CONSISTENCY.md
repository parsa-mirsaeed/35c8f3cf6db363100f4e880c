# Coordinated production backup boundary

EduTalent full backups represent one write-quiesced recovery point across the
application database, Supabase Storage, Qdrant, configuration escrow, and the
completed PostgreSQL WAL archive.

## Capture sequence

`edutalent-operations backup-create`:

1. verifies both the protected backup destination and plaintext staging
   filesystem have the configured free-capacity reserve;
2. records the running writer-facing services;
3. stops the public gateway, application, and Supabase services that can accept
   or process writes;
4. verifies those services are no longer running;
5. keeps PostgreSQL, Qdrant, and the continuous WAL receiver available;
6. captures PostgreSQL logical and physical backups, the stopped Storage volume,
   and a Qdrant collection snapshot;
7. copies the Qdrant snapshot into the protected payload and deletes the
   server-side snapshot, including on later failure paths;
8. copies only completed 24-character WAL segment files and records their exact
   paths for bounded post-commit retirement;
9. restarts exactly the services that were running before capture and verifies
   every recorded container returns to a running/healthy state;
10. normalizes the temporary payload to directory mode `0700` and file mode
    `0600`, writes the manifest, encrypts to a `.partial` archive, and verifies
    that partial archive by decryption and manifest replay;
11. publishes the final archive and metadata names only after verification,
    verifies the sidecar-to-archive SHA-256 digest, and then retires only the
    included local WAL segments older than the configured retained tail.

Every error path attempts to delete any live Qdrant snapshot, restore the
previously running services, remove plaintext staging, and remove unpublished or
incompletely committed archive names before the command returns. PostgreSQL and
Qdrant are deliberately not stopped: PostgreSQL backup tools require the
database, and the Qdrant snapshot API creates the consistent vector-store
checkpoint while application writes are quiesced.

## Protected staging and capacity

Plaintext backup creation, archive verification, and logical restore drills use
`EDUTALENT_BACKUP_STAGING_DIR`. When it is unset, the directory defaults to
`.staging` below `EDUTALENT_BACKUP_DIR`, keeping large temporary payloads on the
separate protected backup filesystem rather than `/tmp`. Both paths must be
absolute, outside the deployment and immutable release, and mode `0700`.

`EDUTALENT_BACKUP_MIN_FREE_BYTES` protects the final destination.
`EDUTALENT_BACKUP_STAGING_MIN_FREE_BYTES` protects staging and defaults to the
same reserve. A distinct staging filesystem is rejected before quiescence when
it cannot satisfy that reserve.

## WAL retirement boundary

A completed full backup contains a physical base backup with fetched required
WAL plus every completed receiver segment selected for that generation. Local
segments are not retired until the encrypted archive has passed decryption,
manifest verification, final publication, and sidecar checksum verification.
Only the exact files copied into that generation are eligible; segments completed
concurrently after inventory remain untouched. `EDUTALENT_WAL_RETAIN_SEGMENTS`
keeps the newest included local segments and defaults to `2`, preventing
monotonic operations-disk growth while retaining a short continuity tail.
Encrypted backup retention remains governed separately by the configured days
and minimum-copy policy.

## Monitoring contract

A backup is fresh only when the newest metadata sidecar names a regular archive
inside the configured backup directory and that archive matches the recorded
SHA-256 digest. Monitoring also evaluates the backup disk independently from the
operations-state disk, treats unknown TLS and database-metric state as critical,
and requires the WAL receiver to be running in addition to having a recent
completed segment. Qdrant and AI Gateway outages remain degraded warning states
when core school services are healthy.

## Restore cleanup

Restore drills use one cleanup handler for the temporary drill database, the
container-side dump copy, and the host-side decrypted payload. The host payload
uses the protected capacity-checked staging directory. This handler runs on
successful and failed drills so plaintext configuration escrow is not retained.
