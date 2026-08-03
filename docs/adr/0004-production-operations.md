# ADR 0004: Local production operations and tested recovery

## Status

Accepted for Plan V1 Production Operations implementation.

## Context

The previous production phases established the offline-first topology, controlled external AI, and a signed air-gapped appliance. Production readiness still requires verified backup and restore, local observability, alerting, security scanning, load/fault evidence, and operator runbooks. The deployment must retain its security boundary: only the reverse proxy is public, only the AI Gateway has external provider egress, and no service may receive a Docker socket merely to collect metrics.

## Decision

EduTalent uses a host-invoked local operations command packaged with the production definitions. It reuses the exact production Compose project, environment, profile, and immutable image lock. It adds no service, public port, remote telemetry dependency, or runtime internet requirement.

The command provides:

- encrypted full backups with PostgreSQL logical and physical data, Supabase Storage, Qdrant snapshots, WAL, release identity, and encrypted secret escrow;
- continuous PostgreSQL WAL reception through a private, capability-free, read-only helper container;
- immediate backup verification and non-destructive temporary restore drills;
- local JSON health/resource snapshots and deterministic alert policy;
- rendered and live topology security verification;
- bounded load/soak and controlled restart testing;
- final acceptance orchestration.

CI adds exact-head Trivy scans, configuration fault tests, full encrypted backup/restore against the production stack, PostgreSQL PITR, Qdrant recovery, migration rollback, database restart, application recreation, and sustained load proof.

## Consequences

The appliance remains usable without a monitoring cloud or third internet exception. Operations data stays local and can integrate with school-local systems. Backup media and passphrase escrow become explicit operator responsibilities. Host invocation requires access to the rootless Docker context, so the dedicated operations account is privileged relative to application data even though no container gains Docker-socket access.

A single appliance is still not highly available. Controlled app recreation and database restart can interrupt service. The provisional 15-minute RPO and 2-hour RTO are engineering targets that each school must validate. Restoring PostgreSQL, Storage, and Qdrant as a coherent generation remains a runbook-controlled process rather than an unsafe one-click overwrite.

## Rejected alternatives

- Mounting the Docker socket into Prometheus or a custom collector: excessive host-control privilege.
- Public or cloud monitoring by default: violates strict-network operation and adds an external data destination.
- Unencrypted tar backups: exposes school data and secrets at rest.
- Backing up only PostgreSQL: omits Storage, vectors, release identity, and recovery credentials.
- Treating green health checks as disaster-recovery evidence: does not prove restoration.
