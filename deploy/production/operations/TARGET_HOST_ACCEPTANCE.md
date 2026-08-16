# PR-13 target-host qualification and acceptance record

This record is intentionally **not pre-filled as PASS by CI**. Complete it on the clean supported replacement host using the exact frozen release candidate and immutable release artifacts. Attach or reference compact evidence without copying secrets, private keys, passphrases, real student data, or plaintext backups into the repository.

## Release and host identity

- Release/version:
- Source commit SHA:
- Release manifest digest:
- Immutable appliance/bundle digest:
- Qualification date (UTC):
- Operator/tester:
- Security reviewer:
- Host asset identifier:
- Hardware/vendor/model:
- Architecture:
- Ubuntu version:
- Kernel:
- Root filesystem:
- Data-at-rest encryption evidence/reference:
- CPU cores:
- RAM bytes/GiB:
- Application/data storage total/free:
- Backup storage total/free and device/filesystem identity:
- Off-host backup/WAL destination identity and physical/network separation evidence:
- Docker Engine version:
- Docker Compose version:
- Docker mode: rootless / rootful-reviewed
- Time source / measured clock skew:
- DNS names and resolved addresses:
- Firewall evidence/reference:
- TLS issuer / expiry (do not record private key material):

## Machine-verifiable preflight

Run both pre-start checks **before `production-up`** and retain their JSON outputs:

```bash
python3 /opt/edutalent/deploy/production/host_preflight.py \
  --require-operations \
  --output /var/lib/edutalent/operations/host-preflight.json

python3 /opt/edutalent/deploy/production/host_network_preflight.py \
  --app-env /opt/edutalent/deploy/production/.env.edutalent \
  --output /var/lib/edutalent/operations/host-network-preflight.json
```

- Host-preflight evidence path/hash:
- Host-preflight automatic result: PASS / FAIL
- Host-preflight pending manual items disposition:
- Network-preflight evidence path/hash:
- Exactly three configured DNS names resolved: yes / no
- Pre-start TCP 80/443 free for gateway: yes / no
- Time synchronization verified: yes / no / manual evidence
- Measured clock skew <= 60 seconds: yes / no / manual evidence
- `production-validate`: PASS / FAIL
- `production-database-check`: PASS / FAIL
- `production-gateway-check`: PASS / FAIL
- `production-qdrant-check`: PASS / FAIL
- `production-ai-check`: PASS / FAIL

Automatic host/network preflight PASS is necessary but not sufficient. Encryption, firewall/daemon tailoring, passphrase escrow/off-host location, and replacement-host recovery remain controlled acceptance evidence.

## CIS Docker / container hardening

- CIS Docker Benchmark version used: 1.8.0
- Host assessment tool/process:
- Assessment artifact path/hash:
- Fail findings:
- Not-applicable findings + rationale:
- Accepted risks (owner + date + rationale + review/expiry):

Render the exact locked release configuration and retain:

```bash
python3 deploy/production/container_hardening_inventory.py \
  rendered-release-compose.json \
  --require-digests \
  --output container-hardening-inventory.json
```

- Container hardening inventory path/hash:
- All release images immutable SHA-256 digests: yes / no
- No privileged containers: yes / no
- No host networking: yes / no
- No Docker socket mounts: yes / no
- Per-container user/group reviewed: yes / no
- Capability set reviewed: yes / no
- Writable paths/read-only root reviewed: yes / no
- PID/resource limits reviewed: yes / no
- Restart/health behavior reviewed: yes / no
- Networks/published ports reviewed: yes / no
- seccomp/AppArmor/security options reviewed: yes / no

## Maintenance automation

- Dedicated `edutalent-operator` account verified: yes / no
- Rootless Docker socket / rootful reviewed Docker context verified: yes / no
- `edutalent-wal.service` enabled/running: yes / no
- `edutalent-wal-verify.timer` enabled: yes / no
- `edutalent-offhost-wal.timer` enabled: yes / no
- `edutalent-monitor.timer` enabled: yes / no
- `edutalent-backup.timer` enabled: yes / no
- `edutalent-offhost-copy.timer` enabled: yes / no
- `edutalent-restore-verify.timer` enabled: yes / no
- Operations state permissions verified: yes / no
- Backup mount separate from production/data filesystem: yes / no
- Backup passphrase permissions 0400/0600: yes / no
- Passphrase escrow separate from backup media: yes / no
- Encrypted off-host backup copy SHA-256 verified: yes / no
- Completed WAL segments copied/verified off-host: yes / no
- Off-host WAL copy frequency satisfies approved RPO: yes / no

## Clean replacement-host recovery

Perform the test from an unconfigured supported host, not by restoring into the original production machine.

- Fresh installation from signed/verified artifact: PASS / FAIL
- Production topology validation: PASS / FAIL
- Live security boundary checks: PASS / FAIL
- Encrypted full backup created and independently verified: PASS / FAIL
- Backup copied to genuinely off-appliance target and verified: PASS / FAIL
- Backup restored on replacement host: PASS / FAIL
- PostgreSQL PITR completed to selected target using verified WAL: PASS / FAIL
- Qdrant snapshot recovery or documented reindex path completed: PASS / FAIL
- Application/database restart recovered correctly: PASS / FAIL
- Disk-low/full condition failed closed and alerted: PASS / FAIL
- Near-expiry/invalid TLS failed preflight: PASS / FAIL
- Corrupted/unsafe configuration failed closed: PASS / FAIL
- Failed migration rolled back without partial state: PASS / FAIL
- AI provider outage preserved core operation: PASS / FAIL
- AI Gateway outage preserved core operation: PASS / FAIL
- Qdrant outage degraded retrieval without misreporting core health: PASS / FAIL
- Evidence references/notes:

## Measured recovery

Do not turn objectives into contractual guarantees without these measurements and business/legal approval.

- Recovery scenario measured:
- Last safely recoverable data timestamp:
- Failure/injection timestamp:
- Service restoration timestamp:
- **Measured RPO:**
- **Measured RTO:**
- Data-integrity verification result:
- Deviations from provisional objectives:
- Follow-up owner/date:

## School-scale load / soak

Record the expected school profile before testing.

- Synthetic users/students/classes/documents represented:
- Concurrent sessions/users:
- Test duration:
- Request/workflow mix:
- CPU peak/steady state:
- RAM peak/steady state:
- Storage growth / IOPS observations:
- Database connection peak:
- p50/p95/p99 latency:
- Error rate:
- AI/Qdrant degraded-mode observations:
- Alert thresholds exercised:
- Capacity headroom conclusion:
- Evidence path/hash:

## Upgrade, patching and rotation rehearsal

Use `MAINTENANCE_ROTATION.md` plus `DEPLOYMENT_UPGRADE.md`; do not improvise credential or data-format changes in production.

- Host OS/Docker patch + post-reboot validation: PASS / FAIL
- Application release upgrade + rollback: PASS / FAIL
- Supabase upgrade/rollback procedure reviewed/rehearsed as applicable: PASS / FAIL
- Qdrant upgrade/recovery procedure reviewed/rehearsed as applicable: PASS / FAIL
- Model/profile change into a versioned collection + rollback: PASS / FAIL
- TLS certificate rotation: PASS / FAIL
- Application database credential rotation: PASS / FAIL
- AI Gateway internal/provider key rotation: PASS / FAIL
- Qdrant API-key rotation: PASS / FAIL
- Supabase JWT/API-key rotation reviewed/rehearsed as applicable: PASS / FAIL
- Backup-passphrase generation rotation + old-archive escrow handling: PASS / FAIL
- Notes/evidence:

## Availability statement

The first-release topology is **single-node and not highly available**. A controlled restart, host failure, upgrade, restore, or other single-node event can interrupt service. Do not describe this deployment as HA or convert measured RPO/RTO into an SLA unless a separately designed and proven HA architecture and contractual approval exist.

- School/operator acknowledges single-node topology: yes / no
- Contract/support language aligned to measured evidence: yes / no

## Findings and residual risks

For every finding include severity, owner, remediation or accepted-risk decision, evidence, review/expiry date.

- Findings:
- Residual/accepted risks:

## Acceptance decision

Choose exactly one:

- [ ] not accepted
- [ ] safe to continue developing
- [ ] ready for final validation
- [ ] ready for limited pilot

`ready for contracted production` is a later PR-15/G4 decision and additionally requires the privacy/legal/contract, independent review, human accessibility, and final exact-head release evidence defined by the production-readiness plan.

- Operator sign-off:
- Security sign-off:
- Date:
- Exact release/source SHA reverified unchanged:
