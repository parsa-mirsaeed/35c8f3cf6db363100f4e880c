# PR-13 target-host hardening revalidation note

```text
Repository: parsa-mirsaeed/35c8f3cf6db363100f4e880c
Base branch: main
Base SHA: 52365ddfbb3196ee46261cbc771bc957c4467882
Feature branch: agent/pr-13-target-host-hardening
Current head SHA: recorded in PR #17 exact-head evidence (a commit cannot truthfully embed its own resulting SHA)
PR number: 17
Relevant plan PR: PR-13
Finding still reproducible: yes — operations/recovery machinery existed, but supported-host qualification, installable maintenance units, tailored CIS/container evidence, off-host copy automation, and a structured replacement-host acceptance record were incomplete
Affected files: deploy/production host/operations/systemd evidence and regression tests; production operator documentation; this revalidation note
Required targeted workflow: AI Change Proof operations/topology classification; Production Operations definition/regression/security-scan tier; Production Foundation rendered topology definition proof
Heavy workflows intentionally deferred: complete Production Foundation + Production Operations recovery/acceptance and final Full Validation until the ordinary targeted head was green; Package/Air-gapped Appliance were not manually requested because runtime/package/appliance inventory was not changed
```

## Reconfirmed inherited controls

The merged baseline already contains substantial PR-13 prerequisites:

- production topology rendering and fail-closed security validation;
- a constrained `NOBYPASSRLS` application database role with live proof;
- encrypted full backup creation/verification and temporary logical restore;
- continuous physical WAL reception and isolated PITR drills;
- Qdrant snapshot recovery/reindex decision support;
- monitoring/alert snapshots without cloud telemetry;
- bounded load and single-node fault/restart tests;
- near-expiry TLS and unsafe CIDR fail-closed checks;
- deployment/upgrade, disaster-recovery and monitoring runbooks;
- Production Foundation and Production Operations workflows with expensive recovery/acceptance tiers gated to final review.

PR-13 does not duplicate those controls. It closes the remaining host-specific qualification gaps around them.

## Added PR-13 controls

- machine-readable Ubuntu 24.04/x86_64 single-node supported-host baseline;
- deterministic live or fixture-fed host preflight for OS/kernel/CPU/RAM/storage/inodes/filesystem/Docker/Compose/operator/time plus operations backup-device/passphrase checks;
- pre-start DNS/public-port/clock-skew host check, with unobservable skew retained as manual evidence instead of false PASS;
- explicit manual evidence state for encryption, firewall visibility, rootful-Docker review, off-host media identity and passphrase escrow rather than false automatic PASS;
- deterministic per-container hardening inventory with final-release SHA-256 digest enforcement mode;
- installable hardened systemd monitor, backup, off-host encrypted-backup copy, restore-verification, WAL reception/verification and off-host WAL synchronization units/timers;
- SHA-256/tamper-checked off-host encrypted backup and completed-WAL copy helpers that never copy passphrases/replication credentials;
- tailored CIS Docker Benchmark v1.8.0 evidence map without certification claims;
- clean replacement-host qualification record covering installation, restore, PITR, Qdrant, disk/TLS/config/migration/outage faults, measured RPO/RTO, school-scale load/soak, maintenance/rotation rehearsal and residual risks;
- maintenance runbook covering host/Docker patching, TLS, database/AI/Qdrant/Supabase credentials, backup-passphrase generations, Qdrant upgrades and embedding/model profile migration/rollback;
- corrected production operator guidance for current `NOBYPASSRLS` transaction-scoped authorization and air-gapped/AI architecture;
- explicit single-node/non-HA language.

## Final-review workflow sequencing

Ordinary exact-head validation is required before enabling the expensive final tier. After the targeted head became green, PR #17 received the repository `full-validation` label.

`Production Operations` subscribes to the `labeled` pull-request event and therefore enters its recovery/production-acceptance tier immediately. `Production Foundation` has label-gated heavy jobs but does not subscribe to a labeled event; this final evidence-note synchronize occurs **after** the label is present so the event payload carries `full-validation` and the migration-role plus complete production-stack jobs execute on the resulting exact head. This is intentional cost-aware orchestration, not reuse of an older definition-only run.

Package/Air-gapped workflows may also start under the repository's broad `deploy/production/**` path policy. They are incidental for PR-13 because no runtime image or appliance inventory was changed, and are not substituted for Production Foundation/Operations evidence.

## Manual/controlled evidence deliberately not simulated in CI

PR-13's plan exit gate still requires a named frozen release to be installed and restored on a clean supported host. The target-host record must contain measured RPO/RTO/load results, encryption/firewall/daemon/CIS evidence, proof the backup/WAL destination is genuinely off-appliance, passphrase-escrow evidence, maintenance rehearsal, and human operator/security residual-risk sign-off.

Until that controlled qualification exists, the engineering PR may be `ready for final validation` or `ready to merge` as implementation work, but the product must not be classified as target-host accepted or contract-ready solely from CI.
