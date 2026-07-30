# EduTalent production operations

This directory is the local operations layer for the offline-first school appliance. It deliberately adds no public port, remote telemetry dependency, cloud control plane, or Docker socket mount. Operators invoke `../edutalent-operations` on the host; the command talks to the existing production Compose project with the same environment, profile, network, and immutable-image override used by the appliance launcher.

## Operational targets

The initial engineering objectives are:

- provisional recovery point objective (RPO): 15 minutes;
- provisional recovery time objective (RTO): 2 hours;
- continuous local WAL reception;
- one encrypted full backup at least every 24 hours;
- restore verification after every backup and a complete recovery drill on a scheduled basis;
- local monitoring at least once per minute;
- no monitoring or backup credential in the application, browser, image, source tree, or immutable release.

These are acceptance targets, not contractual guarantees. A single server is not highly available. Schools must set final retention, RPO, RTO, capacity, and off-host copy requirements according to their infrastructure and legal obligations.

## Required host configuration

Create a dedicated unprivileged operating-system account, for example `edutalent-operator`, with access only to the rootless Docker context and the required operations directories. Do not run the timer as a personal account and do not give it `sudo`.

Set an operations state directory and a backup destination on a physically separate encrypted disk or controlled backup host mount:

```bash
export EDUTALENT_OPERATIONS_STATE_DIR=/var/lib/edutalent/operations
export EDUTALENT_BACKUP_DIR=/mnt/edutalent-backup
export EDUTALENT_BACKUP_PASSPHRASE_FILE=/etc/edutalent/backup.passphrase
```

The passphrase file must be mode `0400` or `0600`. It must be escrowed separately from the encrypted backup media. Losing the passphrase makes the archives unrecoverable; storing it beside the archive defeats the encryption boundary.

For an installed appliance, the command automatically uses the external appliance state directory and immutable Compose lock. It never writes monitoring or backup state into the signed release tree.

## Backups

Run capacity and credential checks first:

```bash
./deploy/production/edutalent-operations backup-preflight
```

Create a backup:

```bash
./deploy/production/edutalent-operations backup-create
```

The command captures:

- PostgreSQL custom-format logical dump;
- PostgreSQL globals without role passwords;
- PostgreSQL physical base backup;
- Supabase Storage data;
- a Qdrant collection snapshot, or an explicit not-yet-created marker;
- archived WAL already received locally;
- configuration templates and the release identity;
- the real installation environment as encrypted secret escrow inside the encrypted payload;
- a file-mode, size, and SHA-256 manifest.

The payload is streamed into an AES-256-CBC archive using PBKDF2-SHA256 with 600,000 iterations. The plaintext staging directory is mode `0700`, removed on every exit, and never retained as a backup. Creation is atomic: a `.partial` file is renamed only after encryption and immediate decrypt/manifest verification succeed.

Verify without restoring:

```bash
./deploy/production/edutalent-operations backup-verify /mnt/edutalent-backup/edutalent-backup-....tar.gz.enc
```

Restore into a temporary drill database without modifying the source database:

```bash
./deploy/production/edutalent-operations restore-drill /mnt/edutalent-backup/edutalent-backup-....tar.gz.enc
```

Apply retention only after successful off-host copy and verification:

```bash
EDUTALENT_BACKUP_RETENTION_DAYS=30 \
EDUTALENT_BACKUP_MINIMUM_COPIES=7 \
./deploy/production/edutalent-operations prune
```

The prune command never deletes the newest minimum number of archives.

## Continuous WAL reception

Start the local WAL receiver:

```bash
./deploy/production/edutalent-operations pitr-start
./deploy/production/edutalent-operations pitr-status
```

The receiver is a dedicated, capability-free, read-only container on the private `edutalent-data` network. It uses a physical replication slot and writes only to the external operations WAL directory. No service gains internet egress or a Docker socket.

Stop it without deleting archived WAL:

```bash
./deploy/production/edutalent-operations pitr-stop
```

A stale or stopped WAL receiver is a critical local alert. Physical WAL must be copied off the appliance at least as frequently as the approved RPO. Retaining a replication slot while the receiver is permanently stopped can fill PostgreSQL storage; resolve the incident rather than ignoring the alert.

## Local monitoring and alerting

Collect a snapshot:

```bash
./deploy/production/edutalent-operations snapshot
```

Evaluate the policy:

```bash
./deploy/production/edutalent-operations alerts
```

Or run both:

```bash
./deploy/production/edutalent-operations monitor-once
```

Snapshots include Compose service state, container resource counters, core health, Qdrant and AI Gateway availability, database size and connection pressure, disk capacity, TLS lifetime, latest encrypted backup, and latest WAL receipt. Files are JSON with mode `0600` under the external operations state directory. No document content, API key, JWT, password, model prompt, vector payload, student identifier, or raw application log is collected.

`alert-policy.json` defines conservative defaults. Critical alert exit status is `2`; this lets a local systemd timer, school-local monitoring system, or restricted administrative script page an operator without internet telemetry. Warnings intentionally distinguish AI/Qdrant degradation from core application failure.

Example systemd timer design:

```ini
# /etc/systemd/system/edutalent-monitor.service
[Unit]
Description=EduTalent local production monitor
After=docker.service

[Service]
Type=oneshot
User=edutalent-operator
EnvironmentFile=/etc/edutalent/operations.env
ExecStart=/opt/edutalent/deploy/production/edutalent-operations monitor-once
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/edutalent/operations /mnt/edutalent-backup
```

```ini
# /etc/systemd/system/edutalent-monitor.timer
[Unit]
Description=Run EduTalent local production monitor every minute

[Timer]
OnBootSec=2min
OnUnitActiveSec=1min
AccuracySec=10s
Persistent=true

[Install]
WantedBy=timers.target
```

The exact paths and rootless Docker environment must be reviewed for the host. Do not copy a user's personal Docker credentials into the service account.

## Security scanning

The `Production Operations` workflow performs:

- Trivy filesystem vulnerability scanning for high and critical dependency findings;
- Trivy configuration scanning for high and critical infrastructure findings;
- the repository's existing all-history secret scan and signed-release checks through the air-gapped release gate;
- deterministic rendered-Compose security verification;
- final live checks for privileged mode, host networking, Docker socket mounts, public ports, and AI egress membership.

Run the local rendered/live boundary check:

```bash
./deploy/production/edutalent-operations security-check
```

A scan finding must be fixed, explicitly risk-accepted with expiry and owner, or shown to be a false positive with narrow evidence. Do not weaken the scanner or hide broad directories to make the workflow green.

## Load and fault testing

Run a bounded load test:

```bash
EDUTALENT_LOAD_DURATION_SECONDS=300 \
EDUTALENT_LOAD_CONCURRENCY=32 \
EDUTALENT_LOAD_MAXIMUM_ERROR_RATE=0.01 \
EDUTALENT_LOAD_MAXIMUM_P95_MS=1000 \
./deploy/production/edutalent-operations load-test https://app.school.example/healthz
```

Run controlled single-node restart checks:

```bash
./deploy/production/edutalent-operations fault-test
```

The fault test restarts PostgreSQL, verifies database connectivity recovers, recreates the application container without touching data volumes, and verifies core health. A single-node controlled recreation can cause a brief interruption; it is not a high-availability rolling deployment.

## Final acceptance

With the production stack running and backup configuration present:

```bash
./deploy/production/edutalent-operations acceptance https://app.school.example/healthz
```

This runs topology and live security checks, constrained database identity, gateway/authentication boundaries, Qdrant, AI outage/recovery, local alerts, load testing, controlled restart recovery, encrypted backup verification, and a temporary-database restore drill. The CI recovery drill separately proves physical PostgreSQL point-in-time recovery, Qdrant snapshot recovery, and failed-migration rollback.

Read `DISASTER_RECOVERY.md`, `DEPLOYMENT_UPGRADE.md`, `MONITORING_ALERTING.md`, and `SECURITY_ACCEPTANCE.md` before production use.
