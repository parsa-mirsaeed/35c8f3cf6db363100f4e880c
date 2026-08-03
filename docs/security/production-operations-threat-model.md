# Production operations threat model

## Assets

- tenant and identity data in PostgreSQL;
- files in Supabase Storage;
- Qdrant vectors and authorization payloads;
- audit records and durable jobs;
- installation secrets and provider credentials;
- signed release identity and configuration;
- backup archives, WAL, and recovery evidence;
- operations account and rootless Docker control.

## Trust boundaries

The operations command runs on the appliance host under a dedicated operator identity. It may invoke Docker and therefore can inspect or control production containers. That host privilege is never delegated into a container through a Docker socket. Backups cross from live volumes into a plaintext mode-`0700` temporary directory, then into encrypted off-release storage. The passphrase crosses a separate local file boundary and is never put in command output, Compose, or the archive metadata.

## Threats and controls

### Backup theft

Control: AES-256-CBC encryption with PBKDF2-SHA256, strict passphrase-file modes, separate escrow, encrypted secret material only, archive checksum and authenticated inventory. Residual risk: CBC encryption does not provide an independent AEAD tag; manifest checks after decryption detect modification, while release procedure must protect against malicious replacement. A future format may migrate to an authenticated streaming envelope after compatibility review.

### Backup tampering or truncation

Control: atomic creation, archive SHA-256 sidecar, per-file SHA-256/size/mode manifest, immediate decrypt verification, scheduled restore drills. Metadata alone is not trusted as proof of archive contents.

### Plaintext residue

Control: strict umask, temporary mode-`0700` directories, cleanup traps, no plaintext backup destination, no secret in release or logs. Residual risk: host swap, filesystem snapshots, or forensic recovery require encrypted host storage and operating-system controls.

### Passphrase compromise or loss

Control: mode `0400/0600`, separate escrow and access policy, no environment-variable passphrase. Loss prevents recovery; compromise requires credential rotation and backup-media incident response.

### WAL retention disk exhaustion

Control: local staleness/disk alerts, explicit replication-slot runbook, separate WAL directory, retention and off-host copy. An abandoned physical slot can retain WAL; stopping alerts without dropping or advancing the slot is unsafe.

### Restore into production by mistake

Control: routine restore uses a random temporary database and drops it; PITR CI uses isolated containers/volumes; disaster runbook requires replacement storage and approval. No convenience command overwrites production volumes.

### Inconsistent PostgreSQL, Storage, and Qdrant generation

Control: one backup operation captures a generation and release identity; runbook restores all components coherently or rebuilds Qdrant from PostgreSQL. Residual risk: the backup is not a distributed transaction across services. Schools requiring tighter consistency must schedule a quiesced backup window.

### Monitoring data leakage

Control: bounded aggregate metrics only, mode `0600`, no raw logs/content/prompts/secrets/identifiers, no default network delivery.

### Monitoring privilege escalation

Control: no Docker socket in containers; dedicated host account, no `sudo`, systemd hardening, rootless Docker context. Residual risk: Docker control remains host-significant and must be limited to trusted operators.

### Scanner bypass

Control: high/critical Trivy gates, existing all-history secret gate, exact-head artifacts, no broad ignore paths. False-positive suppressions require narrow evidence, owner, and expiry.

### Load test denial of service

Control: explicit duration, concurrency, timeout, error-rate, and p95 limits; CI environment; production execution only in an approved maintenance window. The tool is not exposed as an API.

### AI outage misclassification

Control: alert severity distinguishes provider/AI Gateway warning from core outage. Core health does not depend on external AI.

## Residual risk

- Single-host failure can cause service interruption until restore.
- The backup capture is coordinated but not globally transactional across PostgreSQL, Storage, and Qdrant.
- Rootless Docker access remains sensitive host authority.
- Final RPO/RTO depend on media speed, database size, operator practice, and replacement infrastructure.
- Legal retention and geographic requirements remain deployment-specific.
