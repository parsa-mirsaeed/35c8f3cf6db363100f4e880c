# Production patching, rotation, and rollback runbook

This runbook covers the host/application maintenance operations that are separate from ordinary application deployment. Perform every change against a named release with a recent verified encrypted backup, active WAL reception, an approved maintenance window, and an explicit rollback decision point. Never rotate several unrelated credentials or infrastructure layers in one uncontrolled step.

## General change rule

Before every maintenance action:

1. record the exact source/release SHA and immutable artifact digest;
2. verify `production-validate`, `production-database-check`, backup verification, WAL reception and current alerts;
3. create and verify a fresh encrypted backup when the change can affect persistent state;
4. preserve the previous configuration/credential in protected escrow only for the bounded rollback window;
5. make one change at a time;
6. rerun the relevant live security/health checks;
7. record evidence, owner, date and rollback disposition;
8. destroy superseded credentials only after rollback is no longer required and dependent services have demonstrably moved to the new value.

Do not use floating image/model tags, silently overwrite an immutable release directory, or edit applied database migrations.

## Host OS and Docker patching

- Review Ubuntu/Docker security advisories and the supported-host baseline before patching.
- Confirm the proposed kernel/Docker/Compose versions remain compatible with the supported baseline and pinned Compose features.
- Preserve a bootable previous kernel/package state according to the host policy.
- Verify recent backup/WAL evidence before reboot or daemon restart.
- Apply patches inside an approved maintenance window; this is a single-node deployment and interruption is expected.
- After reboot/daemon restart, rerun host preflight, production preflight, database/gateway/AI/Qdrant checks, WAL verification, monitoring and a bounded functional smoke/load check.
- If the host or Docker runtime no longer satisfies the baseline, roll back the host package/kernel change or move to a separately qualified baseline rather than lowering the check.

## TLS certificate rotation

1. Obtain the replacement certificate/key outside the release tree. The key must be mode `0600` and cover all three configured DNS names.
2. Keep the currently accepted certificate/key available for rollback without copying either key into Git or ordinary evidence artifacts.
3. Point `TLS_CERT_FILE` / `TLS_KEY_FILE` to the replacement pair in the protected operator configuration.
4. Run `production-validate` **before** restarting the gateway. It must prove hostname coverage, key/cert match and at least 14 days of validity.
5. Restart/recreate only the bounded gateway TLS staging/gateway path according to the installed release procedure, then run `production-gateway-check` and public HTTPS probes.
6. On any failure, restore the prior protected paths and rerun the same checks.
7. After the rollback window, securely retire the old private key according to the school's key-destruction policy.

## Application database credential rotation

The long-running application role is `NOBYPASSRLS`; rotation must not grant bootstrap authority.

1. Create a fresh URL-safe `DATABASE_APP_PASSWORD` in protected operator storage.
2. Using only the one-shot database administration/configuration path, update the existing application role password while preserving its exact role attributes and grants.
3. Update the protected application environment atomically; do not expose the password in shell history or logs.
4. Recreate the app/worker containers and run `production-database-check` plus tenant/RLS smoke evidence.
5. Verify the previous password no longer authenticates after the rollback decision point.
6. If rollback is required before that point, restore the prior password through the same one-shot admin path and restore the protected environment value.

Never place `POSTGRES_PASSWORD` in the application environment to make rotation easier.

## AI Gateway and provider credential rotation

Rotate `AI_GATEWAY_INTERNAL_TOKEN`, `OPENAI_API_KEY`, or `LLM_API_KEY` independently.

For the internal token:

1. generate a fresh high-entropy value;
2. update both app and AI Gateway protected environment values as one coordinated maintenance action;
3. recreate both services without changing provider destinations;
4. run `production-ai-check` and confirm invalid/old internal tokens are rejected;
5. roll back both sides together if required.

For provider credentials:

1. update only the AI Gateway secret value; the app/browser must never receive it;
2. recreate only the AI Gateway;
3. run provider-specific bounded probe plus `production-ai-check` and verify core application health is independent of provider availability;
4. revoke the old provider credential only after the new one is proven.

## Qdrant API-key rotation

1. create a verified Qdrant snapshot and confirm the latest encrypted system backup before changing access credentials;
2. generate a fresh `QDRANT_API_KEY` in protected configuration;
3. update Qdrant and the authorized application-side consumer in one maintenance window;
4. recreate the affected containers and run `production-qdrant-check` plus knowledge retrieval tenant-filter smoke;
5. verify the old key is rejected before destroying it;
6. roll back both values together if required.

Do not expose Qdrant publicly during rotation and do not temporarily disable API-key authentication.

## Supabase JWT/API key rotation

Supabase Auth/JWT/API-key rotation affects multiple coordinated services and user sessions. It is not an ordinary environment edit.

- Schedule it as a dedicated migration/change with a fresh backup and recovery point.
- Regenerate/replace keys using the pinned self-hosted Supabase procedure for the installed version.
- Update all dependent local Supabase services and EduTalent verification configuration atomically.
- Expect existing sessions/tokens to require invalidation/re-authentication unless a deliberately reviewed overlap strategy exists.
- Run Auth login/refresh/logout, inactive-account, public-signup denial, administrative-boundary and server-function smoke tests before reopening normal use.
- Retain the old key material only inside protected rollback escrow for the approved rollback window; never publish it as evidence.

## Backup passphrase rotation

Changing the passphrase does **not** transparently re-encrypt old archives.

1. escrow the new passphrase separately from both local and off-host backup media;
2. update only `EDUTALENT_BACKUP_PASSPHRASE_FILE` to the protected new file;
3. create, verify, off-host copy and restore-drill a new backup generation;
4. retain the old passphrase in protected escrow for all retained archives encrypted with it until those archives expire or are deliberately re-encrypted under an approved migration;
5. never delete old escrow while retained archives still require it.

## Qdrant version upgrade and rollback

- Pin the exact target image digest/version; no `latest` tag.
- Review the Qdrant vendor-supported upgrade path and snapshot compatibility.
- Create/verify a Qdrant snapshot and full encrypted system backup.
- Restore the snapshot into an isolated target-version drill where practical before changing production.
- Upgrade in the maintenance window, run private readiness, collection/vector-dimension checks, tenant-filter retrieval smoke and ingestion/retry checks.
- Roll back to the prior image only when its data/snapshot compatibility is proven; otherwise restore the pre-upgrade snapshot/rebuild vectors from authorized PostgreSQL source data.

## Embedding/model/profile change

A model/profile change is a data migration, not a string replacement.

- Use a new versioned embedding profile and **new Qdrant collection** when model or vector dimension changes.
- Preserve PostgreSQL publication/authorization as the source of truth.
- Backfill/reindex only authorized published assets through the durable ingestion path.
- Do not mix vectors from different model/dimension contracts in one collection.
- Validate retrieval quality/tenant isolation and complete bounded load evidence before switching the active profile.
- Rollback means switching the application/gateway back to the prior profile and collection; do not attempt to reinterpret incompatible vectors.
- Retire the old collection only after the rollback window and retention/approval process.

## Release/application rollback

Use `DEPLOYMENT_UPGRADE.md` for side-by-side immutable release upgrade/rollback and failed migration handling. A code rollback is permitted only if the prior binary remains compatible with the current database schema; otherwise use a forward fix or coordinated restore to the pre-upgrade backup/PITR generation.

## Acceptance evidence

Record each practiced rotation/upgrade in `TARGET_HOST_ACCEPTANCE.md` with:

- exact release/source SHA;
- component/credential rotated;
- old/new version identifiers or secret identifiers (never secret values);
- backup/recovery point;
- commands/checks executed;
- observed downtime and recovery behavior;
- rollback result or explicit decision not to roll back;
- operator/security reviewer/date;
- residual risk and next review date.
