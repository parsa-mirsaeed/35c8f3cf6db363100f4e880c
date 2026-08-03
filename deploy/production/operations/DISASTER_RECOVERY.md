# Disaster recovery runbook

## Safety rules

1. Declare an incident owner and record the current release digest, exact time, affected services, last verified backup, last WAL receipt, and observed data loss window.
2. Preserve the failed disk and logs. Do not repeatedly restart or run migrations on a possibly corrupt database.
3. Restore onto new storage and a separate Compose project first. Never overwrite the only copy of production data during diagnosis.
4. Keep the network isolated. Recovery must not introduce registry pulls, general egress, public database ports, or temporary default credentials.
5. Verify tenant isolation, RLS, publication state, Qdrant filters, and browser secret exclusion before reopening access.

## Recovery decision

Use the newest verified logical backup for portable object-level recovery. Use the physical base backup plus WAL archive for point-in-time recovery when the required target is after the full backup. Restore Supabase Storage from the same backup generation. Restore Qdrant from its matching snapshot; if that snapshot is unavailable or inconsistent, rebuild vectors from PostgreSQL-authorized assets rather than accepting unverified vector state.

Target order:

1. PostgreSQL;
2. Storage files;
3. authentication and application checks;
4. Qdrant snapshot or controlled reindex;
5. gateway and administration boundary;
6. AI Gateway last, because external AI is not a core health dependency.

## PostgreSQL point-in-time recovery

1. Select a verified encrypted archive and copy the archive, its metadata, and all subsequent WAL files to an isolated recovery host.
2. Verify the archive checksum and decrypt/manifest verification with `backup-verify`.
3. Extract `database/base.tar` into a new empty PostgreSQL data directory owned by the PostgreSQL service identity.
4. Place the corresponding WAL archive on a read-only recovery path.
5. Add `recovery.signal` and configure:

```text
restore_command = 'cp /recovery-wal/%f %p'
recovery_target_time = '<approved UTC target>'
recovery_target_action = 'promote'
```

6. Start the pinned PostgreSQL image with no public port and no application attached.
7. Confirm recovery reaches the target and promotes. Record the achieved timestamp and timeline.
8. Run consistency queries, migration checksum verification, RLS/security invariants, and an application-role privilege check.
9. Do not reuse the original physical replication slot blindly. Recreate the WAL receiver against the promoted timeline after the restored database becomes authoritative.

The CI `recovery_drill.py` materializes the exact commit in `SUPABASE_UPSTREAM`, resolves the database image from that materialized Compose definition, and rejects floating or non-Supabase PostgreSQL images. Both the source and restored instances use that same image, the pinned Supabase database initialization scripts, the production `pg_hba.conf`, `/etc/postgresql/postgresql.conf`, and the persistent `/etc/postgresql-custom` configuration volume. The evidence records the upstream commit, exact image, PostgreSQL version, HBA/config paths, and `supabase_admin` role state before and after recovery. The drill proves that a base backup taken before a transaction can replay archived WAL to a target after that transaction while excluding a later transaction.

## Logical database restore

Use `restore-drill` for routine validation. For disaster promotion:

1. create a new empty database on a replacement PostgreSQL instance;
2. restore globals only after reviewing every role and excluding passwords;
3. restore the custom-format dump with `--no-owner --no-acl`;
4. rerun the reviewed role configurator;
5. verify migration registry checksums, RLS, tenant boundaries, and durable queue constraints;
6. switch the application only after validation.

The Supabase dump contains protected schemas and functions that require the pinned image's existing `supabase_admin` superuser during replay. `restore-drill` verifies that exact role boundary and uses it only for `pg_restore --exit-on-error` into the temporary database. It does not elevate or grant privileges to the routine `postgres` role.

`backup-verify` and `restore-drill` accept only a canonical direct file under the configured backup root. The adjacent sidecar must name that same generation and its recorded SHA-256 must match before any decryption or temporary database creation begins. This prevents a caller-supplied archive from being paired with a same-named but different archive under the backup root.

Restore cleanup is registered on shell `EXIT`, not only normal function return. The temporary database, container dump, and decrypted plaintext payload are therefore removed for successful completion, command failure, and explicit error exits.

## Supabase Storage

Restore the Storage tree from the same backup generation into a new empty storage volume. Verify ownership and permissions against the pinned Storage image. Reconcile Storage metadata in PostgreSQL with physical files. Missing physical objects or unreferenced files are incident findings; do not silently discard them.

## Qdrant

The encrypted archive contains a collection snapshot when the collection existed. Restore it to a new collection or replacement Qdrant instance first, then verify:

- vector dimension and distance metric;
- collection profile name;
- school/tenant payload fields;
- exact authorized asset filtering;
- unpublished and archived assets are absent from retrieval;
- a known deterministic test point is readable.

If the snapshot does not correspond to the restored PostgreSQL point, rebuild the versioned collection from the restored authoritative database. Never mix OpenAI and local BGE vectors or rename the compatibility collection during recovery.

## Secret escrow

Installation environment files exist only inside the encrypted payload. Restore them to mode `0600` outside the immutable release. Rotate provider keys, dashboard credentials, database passwords, Supabase secret keys, Qdrant keys, and internal tokens if backup media exposure is suspected. Restoring a secret is not evidence that it remains uncompromised.

## Verification before reopening

Run:

```bash
edutalent-production database-check
edutalent-production gateway-check
edutalent-production qdrant-check
edutalent-production ai-check
edutalent-operations security-check
edutalent-operations monitor-once
```

Then execute the tenant/document/final security acceptance cases in `SECURITY_ACCEPTANCE.md`. Record actual RPO/RTO and compare them with the provisional 15-minute/2-hour targets.

## Scheduled drills

- every backup: decrypt, adjacent-sidecar SHA-256, and manifest verification;
- weekly: temporary logical database restore;
- monthly: isolated PostgreSQL PITR on the pinned Supabase database image and Qdrant snapshot recovery;
- quarterly: full replacement-host restore including Storage, secrets, TLS, load, and controlled failback;
- after every database, Supabase, Qdrant, storage, or release-format upgrade: full recovery drill before deployment.

## Coordinated backup boundary

`backup-create` stops the running application and Supabase writer-facing services before capturing PostgreSQL, Storage, and the Qdrant snapshot. A capability-free one-off application image performs only the Qdrant snapshot request. An `EXIT` cleanup handler is registered before quiescence and remains active until explicit successful cleanup. It resumes the exact stopped containers, deletes any server-side Qdrant snapshot, removes plaintext staging and partial/final uncommitted artifacts, and preserves the original failure status. This applies to normal completion, `set -e` termination, and explicit error exits.

This creates a documented write-quiesced cross-service recovery point instead of three independently timed copies without allowing a failed backup command to leave production partially stopped or plaintext material behind.
