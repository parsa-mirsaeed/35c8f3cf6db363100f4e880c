# Local monitoring and alerting runbook

## Design

Monitoring is host-initiated and local. No production service mounts the Docker socket. No metrics UI, alert receiver, or collector is exposed publicly. The operator command reads only bounded service state and health data and writes mode-`0600` JSON snapshots outside the signed release.

## Signals

Core critical signals:

- required core service missing, exited, or unhealthy;
- application `/healthz` unavailable;
- disk below the approved reserve;
- TLS certificate below 14 days;
- encrypted backup older than 25 hours or unverifiable;
- WAL archive older than 20 minutes or receiver stopped;
- database connection usage at or above 85 percent;
- backup preflight capacity failure.

Degraded warnings:

- Qdrant unavailable while core school functions remain healthy;
- AI Gateway unavailable while core school functions remain healthy;
- high but non-critical resource pressure;
- backup duration or load-test p95 regression.

External AI availability must never be a core health criterion.

## Alert delivery

The command exits `2` when a critical alert exists and `0` when there is no critical alert. A school may connect this to:

- a local systemd `OnFailure` unit;
- a school-local SMTP relay;
- an on-premises network-management system;
- a restricted administrative dashboard reading the JSON file;
- a physical operations console.

Any integration must stay inside the approved management network unless the security policy formally approves another destination. Alert payloads must contain codes and bounded metadata, not secrets, document content, prompts, student data, or raw logs.

## Triage order

1. Confirm the alert snapshot timestamp and release identity.
2. Check disk and WAL first; a full disk can damage several services.
3. Check PostgreSQL and Storage before restarting the application.
4. Preserve logs and state before destructive action.
5. Distinguish AI/Qdrant degradation from core outage.
6. Follow the disaster-recovery runbook when integrity is uncertain.
7. Record resolution, actual impact, RPO/RTO, and any threshold change.

## Threshold changes

Thresholds live in `alert-policy.json`. Changes require review and regression tests. Lowering sensitivity to hide a recurring incident is not remediation. Capacity thresholds must account for database growth, WAL retention, Qdrant snapshots, Storage, and at least one complete backup staging cycle.

## Fail-closed integrity signals

Monitoring verifies the newest backup sidecar against the referenced encrypted
archive and its SHA-256 digest. A missing, truncated, replaced, or path-unsafe
archive is critical even when the sidecar timestamp is recent. Standalone
`backup-verify` and `restore-drill` also require that adjacent sidecar to match
before decrypting or creating a drill database.

The TLS lifetime is derived from the prepared certificate mounted at
`/etc/caddy/tls/fullchain.pem` in the running gateway, not from the host renewal
source. Replacing `TLS_CERT_FILE` without restarting/reloading the gateway cannot
therefore make monitoring report a certificate that Caddy has not loaded. A
missing, unreadable, or invalid prepared certificate is an unknown critical
state.

The backup disk has an independent free-space threshold, and the WAL receiver
must be running in addition to having a recent completed segment.
