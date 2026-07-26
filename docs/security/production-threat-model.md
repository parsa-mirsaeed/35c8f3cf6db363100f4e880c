# Production threat model

## Assets

- student, teacher, manager, and school identity data;
- authentication credentials, JWT signing keys, service-role keys, and database
  credentials;
- governed PDFs and derived text;
- tenant membership, authorization, publication state, and audit records;
- embeddings and vector metadata;
- durable ingestion jobs and retry state;
- backups, TLS private keys, release artifacts, and operational logs.

## Trust boundaries

1. Public client to Caddy over TLS.
2. Caddy to EduTalent or the Supabase gateway.
3. Application/API services to the private data network.
4. Migration/bootstrap administrator to PostgreSQL.
5. Long-running EduTalent backend role to PostgreSQL.
6. Administrative access to the private admin network.
7. Future AI gateway to approved external providers.
8. Release preparation and update media to the production host.

Docker network membership is not an authorization decision. Every request must
still be authenticated, authorized, validated, and scoped to the active tenant.

## Primary threats and controls

### Cross-school data access

Controls: PostgreSQL/RLS policy tests for Supabase client roles, application
authorization before vector search, exact authorized asset filters, non-public
Qdrant, negative tenant tests, and audit logging. Direct object identifiers must
not bypass these checks.

The current Rust repositories use a shared direct PostgreSQL pool and do not yet
set transaction-local request identity. Therefore the long-running server uses a
dedicated non-superuser backend role with intentional `BYPASSRLS`, while
server-side membership/tenant authorization remains authoritative. This is
narrower than a PostgreSQL superuser but is still a residual trusted-computing
boundary. Issue #8 tracks request-scoped database context and conversion to a
`NOBYPASSRLS` role.

### Database administrator compromise or leakage

Controls: the Supabase `postgres` credential is restricted to the migration and
one-shot role-provisioning services. The long-running app never receives it. The
backend role is generated separately, cannot create roles or databases, cannot
replicate, is not a superuser, cannot create persistent public-schema objects,
and cannot write migration-integrity tables. Runtime and PostgreSQL 17 CI checks
verify these properties.

### Credential disclosure

Controls: no secrets in Git/images/browser bundles, mode-600 environment files,
non-printing generators, no Docker socket mounts, redacted diagnostics, static
secret scanning, separate server-only Supabase secret credentials, and distinct
bootstrap/application database credentials.

### Public exposure of internal services

Controls: rendered-Compose CI verification; only Caddy may publish host ports;
only Caddy joins the dedicated host-ingress bridge; and PostgreSQL, Supavisor,
Qdrant, Studio, and internal Supabase services remain exclusively on internal
networks. The ingress bridge contains no backend or data service.

### Malicious or malformed PDFs

Controls retained from the governed workflow: manager-only submission,
quarantine, size/type/magic-byte validation, local malware scanning, isolated
parsing, durable jobs, duplicate prevention, publication checks, and no direct
teacher ingestion.

### Prompt injection and external data disclosure

The production foundation does not yet enable external AI. The follow-up AI
gateway must minimize authorized context, reject user-controlled provider URLs,
apply request limits and timeouts, avoid sensitive logging, and audit provider
use. Provider failure must not bypass authorization or delete queued jobs.

### Supply-chain compromise

Controls: immutable upstream Supabase commit, pinned image tags in the official
coordinated stack, exact source-build application image, CI checks, and no
runtime fetching. Follow-up delivery adds digest pinning, SBOMs, signatures,
provenance, vulnerability policy, and offline image manifests.

### Configuration drift or insecure defaults

Controls: generated secrets, placeholder rejection, TLS key/certificate matching,
unsafe-auth-setting rejection, database-role attribute/privilege verification,
and machine-verifiable Compose exposure rules. Historical migrations remain
checksum protected and fail closed if modified.

### Host compromise

Container controls reduce blast radius but do not protect a compromised root
host. Required operational controls include a patched minimal OS, restricted
SSH/VPN administration, full-disk or volume encryption, firewall policy,
separate encrypted backups, audit forwarding, and incident-response procedures.

## Security invariants

- no direct teacher PDF ingestion;
- durable ingestion queue remains authoritative;
- duplicate jobs remain prevented;
- database authorization precedes Qdrant retrieval;
- Qdrant filters contain exactly the authorized asset identifiers;
- unpublished and archived assets are not retrievable;
- migration/bootstrap credentials never reach the long-running application;
- Supabase client roles remain governed by RLS;
- external provider failures never make the core application authorization
  fail open;
- production secrets never reach browser code or release archives;
- only the reverse proxy publishes host ports;
- only the future AI gateway may receive external-AI egress permissions.

## Residual risk

A single-host deployment has a shared host and shared failure domain. The backend
role's intentional RLS bypass remains a trusted server boundary until issue #8
is complete. Static TLS certificate renewal, offline update media, provider
data-processing terms, country-specific education/privacy obligations, and
disaster-recovery targets remain operator and governance responsibilities. No
architecture can guarantee immunity from all attacks; these controls provide
defense in depth and explicit failure behavior.
