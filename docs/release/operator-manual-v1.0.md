# EduTalent operator manual v1.0

Applies to the first production release family. The deployment record must also state the exact release tag, commit SHA, image/appliance digests and target-host qualification record.

## Supported operating modes

### Connected AI

Core application and authentication remain self-hosted. Authorized AI requests go only through the local AI Gateway. The gateway is the only component with approved AI egress and holds provider credentials. Current connected profiles use fixed provider origins/models defined by code/configuration; browser clients do not choose provider destinations.

### Degraded AI

If the AI Gateway or provider is unavailable, core login and school operations remain available. AI-backed calls return controlled unavailable behavior and durable embedding/personalization work retries with bounded backoff. Operators investigate the gateway/provider path without treating AI outage as a reason to expose internal services.

### Fully offline/local AI

Use the air-gapped appliance/local profile. Required images/model artifacts are prepared and verified before target-host startup; first offline startup must not pull images/models from registries or providers.

## Supported host

Follow `../../deploy/production/HOST_BASELINE.md` and the machine-readable `../../deploy/production/host-baseline.json`. The first qualification target is Ubuntu Server 24.04 LTS on x86_64 with the documented kernel/Docker/Compose/resource/storage/network/time floors. It is single-node, not HA.

## Installation and validation

1. Verify the named release, checksum/signature/provenance as applicable.
2. Install under the approved read-only release path and use the dedicated unprivileged operator account.
3. Configure three DNS names, restricted admin CIDRs and operator-supplied TLS.
4. Run host and network/time preflight before startup.
5. Materialize the pinned Supabase runtime/prepared appliance artifacts.
6. Generate production secrets on the target environment; never commit them.
7. Run `production-validate`, then start production.
8. Run database, gateway, AI and Qdrant checks.
9. Retain the container-hardening inventory and target-host evidence.

Primary procedure: `../../deploy/production/README.md`.

## School onboarding

- Create/approve the school through the PlatformAdmin-supported flow.
- Provision authorized SchoolManager accounts through the supported administrative process; public signup remains disabled.
- SchoolManager provisions/maintains only currently supported user/class/enrollment workflows.
- Confirm the contracted feature matrix before enabling training or customer acceptance.
- Configure AI profile only when the contract and privacy review allow it.

## Backup, restore and continuity

Use `../../deploy/production/operations/README.md` and systemd units under `../../deploy/production/systemd/`.

- encrypted full backup on the accepted schedule;
- continuous/periodic WAL handling consistent with accepted RPO;
- verified off-appliance copy and separate passphrase escrow;
- weekly/accepted restore verification;
- PostgreSQL PITR and Qdrant recovery/reindex procedure;
- retain evidence and alert on stale/failed operations.

Contractual RPO/RTO are the values measured and accepted for the school, not CI defaults.

## Monitoring and alerts

Use the production monitoring snapshot/alert commands and installed timers. Investigate critical database/backup/disk/configuration states immediately. AI/Qdrant degradation can be operationally significant while core school service may remain healthy; escalation severity follows `security-organization.md` and the signed service schedule.

## Patch, upgrade and rollback

Use `../../deploy/production/operations/MAINTENANCE_ROTATION.md`.

Before change: backup, record release/config, confirm rollback material. After change: rerun host/preflight and production checks. Model/dimension changes use a new embedding profile/collection and complete re-index; do not mix vector spaces.

## Key and certificate rotation

Rotate TLS, application DB, AI Gateway/provider, Qdrant, Supabase and backup-passphrase generations using the documented bounded procedures. Preserve rollback material securely, never place secret values in ordinary evidence artifacts, and verify the affected boundary after rotation.

## Incident response

Follow `security-organization.md` and the production threat/runbook material. Preserve timestamps/logs/evidence, limit privileges, avoid destructive cleanup before evidence capture, rotate affected credentials, validate tenant boundaries, restore from verified data where required, and execute customer/legal notification according to the approved privacy/contract procedure.

## Offboarding and secure deletion

1. Freeze the school offboarding request and authorized scope.
2. Produce agreed export/return data through an approved operator process.
3. Confirm contractual retention/hold requirements.
4. Revoke user/operator access and provider credentials scoped to the departing environment.
5. Delete school data and derived/vector copies according to the approved deletion schedule.
6. Handle backups/WAL according to retention and cryptographic-erasure policy; do not promise instantaneous erasure from retained recovery media unless contractually designed and proven.
7. Record completion evidence and exceptions.

Human target-host/operator acceptance is recorded separately in PR #16; this manual does not self-approve an installation.
