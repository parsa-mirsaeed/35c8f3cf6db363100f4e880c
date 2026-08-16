# EduTalent supported target-host baseline

This document defines the first production qualification baseline implemented by PR-13. The machine-readable source is `host-baseline.json`; if the two disagree, treat that as a release blocker and correct both before qualification.

## Supported first-release host

- Ubuntu Server **24.04 LTS**;
- `x86_64` architecture for the first target-host qualification baseline;
- Linux kernel **6.8.0 or newer** within the supported Ubuntu 24.04 line;
- root filesystem `ext4` or `xfs` on SSD storage;
- Docker Engine **24.0.0 or newer**;
- Docker Compose **2.24.4 or newer**;
- minimum **4 CPU cores**;
- minimum **8 GiB RAM**;
- minimum **20 GiB free** on the application/system filesystem at host preflight;
- minimum **100,000 free inodes** at host preflight;
- a physically/logically separate protected backup filesystem with at least the operations preflight minimum free capacity;
- synchronized system time, with maximum accepted measured skew of **60 seconds**;
- three distinct resolvable DNS names for app, Supabase API and restricted administration;
- host TCP ports **80 and 443** available before EduTalent startup and reserved for the gateway;
- operator-supplied TLS covering all three names with at least 14 days of remaining validity at preflight.

The 20 GiB free-space threshold is an installation/operations floor, **not school capacity sizing**. Document, database, model, Qdrant, WAL and backup-retention sizing must be measured against the expected school workload and recorded in `operations/TARGET_HOST_ACCEPTANCE.md`.

The appliance may contain artifacts for other architectures, including arm64, but that does not make an architecture a qualified production host. Each supported target-host architecture requires its own installation, restore, load and operational acceptance evidence.

## Storage and encryption

All production data-bearing filesystems must use encryption at rest appropriate to the school's host/storage environment. The repository cannot prove block-device or hypervisor encryption from CI, so target-host evidence is mandatory.

Backups must be written to a separate protected filesystem/device or controlled backup-host mount. The backup passphrase must be mode `0400` or `0600`, must never be stored in the signed release or backup archive, and must be escrowed separately from encrypted backup media. WAL and encrypted backups must be copied off the appliance at frequencies consistent with the accepted measured RPO. The checked-in systemd helpers SHA-256 verify encrypted archive copies and completed WAL segments, but target acceptance must prove that the mounted destination is genuinely off-appliance.

## Docker and operator model

Use a dedicated unprivileged operating-system account such as `edutalent-operator`; do not run routine timers under a personal administrator account and do not grant the timer account `sudo`.

Rootless Docker is preferred because it narrows the host daemon boundary. A rootful Docker daemon is not silently classified as a failure because some supported infrastructure may require it, but it is acceptable only after the target-host CIS Docker/host review explicitly covers daemon/socket access, firewalling, logging/auditing, and operator membership. An unauthenticated remote Docker API is not supported.

The current official CIS catalogue lists **CIS Docker Benchmark v1.8.0**. The Ubuntu host assessment should use the current applicable Ubuntu 24.04 benchmark at qualification time. Repository evidence is a tailored control map, not a certification claim.

## Networking

Only the EduTalent gateway may publish host TCP ports 80 and 443. PostgreSQL, Supabase internal services, Qdrant and administration internals publish no host ports. The public/admin DNS and firewall configuration must preserve the repository topology: administration is restricted by approved CIDRs and authentication, and only the AI Gateway may receive approved external AI egress.

Run `host_network_preflight.py` **before `production-up`**. It proves that all three configured DNS names resolve, the required port contract remains exactly 80/443, and those ports are not already occupied by an unrelated process before the gateway starts. The host firewall ruleset and upstream network policy remain target-host evidence; if the general host preflight cannot inspect the firewall due permissions/tooling, it reports a manual item rather than claiming PASS.

## Time

A synchronized clock is required for TLS, token/session behavior, backup evidence and incident timelines. The live preflight fails when it can prove NTP is unsynchronized. The network/time preflight also reads quantified `chronyc tracking` skew when available and fails if absolute skew exceeds 60 seconds. If synchronization/skew cannot be observed automatically, qualification must record the approved time source and measured/verified clock state manually rather than treating the missing measurement as a pass.

## Availability

The first-release architecture is **single-node and is not highly available**. Host failure, maintenance, upgrade, controlled recreation or restore can interrupt service. Actual RPO/RTO are measured acceptance results, not SLA guarantees. Do not market or contract this topology as HA without a separately designed and proven HA architecture.

## Commands and evidence

Run the machine checks on the actual target host **before starting EduTalent**:

```bash
python3 deploy/production/host_preflight.py \
  --require-operations \
  --output /var/lib/edutalent/operations/host-preflight.json

python3 deploy/production/host_network_preflight.py \
  --app-env deploy/production/.env.edutalent \
  --output /var/lib/edutalent/operations/host-network-preflight.json
```

Then run `production-validate`, start the stack, and retain the live database/gateway/AI/Qdrant evidence. Render and retain the container posture. For source-topology review, tags may still be recorded; for the immutable release candidate, digests are mandatory:

```bash
bash edutalent production-config > /var/lib/edutalent/operations/rendered-compose.json
python3 deploy/production/container_hardening_inventory.py \
  /var/lib/edutalent/operations/rendered-compose.json \
  --output /var/lib/edutalent/operations/container-hardening.json

# Final locked release acceptance:
python3 deploy/production/container_hardening_inventory.py \
  rendered-release-compose.json \
  --require-digests \
  --output container-hardening-release.json
```

Install and validate the checked-in `systemd/` maintenance units according to `systemd/README.md`. Use `operations/MAINTENANCE_ROTATION.md` for host/Docker patching, TLS and credential rotation, Qdrant upgrades, and model/profile migration/rollback.

Finally complete `operations/TARGET_HOST_ACCEPTANCE.md` on a clean replacement host. CI evidence alone cannot satisfy PR-13's target-host exit gate.
