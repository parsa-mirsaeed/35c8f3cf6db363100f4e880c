# Production threat model

## Assets

- student, parent, teacher, manager and school identity/education data;
- authentication/session material, JWT signing/service keys and database credentials;
- governed/class knowledge content, derived text/embeddings and publication state;
- tenant membership, authorization/audit records and durable jobs;
- Qdrant vectors/metadata and embedding-profile integrity;
- provider/internal AI-Gateway credentials and authorized AI context;
- backups/WAL, TLS keys, release artifacts and operational evidence.

## Trust boundaries

1. Public client -> Caddy over operator-supplied TLS.
2. Caddy -> EduTalent or approved Supabase API paths.
3. Authenticated application request -> transaction-scoped PostgreSQL authorization context.
4. Migration/bootstrap administrator -> PostgreSQL (separate from long-running app identity).
5. Authorized application -> private Qdrant / durable ingestion services.
6. Application -> internal AI Gateway using internal credential + authoritative school ID.
7. AI Gateway -> approved connected providers, or -> private local TEI for the local profile.
8. Restricted administrator/operator access -> production host/private admin network.
9. Signed release/update media -> target host.
10. Live data -> encrypted backup/WAL -> verified off-appliance recovery destination.

Docker/network membership is never sufficient authorization by itself. Role, school/tenant and object authorization remain server/database responsibilities.

## Primary threats and controls

### Cross-school data access

Controls include server role/tenant/object checks, direct-ID negative tests, exact endpoint authorization inventory, transaction-scoped PostgreSQL actor context, a long-running `NOBYPASSRLS` non-superuser role, forced RLS on governed application tables, database authorization before Qdrant retrieval, exact authorized asset filters and audit requirements for sensitive operations.

Protected repository work executes through the active authorized transaction; missing transaction scope fails closed rather than silently falling back to a pooled privileged query. Application authorization remains authoritative at the service boundary and PostgreSQL RLS provides defense in depth.

### Database administrator/role compromise

Bootstrap/migration administrator authority is separate. The long-running app/worker role is `NOSUPERUSER`, `NOINHERIT`, `NOCREATEDB`, `NOCREATEROLE`, `NOREPLICATION`, `NOBYPASSRLS`, cannot assume privileged memberships, cannot create public-schema objects or write migration integrity state, and does not receive the bootstrap administrator URL/password.

### Credential disclosure

Controls include no production secrets in Git/images/browser/release payloads, generated mode-restricted environment/secret files, non-printing generators, no Docker socket mounts in product containers, redacted/bounded diagnostics, separate bootstrap/application credentials, gateway-only provider credentials and offline target-host secret generation for the appliance.

### Public exposure / privilege escalation

Only Caddy publishes host ports 80/443. PostgreSQL, Supavisor, Qdrant, Studio, TEI, AI Gateway and internal Supabase services are not directly published. The gateway is non-root and capability-free; administration is separately named/restricted by approved CIDRs and authentication. Rootless Docker is preferred for host operations; rootful daemon use requires tailored review.

### Prompt injection, arbitrary AI egress or external disclosure

The AI Gateway is implemented and is the only component attached to approved AI egress. Provider origins/models are fixed by reviewed code/profile, redirects are disabled, requests require the internal token and authoritative school ID, quotas/body/token/concurrency limits apply, and provider prompts omit unnecessary identity fields. Governed excerpts are untrusted data and authorization occurs before retrieval. Provider/gateway outage is controlled and does not make core school health fail open.

Residual risk: host DNS/routing/firewall compromise can defeat container-network destination intent; target-host qualification must enforce/review upstream egress controls. Connected provider legal/retention/region terms require qualified review before contracting.

### Embedding-space corruption

Immutable profiles bind provider/model/version/dimensions/Qdrant collection. Local and connected embeddings use distinct collections; wrong model/dimension/non-finite responses fail closed; automatic cross-profile fallback is forbidden; model/dimension change requires new collection and re-index.

### Malicious/malformed governed content

Manager/governed submission, quarantine/validation/scanning/isolated parsing, durable jobs, publication checks and no direct teacher PDF ingestion remain controls. Prompt/reference content is never treated as trusted instructions or authorization.

### Supply-chain / offline-media compromise

The full appliance uses exact source SHA/platform/image inventory, local tags, `pull_policy: never`, pinned safe model artifacts, immutable manifest/checksums, SBOMs, signatures/provenance and offline verification. Target secrets are generated after payload verification; TLS keys remain operator supplied. Protected release identities do not use mutable `latest` as the production identity.

### Backup/recovery tampering or unavailability

Encrypted backup, per-file/archive integrity metadata, immediate verification, continuous/periodic WAL/PITR, Qdrant recovery/reindex paths, scheduled restore verification, off-appliance copy verification and separate passphrase escrow reduce loss/tamper risk. Actual school RPO/RTO and genuinely off-appliance storage are target-host/manual acceptance evidence, not CI assumptions.

### Configuration drift/insecure defaults

Preflight rejects placeholders/unsafe auth, TLS mismatch/near-expiry, broad admin CIDRs and topology drift. Live checks verify database identity, non-root gateway, private Qdrant and AI outage/recovery. Host preflight covers supported OS/resources/time/network while encryption/firewall/daemon governance remain controlled-host evidence where not machine-observable.

### Host compromise / denial of service

Container hardening cannot defend against compromised host root. Required controls include patched supported host, restricted administration, encryption at rest, firewall/upstream controls, dedicated operators, bounded resources, backup separation and incident response. Internet-scale WAF/DoS protection beyond the accepted single-host gateway must be allocated to the customer/upstream environment when risk/scale requires it.

## Security invariants

- no direct teacher PDF ingestion;
- durable ingestion/retry state remains authoritative;
- server role/tenant/object authorization cannot be replaced by browser routing or object IDs;
- protected PostgreSQL access uses transaction-scoped actor context with `NOBYPASSRLS` runtime role;
- database authorization precedes Qdrant retrieval and exact authorized asset filters remain exact;
- unpublished/archived governed assets are not retrievable;
- bootstrap/migration credentials never reach long-running application/browser;
- provider credentials/destinations never enter browser code;
- no installation-wide fallback school ID for AI requests;
- only AI Gateway receives approved external-AI egress;
- provider/AI outage never makes core authorization/health fail open;
- only the gateway publishes host ports;
- air-gapped accepted startup does not fetch untracked images/models;
- release/backup/manual evidence is tied to an exact release identity.

## Residual risk

The first production topology is single-node/not HA and shares a host failure domain. Host-root compromise, upstream network attacks, operator mistakes, customer capacity shortfall and third-party/provider legal/security changes remain risks requiring host/security/privacy/contract controls. Backup capture is not a global distributed transaction across every service. RPO/RTO/availability commitments require measured target-host evidence and signed terms. Independent penetration/security review, human accessibility, target-host/operator qualification and qualified privacy/legal approval remain manual/external release gates in PR #16.
