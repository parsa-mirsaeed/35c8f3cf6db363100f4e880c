# EduTalent production systemd units

These units are reference templates for the supported single-node host baseline. They automate local monitoring, encrypted backups, restore verification, and continuous WAL reception without adding internet telemetry or granting the service account `sudo`.

## Installation assumptions

- install the release under `/opt/edutalent` as root, then keep the signed release tree read-only to the operator;
- create a dedicated unprivileged `edutalent-operator` account;
- configure that account's approved Docker context (rootless is preferred; a rootful daemon requires the tailored host/CIS review);
- create `/var/lib/edutalent/operations` mode `0700`, owned by `edutalent-operator`;
- mount the protected backup filesystem at `/mnt/edutalent-backup`, owned by the operator and not on the same filesystem/device as the production installation/data;
- create `/etc/edutalent/operations.env` mode `0600`, root-owned, with at least:

```dotenv
EDUTALENT_OPERATIONS_STATE_DIR=/var/lib/edutalent/operations
EDUTALENT_BACKUP_DIR=/mnt/edutalent-backup
EDUTALENT_BACKUP_PASSPHRASE_FILE=/etc/edutalent/backup.passphrase
EDUTALENT_APP_ENV=/opt/edutalent/deploy/production/.env.edutalent
EDUTALENT_SUPABASE_ENV=/opt/edutalent/deploy/production/runtime/supabase/.env
```

The passphrase file must be mode `0400` or `0600` and its escrow must remain separate from backup media. The environment file must not contain a copy of the passphrase itself.

Copy the units into `/etc/systemd/system/`, review paths and the Docker context for the actual host, then run:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now edutalent-wal.service
sudo systemctl enable --now edutalent-wal-verify.timer
sudo systemctl enable --now edutalent-monitor.timer
sudo systemctl enable --now edutalent-backup.timer
sudo systemctl enable --now edutalent-restore-verify.timer
```

`edutalent-backup` performs an encrypted backup and immediate cryptographic/manifest verification. `edutalent-restore-verify` separately restores the newest verified archive into the temporary drill database so backup verification is not confused with restoration proof. `edutalent-wal-verify` exercises the running receiver and forces a WAL boundary periodically.

Before enabling the units, run the live host preflight with operations checks and retain its JSON output:

```bash
python3 /opt/edutalent/deploy/production/host_preflight.py \
  --require-operations \
  --output /var/lib/edutalent/operations/host-preflight.json
```

An automatic PASS does not complete target-host acceptance. Encryption, firewall/daemon tailoring, off-host copy, measured RPO/RTO/load, and replacement-host recovery must be recorded in `../operations/TARGET_HOST_ACCEPTANCE.md`.
