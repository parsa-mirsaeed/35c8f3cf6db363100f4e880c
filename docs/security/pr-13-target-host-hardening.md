# PR-13 target-host hardening revalidation note

```text
Repository: parsa-mirsaeed/35c8f3cf6db363100f4e880c
Base branch: main
Base SHA: 52365ddfbb3196ee46261cbc771bc957c4467882
Feature branch: agent/pr-13-target-host-hardening
Current head SHA: recorded in the PR after the staged implementation is squashed
PR number: recorded after opening the draft PR
Relevant plan PR: PR-13
Finding still reproducible: yes — operations/recovery machinery exists, but supported-host qualification, installable maintenance units, tailored CIS/container evidence, and a structured replacement-host acceptance record were incomplete
Affected files: deploy/production host/operations/systemd evidence and regression tests; production operator documentation
Required targeted workflow: AI Change Proof operations/topology classification; Production Operations definition/regression/security-scan tier; Production Foundation rendered topology definition proof
Heavy workflows intentionally deferred: Full Validation plus complete Production Foundation and Production Operations recovery/acceptance until the stable final PR head; Package/Air-gapped Appliance unless runtime/package/appliance inventory changes
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
- explicit manual evidence state for encryption, firewall visibility, rootful-Docker review, off-host copy and passphrase escrow rather than false automatic PASS;
- deterministic per-container hardening inventory with final-release SHA-256 digest enforcement mode;
- installable hardened systemd monitor, backup, restore-verification and WAL units/timers;
- tailored CIS Docker Benchmark v1.8.0 evidence map without certification claims;
- clean replacement-host qualification record covering installation, restore, PITR, Qdrant, disk/TLS/config/migration/outage faults, measured RPO/RTO, school-scale load/soak, upgrades/key rotation and residual risks;
- explicit single-node/non-HA language.

## Manual/controlled evidence deliberately not simulated in CI

PR-13's plan exit gate still requires a named frozen release to be installed and restored on a clean supported host. The target-host record must contain measured RPO/RTO/load results, encryption/firewall/daemon/CIS evidence, off-host backup/passphrase-escrow evidence, and human/operator residual-risk sign-off.

Until that controlled qualification exists, the engineering PR may be `ready for final validation` or `ready to merge` as implementation work, but the product must not be classified as target-host accepted or contract-ready solely from CI.
