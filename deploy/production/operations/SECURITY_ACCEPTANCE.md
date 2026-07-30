# Final security and operational acceptance suite

A production candidate is accepted only on the exact release commit and exact appliance artifacts. A skipped test is not a pass. Record evidence, date, operator, environment, release digest, and residual risk.

## Offline and network

- signed release, checksum, SBOM, and provenance verification;
- first startup with registry access disabled and no image pull;
- only gateway host ports 80/443;
- only AI Gateway on the external AI egress network;
- no privileged container, host network, Docker socket, or unexpected capability;
- both providers unavailable while login and core school functions remain healthy;
- administrative surface restricted by management CIDR and authentication.

## Tenant, authorization, and documents

- School A cannot enumerate or retrieve School B users, courses, assets, jobs, or vectors;
- direct IDs cannot bypass PostgreSQL authorization;
- RLS is forced and tested for every browser/client tenant table;
- Qdrant receives school, publication state, and exact authorized asset filters;
- unpublished and archived assets are not retrievable;
- teacher workflows cannot bypass manager approval;
- malformed, oversized, MIME-spoofed, bomb, malware, parser-crash, duplicate, interrupted, and unauthorized publication cases fail safely;
- durable jobs are neither lost nor permanently failed by provider outage.

## AI

- provider timeout, 429, invalid JSON, redirect, wrong model, wrong dimension, oversized and chunked response;
- circuit breaker, bounded retry, quota, and recovery;
- prompt injection inside documents remains untrusted content;
- cross-tenant context and secret-shaped input are rejected;
- provider credentials never reach app/browser/logs;
- external provider loss is not core health failure.

## Operations

- encrypted full backup contains PostgreSQL logical and physical data, Storage, Qdrant, WAL, release identity, and encrypted secret escrow;
- backup creation fails before writing when free capacity is below threshold;
- checksum, mode, inventory, decryption, and wrong-passphrase failure;
- temporary logical restore;
- PostgreSQL base backup plus WAL point-in-time recovery;
- Qdrant snapshot delete-and-recover;
- failed migration transaction leaves no partial object;
- database restart and application controlled recreation recover;
- corrupted configuration and unsafe admin CIDR fail closed;
- near-expiry certificate fails preflight;
- sustained load/soak remains within approved error and p95 limits;
- backup, WAL, disk, TLS, database pressure, core, Qdrant, and AI alerts are exercised;
- restore runbook is executed on a replacement environment.

## Security scanning

- Trivy high/critical dependency vulnerability scan;
- Trivy high/critical infrastructure misconfiguration scan;
- repository all-history secret scan;
- custom image SBOM and vulnerability evidence;
- signed multi-architecture release provenance;
- review of licences, base image pins, and known residual findings.

## Acceptance result

Classify the candidate as exactly one of:

- **safe to continue developing** — focused checks pass, final evidence incomplete;
- **ready for final validation** — implementation stable, full-validation label may be applied;
- **ready to merge** — every required exact-head job and artifact is successful, review threads are resolved, and residual risks are documented;
- **not accepted** — any required test failed, was canceled, was skipped unexpectedly, or ran on another SHA.
