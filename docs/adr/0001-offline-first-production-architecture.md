# ADR 0001: Offline-first self-hosted production architecture

- Status: Accepted
- Date: 2026-07-10
- Decision owners: EduTalent engineering

## Context

EduTalent serves schools that may operate with highly restricted or unreliable
internet access. Identity, authorization, course operations, governed document
storage, ingestion queues, vector search, and administrative functions must
continue locally. Only approved embedding and LLM providers may require outbound
connectivity, and their loss must not make the core school system unhealthy.

The previous production-like Compose stack used a standalone PostgreSQL
container while Supabase authentication remained external. It also published
PostgreSQL and Qdrant ports and required target hosts to pull supporting images.
That topology is useful for development but is not the production trust model.

## Decision

1. Production uses the complete official self-hosted Supabase Docker topology,
   pinned to an immutable upstream commit and prepared before deployment.
2. Supabase PostgreSQL is the single authoritative application database.
   EduTalent migrations run against that database; no second production
   PostgreSQL database is permitted.
3. Caddy is the only service that publishes host ports. It terminates static TLS
   and exposes only the EduTalent app, approved Supabase API prefixes, and a
   separately named administration host.
4. Supabase API, administration, and data services use separate internal Docker
   networks. PostgreSQL, Supavisor, Qdrant, Studio, and internal APIs publish no
   host ports.
5. Qdrant remains the vector engine for this phase because its exact metadata
   filters are already part of the governed retrieval security model. It is
   private-network-only and requires an API key.
6. The existing local TEI service remains an explicit optional profile. It is
   never silently mixed with vectors from another embedding model.
7. Public signup, anonymous users, phone authentication, cloud SMTP defaults,
   the Studio AI assistant, and unauthenticated Edge Functions are disabled by
   default.
8. Production secrets are generated after deployment configuration is created,
   stored outside Git, permission-restricted, and never printed by EduTalent
   tooling.
9. Runtime startup never fetches floating upstream code or TLS certificates.
10. External embedding and LLM access will be added through a dedicated local AI
    gateway in a separate change, with egress allowlists, quotas, circuit
    breakers, and audit controls.

## Consequences

- A connected preparation host is needed to materialize the pinned Supabase
  runtime until the full air-gapped appliance bundle is implemented.
- The production server requires operator-supplied certificates and sufficient
  resources for the complete Supabase stack.
- Existing development commands remain available and intentionally use the
  smaller development stack.
- Provider outages leave embedding jobs durable and retryable; AI-dependent
  features may degrade while the school system remains available.
- High availability is not provided by a single-host Compose deployment. It
  requires a future multi-node database/storage/vector design.

## Follow-up decisions

- Controlled AI gateway and provider contract.
- Full offline image/model bundle, SBOM, signing, provenance, and GHCR release.
- Backup/PITR, Qdrant snapshots, restore drills, observability, and SLOs.
- Benchmark Qdrant versus pgvector using real tenant-filtered school workloads.
