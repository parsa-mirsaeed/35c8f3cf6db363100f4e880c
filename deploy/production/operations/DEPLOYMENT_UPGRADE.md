# Deployment, upgrade, and rollback runbook

## Initial deployment

1. Verify the signed appliance, checksums, provenance, SBOMs, and exact platform.
2. Install into a new empty immutable release directory outside the external appliance state directory.
3. Load manifest-owned images with registry access disabled.
4. Create or review the external state directory, TLS paths, administration CIDRs, backup disk, passphrase escrow, and operations account.
5. Initialize secrets once. Never copy example or previous-school credentials.
6. Run production preflight and operations security checks.
7. Start the stack, then run database, gateway, Qdrant, AI outage/recovery, monitoring, backup, and restore checks.
8. Enable continuous WAL reception and local monitoring only after the first verified full backup.
9. Record the release digest, configuration owner, backup generation, and acceptance evidence.

## Upgrade prerequisites

An upgrade is allowed only when:

- the new release is signed and verified independently;
- release notes and migrations are reviewed;
- no generated environment or mutable data is inside the release;
- at least one recent encrypted full backup and WAL stream are verified;
- a restore drill using that backup has passed;
- sufficient free space exists for the old release, new release, backup, database growth, WAL, and rollback;
- the current exact release and database migration checksums are recorded;
- a maintenance window and rollback decision point are approved.

## Side-by-side release procedure

1. Stop scheduled prune jobs, not backups or WAL reception.
2. Install the new release beside the old immutable release. Do not overwrite either directory.
3. Verify the new release before it can read external state.
4. Run definition and preflight checks against a copied, redacted configuration.
5. Create and verify a fresh full backup immediately before migrations.
6. Stop application traffic at the gateway or enter approved maintenance mode.
7. Run the new release migration command once. The migration registry must reject changed historical migrations.
8. Run the database-role configurator and database security checks.
9. Start the new application and gateway against the preserved external volumes.
10. Run security, health, tenant, document, AI outage, backup, restore, and bounded load acceptance.
11. Reopen traffic only after all required checks pass.
12. Retain the old release until the rollback window expires; it contains no mutable data or secrets.

## Application-only controlled recreation

`edutalent-operations fault-test` exercises the single-node app recreation path. This is a controlled restart, not zero-downtime high availability. Schedule it within a maintenance window unless a load balancer and multiple independently validated replicas are deployed.

## Database and Supabase upgrades

Never upgrade PostgreSQL or the pinned Supabase runtime as an incidental image-tag change. Required evidence includes:

- vendor upgrade path and extension compatibility;
- exact image digests;
- logical restore into the target version;
- physical recovery strategy for the source version;
- migration replay and checksum integrity;
- Auth, Storage, Realtime, PostgREST, Supavisor, and Studio compatibility;
- application-role privilege verification;
- full backup and recovery drill.

A physical PostgreSQL base backup is version-specific. Keep a logical dump for cross-major recovery.

## Failed migration rollback

Migrations are transactionally applied where PostgreSQL permits. If a migration fails:

1. keep traffic closed;
2. capture the exact error and database state;
3. verify no partial object remains;
4. do not edit an already-applied migration;
5. fix forward with a new migration when the transaction rolled back cleanly;
6. restore the pre-upgrade backup/PITR target when a non-transactional operation or external side effect occurred;
7. rerun full validation on a new exact commit.

The CI recovery drill deliberately triggers a transactional migration error and proves the table is absent afterward.

## Rollback

Code rollback is possible only when the previous binary remains compatible with the current schema. Before migration, define one of:

- backward-compatible migration and old-release restart;
- forward fix;
- full database/Storage/Qdrant recovery to the pre-upgrade backup generation.

Do not point the old application at a schema it cannot understand. Do not restore PostgreSQL without restoring matching Storage and reconciling Qdrant.

## Post-upgrade observation

For at least one business cycle, monitor:

- service restarts and health;
- database size, connections, WAL growth, and replication slot lag;
- backup creation and verification;
- Storage and Qdrant growth;
- authentication failures and administrative access;
- AI provider errors without treating them as core outage;
- p95 latency and error rate against the approved load baseline.
