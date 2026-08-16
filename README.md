# EduTalent

EduTalent is a Rust/Dioxus school platform with self-hosted PostgreSQL/Supabase, private Qdrant, governed knowledge ingestion, optional AI through a controlled AI Gateway, and a complete air-gapped appliance profile.

The authoritative production/commercial documentation index is [`docs/release/README.md`](docs/release/README.md). The product capability matrix is [`docs/release/feature-matrix.md`](docs/release/feature-matrix.md).

## Quick start for development

Requirements: Docker Engine with Docker Compose v2 and GNU Make.

```bash
make init
make dev
```

Open `http://localhost:8080`. Development uses the repository's lightweight stack and placeholder Supabase settings; login/provisioning require valid configured identity settings.

## Unified command surface

| Command | Purpose |
| --- | --- |
| `make dev` | Hot-reload development stack |
| `make up` / `make down` | Lightweight production-like application stack |
| `make migrate` | Apply canonical migrations transactionally |
| `make build` | Build the local runtime image |
| `make package` | Create the thin application-image bundle |
| `make appliance-build` | Build the complete signed air-gapped appliance |
| `make appliance-verify ARGS=<bundle>` | Verify an extracted appliance manifest/checksums/signature |
| `make production-bootstrap` | Materialize the pinned official Supabase runtime |
| `make production-init` | Generate production secrets after operator configuration |
| `make production-validate` | Verify TLS, secrets, topology and AI/security invariants |
| `make production-up` / `make production-down` | Start/stop production without deleting data |
| `make production-migrate` | Re-run migrations and backend-role configuration |
| `make production-database-check` | Verify the constrained database identity |
| `make production-gateway-check` | Verify non-root/capability-free gateway and TLS key boundary |
| `make production-ai-check` | Verify AI-only egress, auth/tenant boundaries, outage tolerance/recovery |
| `make production-qdrant-check` | Verify authenticated private Qdrant readiness |

The same commands are available through `bash edutalent ...`.

## Production architecture

Production uses the complete pinned official self-hosted Supabase topology with Supabase PostgreSQL as the authoritative database, private authenticated Qdrant, a local AI Gateway, and a single Caddy ingress gateway.

Key invariants:

- only the gateway publishes host TCP ports 80/443;
- PostgreSQL/Supabase internals, Qdrant, Studio/administration internals, TEI and AI Gateway internals are not directly published;
- the gateway runs as a numeric non-root identity with zero effective Linux capabilities;
- the long-running application uses a generated non-superuser database identity separated from bootstrap/migration authority;
- that application identity is `NOBYPASSRLS` and requests establish transaction-scoped PostgreSQL authorization context, so RLS complements server-side role/tenant/object authorization;
- public signup, anonymous/phone auth, Studio AI and default public/internal services not required by EduTalent are disabled/restricted by production configuration;
- the first supported production topology is single-node and is **not highly available**.

Start from [`deploy/production/README.md`](deploy/production/README.md) and the supported-host contract in [`deploy/production/HOST_BASELINE.md`](deploy/production/HOST_BASELINE.md). Real host qualification, measured school RPO/RTO/load and human/operator/security sign-off are manual/external release evidence and are not inferred from CI.

## AI modes

The application and ingestion worker call only the local AI Gateway. Browser/application code does not receive provider credentials or arbitrary provider destinations.

### Connected AI

Connected mode permits only code/configuration-approved provider origins/models. Requests carry internal gateway authentication and authoritative school identity. Provider redirects are disabled. Current production profiles include a fixed OpenAI embedding profile and the approved LLM provider profile documented by the controlled-AI architecture.

### Degraded AI

AI Gateway/provider availability is not a core login/service health criterion. On outage, core school operations remain healthy, AI-backed calls degrade in a controlled way, and durable embedding/personalization work retries with bounded backoff.

### Fully offline/local AI

The local profile uses packaged local model artifacts and a separate immutable embedding/vector collection contract. The complete air-gapped appliance includes required images/models and proves no-pull startup. Automatic fallback between embedding spaces is forbidden.

See [`docs/adr/0002-controlled-external-ai.md`](docs/adr/0002-controlled-external-ai.md) and [`docs/adr/0003-air-gapped-appliance-and-ghcr.md`](docs/adr/0003-air-gapped-appliance-and-ghcr.md).

## Product scope truthfulness

The exact enabled/optional/disabled/excluded scope is documented in [`docs/release/feature-matrix.md`](docs/release/feature-matrix.md) and checked against `docs/release/product-capabilities.json` plus `packages/api/endpoint_authorization_manifest.psv`.

Deliberately disabled first-contract product areas include attendance, timetable management, grade trends, parent reports, parent/teacher messaging, school-manager reports, derived academic metrics, and synthetic in-product health.

Do not infer a commercial feature merely because a database table, legacy component, route stub, or disabled server function exists.

## Production operations and recovery

Production operations include encrypted backups, verified restore, WAL/PITR, Qdrant recovery/reindex procedures, local monitoring/alerts, bounded load/restart exercises, hardened maintenance timers, verified off-appliance backup/WAL copy helpers, and maintenance/key/certificate/model rotation procedures.

Use [`deploy/production/operations/README.md`](deploy/production/operations/README.md), [`deploy/production/operations/MAINTENANCE_ROTATION.md`](deploy/production/operations/MAINTENANCE_ROTATION.md), and the release operator manual [`docs/release/operator-manual-v1.0.md`](docs/release/operator-manual-v1.0.md).

Availability, RPO and RTO become customer commitments only when measured/accepted for the deployed school environment and written into the signed service schedule. CI thresholds are regression evidence, not an SLA.

## Release editions

### Thin application bundle

`make package ARGS=<version>` creates the source-free application-image bundle for connected preparation/use. It is not the complete offline appliance.

### Full air-gapped appliance

`make appliance-build ARGS=<version>` builds the complete production appliance with exact image inventory, local model artifacts, immutable manifest/checksums, SBOMs, signatures/provenance, pinned Supabase runtime, offline installer/secret generation, and registry-disabled first-start proof. Protected release tags publish versioned/commit-addressed custom images; no production `latest` identity is used.

See [`deploy/appliance/README.md`](deploy/appliance/README.md).

## Security boundaries

Production must not:

- bypass transaction-scoped PostgreSQL/RLS or server role/tenant/object authorization;
- retrieve Qdrant material before database authorization or broaden exact asset filters;
- expose unpublished/archived governed material;
- place database/Qdrant/provider/internal-gateway secrets in browser code;
- use an installation-wide fallback school identity for AI;
- mix embedding models/dimensions in one Qdrant collection;
- make AI provider/gateway availability a core service health criterion;
- expose backend/data/internal AI services directly to the host network;
- pull untracked images/models during accepted air-gapped startup;
- accept unsigned/tampered/wrong-platform/untracked appliance payloads.

Architecture/security evidence: [`docs/adr/0001-offline-first-production-architecture.md`](docs/adr/0001-offline-first-production-architecture.md), [`docs/adr/0005-transaction-scoped-rls.md`](docs/adr/0005-transaction-scoped-rls.md), [`docs/security/production-threat-model.md`](docs/security/production-threat-model.md), and [`docs/release/api-security-inventory.md`](docs/release/api-security-inventory.md).

## Contract/privacy/security review pack

The `docs/release/` package contains the versioned operator guide, role guides, feature/exclusion matrix, API/security architecture inventory, privacy/governance draft, security-response/governance process, procurement questionnaire, support/service definition, commercial feature schedule and counsel inputs.

Those documents are engineering/business inputs. Qualified human accessibility, target-host/operator, independent security/penetration, privacy/legal and final contract approvals are tracked separately in the manual/external acceptance PR and must not be replaced by AI or CI evidence.
