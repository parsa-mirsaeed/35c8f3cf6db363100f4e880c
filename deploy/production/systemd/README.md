# EduTalent production systemd units

These units are reference templates for the supported single-node host baseline. They automate local monitoring, encrypted backups, verified off-host backup/WAL copy, restore verification, and continuous WAL reception without adding internet telemetry or granting the service account `sudo`.

## Installation assumptions

- install the release under `/opt/edutalent` as root, then keep the signed release tree read-only to the operator;
- create a dedicated unprivileged `edutalent-operator` account;
- configure that account's approved Docker context (rootless is preferred; a rootful daemon requires the tailored host/CIS review);
- for rootless Docker, enable the operator's user daemon/linger according to the host policy and set `DOCKER_HOST` to that operator's approved Unix socket; these system units intentionally do not depend on the rootful `docker.service`;
- create `/var/lib/edutalent/operations` mode `0700`, owned by `edutalent-operator`;
- mount the protected local backup filesystem at `/mnt/edutalent-backup`, owned by the operator and not on the same filesystem/device as production/data;
- mount the approved off-host backup target at `/mnt/edutalent-offhost`; this mount must represent the school's controlled off-appliance destination, not merely a second directory on the same local filesystem;
- create `/etc/edutalent/operations.env` mode `0600`, root-owned, with at least:

```dotenv
EDUTALENT_OPERATIONS_STATE_DIR=/var/lib/edutalent/operations
EDUTALENT_BACKUP_DIR=/mnt/edutalent-backup
EDUTALENT_OFFHOST_BACKUP_DIR=/mnt/edutalent-offhost
EDUTALENT_OFFHOST_WAL_DIR=/mnt/edutalent-offhost/wal
EDUTALENT_BACKUP_PASSPHRASE_FILE=/etc/edutalent/backup.passphrase
EDUTALENT_APP_ENV=/opt/edutalent/deploy/production/.env.edutalent
EDUTALENT_SUPABASE_ENV=/opt/edutalent/deploy/production/runtime/supabase/.env
# Rootless example; use the actual numeric UID and approved socket:
# DOCKER_HOST=unix:///run/user/1001/docker.sock
```

The passphrase file must be mode `0400` or `0600` and its escrow must remain separate from both local and off-host backup media. The environment file must not contain a copy of the passphrase itself. When rootless Docker is used, verify the socket belongs to the dedicated operator and is not shared with unrelated users.

Copy the units into `/etc/systemd/system/`, review paths and the Docker context for the actual host, then run:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now edutalent-wal.service
sudo systemctl enable --now edutalent-wal-verify.timer
sudo systemctl enable --now edutalent-offhost-wal.timer
sudo systemctl enable --now edutalent-monitor.timer
sudo systemctl enable --now edutalent-backup.timer
sudo systemctl enable --now edutalent-offhost-copy.timer
sudo systemctl enable --now edutalent-restore-verify.timer
```

`edutalent-backup` performs an encrypted backup and immediate cryptographic/manifest verification. `edutalent-offhost-copy` selects the newest verified encrypted archive, verifies its source SHA-256, copies only the encrypted archive plus metadata to the mounted off-host destination using an atomic partial file, and verifies the copied SHA-256 before publication. It never reads or copies the backup passphrase. `edutalent-wal-verify` exercises the running receiver and forces a WAL boundary periodically; `edutalent-offhost-wal` copies only completed 24-hex WAL segment files to the off-host mount, SHA-256 verifies new and existing segments, and fails closed on any mismatch. `edutalent-restore-verify` separately restores the newest verified local archive into the temporary drill database so checksum verification is not confused with restoration proof.

Before enabling the units, run the live host preflight with operations checks and the pre-start DNS/port/time check, retaining both JSON outputs:

```bash
python3 /opt/edutalent/deploy/production/host_preflight.py \
  --require-operations \
  --output /var/lib/edutalent/operations/host-preflight.json
python3 /opt/edutalent/deploy/production/host_network_preflight.py \
  --app-env /opt/edutalent/deploy/production/.env.edutalent \
  --output /var/lib/edutalent/operations/host-network-preflight.json
```

An automatic PASS does not complete target-host acceptance. Encryption, firewall/daemon tailoring, proof that `/mnt/edutalent-offhost` is genuinely off-appliance, passphrase escrow, measured RPO/RTO/load, and replacement-host recovery must be recorded in `../operations/TARGET_HOST_ACCEPTANCE.md`.
