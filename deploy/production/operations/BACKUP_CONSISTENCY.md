# Coordinated production backup boundary

EduTalent full backups represent one write-quiesced recovery point across the
application database, Supabase Storage, Qdrant, configuration escrow, and the
completed PostgreSQL WAL archive.

## Capture sequence

`edutalent-operations backup-create`:

1. records the running writer-facing services;
2. stops the public gateway, application, and Supabase services that can accept
   or process writes;
3. verifies those services are no longer running;
4. keeps PostgreSQL, Qdrant, and the continuous WAL receiver available;
5. captures PostgreSQL logical and physical backups, the stopped Storage volume,
   and a Qdrant collection snapshot;
6. copies only completed 24-character WAL segment files;
7. restarts exactly the services that were running before capture and verifies
   application recovery;
8. normalizes the temporary payload to directory mode `0700` and file mode
   `0600`, writes the manifest, encrypts the archive, and verifies both the
   decrypted manifest and sidecar-to-archive SHA-256 digest.

Every error path attempts to restore the previously running services before the
command returns. PostgreSQL and Qdrant are deliberately not stopped: PostgreSQL
backup tools require the database, and the Qdrant snapshot API creates the
consistent vector-store checkpoint while application writes are quiesced.

## Monitoring contract

A backup is fresh only when the newest metadata sidecar names a regular archive
inside the configured backup directory and that archive matches the recorded
SHA-256 digest. Monitoring also evaluates the backup disk independently from the
operations-state disk, treats unknown TLS state as critical, and requires the WAL
receiver to be running in addition to having a recent completed segment.

## Restore cleanup

Restore drills use one cleanup handler for the temporary drill database, the
container-side dump copy, and the host-side decrypted payload. This handler runs
on successful and failed drills so plaintext configuration escrow is not left in
`/tmp`.
