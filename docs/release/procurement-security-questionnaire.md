# Procurement and security questionnaire pack

Use these answers as technical inputs and update them for the exact contracted release/host. Qualified reviewers must approve legal/privacy assertions.

| Topic | Current technical answer |
| --- | --- |
| Deployment | Self-hosted single-node production topology; optional complete air-gapped appliance. Not HA. |
| Public ingress | Gateway only on TCP 80/443; administration additionally restricted by configured CIDRs/auth. |
| Database | Self-hosted Supabase PostgreSQL; long-running app uses constrained non-superuser `NOBYPASSRLS` role with transaction-scoped authorization context. |
| Tenant isolation | Role + school/tenant + object authorization in server layer with PostgreSQL RLS defense in depth; negative cross-tenant tests exist for critical journeys. |
| Vector data | Private authenticated Qdrant; retrieval follows database authorization/exact asset filters. |
| Authentication | Supabase-based password authentication; public signup disabled in production. |
| AI | Optional. Only internal AI Gateway may use approved provider egress. Core service degrades safely when AI unavailable. Local/offline profile supported. |
| AI credentials | Gateway-only; not sent to browser/application UI. |
| Encryption in transit | Operator-supplied TLS at gateway; production preflight validates key/cert/hostname/validity. |
| Encryption at rest | Required target-host control; actual disk/storage encryption is human target-host evidence in PR #16, not claimed by CI. |
| Backups | Encrypted full backups + WAL/PITR + verified restore; verified off-appliance copy helpers and separate passphrase requirement. |
| DR | PostgreSQL PITR and Qdrant recovery/reindex procedures; actual school RPO/RTO measured during target-host acceptance. |
| Logging/monitoring | Local operational monitoring/alerts and retained evidence; no requirement for internet telemetry. |
| Secure development | Authorization manifest, targeted/full CI, dependency/config scans, signed release evidence and threat/ADR documentation. |
| SBOM/provenance | Air-gapped/release process produces SBOM/signature/provenance evidence for applicable artifacts. |
| Vulnerability response | Defined in `security-organization.md`; external disclosure channel and contractual targets finalized before production signature. |
| Penetration testing | Required external/human evidence is consolidated in PR #16; do not mark complete until signed. |
| Data location | Core data intended on customer/self-hosted host + approved backup location; optional connected AI adds selected provider processing. |
| Retention/DSR | Draft procedure in `privacy-governance-draft.md`; school-specific/legal approval required. |
| Availability | Single-node/not HA. Availability measurement and SLA, if any, are defined only in signed service schedule after target-host evidence. |
| Disabled product areas | Attendance, timetable, grade trends, parent reports, parent/teacher messaging, school-manager reports, derived academic metrics and synthetic product health. |

Attach/review the feature matrix, API/security inventory, operator manual, privacy draft, support definition and exact release evidence for procurement review.
