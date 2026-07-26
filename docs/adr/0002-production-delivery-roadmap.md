# ADR 0002: Production delivery roadmap

- Status: Accepted
- Date: 2026-07-10

The production program is intentionally split into independently reviewable
security boundaries:

1. **Production foundation**: pinned self-hosted Supabase, one authoritative
   database, private Qdrant, static TLS, isolated networks, generated secrets,
   and fail-closed topology validation.
2. **Controlled external AI**: local AI gateway, OpenAI embeddings, approved LLM
   integration, provider allowlists, egress controls, quotas, timeouts, circuit
   breakers, model/version registry, and outage tests.
3. **Air-gapped delivery**: all runtime images and optional model artifacts,
   immutable digests, SBOMs, signatures, provenance, offline installer, upgrade
   and rollback tooling, and GHCR publishing.
4. **Production operations**: PostgreSQL PITR, Storage and Qdrant recovery,
   monitoring, alerting, restore drills, load/soak tests, SLOs, and incident
   runbooks.

A later phase may not weaken an invariant established by an earlier phase. Each
phase must preserve tenant isolation, governed document lifecycle rules,
durable ingestion, exact vector authorization filters, and server-only secrets.
