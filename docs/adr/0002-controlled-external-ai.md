# ADR 0002: Route approved external AI through one local gateway

- Status: Accepted for New PR 2
- Date: 2026-07-27
- Supersedes: direct provider access from application and worker code

## Context

EduTalent is an offline-first school appliance. The local application, Supabase stack, PostgreSQL, Qdrant, document processing, and administration services must continue to operate when external AI providers are unavailable. External AI traffic also carries school-scoped educational context, so arbitrary destinations, shared tenant identities, provider credentials in application containers, and mixed embedding spaces are unacceptable.

The approved external capabilities are:

1. OpenAI embeddings at the fixed `https://api.openai.com/v1/` origin.
2. The approved LLM at the fixed `https://api.deepseek.com/v1/` origin.

The optional local TEI profile remains available without granting the application direct access to TEI.

## Decision

A dedicated local Rust process, `ai_gateway`, is the only runtime component attached to the non-internal `ai_egress` network. Application and ingestion code call the fixed internal origin `http://ai-gateway:8090` with:

- an installation-specific internal bearer token;
- the authoritative non-nil school ID resolved from PostgreSQL;
- a generated request ID;
- a request body constrained by the shared internal protocol.

The application cannot submit provider URLs or arbitrary provider models. The gateway owns provider credentials, provider origins, retries, circuit breakers, concurrency controls, school quotas, response limits, and provider-response validation. Redirects are disabled. Provider response bodies and document content are not logged.

The core application has no startup or health dependency on the gateway. Provider or gateway failure produces a controlled temporary-unavailable result. Durable embedding jobs are returned to PostgreSQL with bounded backoff rather than being lost or marked as permanent document failures.

## Embedding registry

Embedding model, version, dimensions, provider protocol, and Qdrant collection form one immutable profile contract:

| Profile | Provider | Model | Dimensions | Collection |
| --- | --- | --- | ---: | --- |
| `openai-v1` | OpenAI | `text-embedding-3-small` | 1536 | `edutalent_openai_v1` |
| `local-bge-v1` | local TEI | `BAAI/bge-small-en-v1.5` | 384 | `edutalent_local_bge_v1` |

A profile change requires a distinct collection and complete re-index. Automatic fallback between profiles is forbidden because it would mix vector spaces. Qdrant writes and searches validate the active collection and dimensions, and governed knowledge queries retain PostgreSQL authorization before exact Qdrant asset filtering.

## Operating modes

### Connected AI

The gateway routes embeddings and LLM requests to the two approved HTTPS origins.

### Degraded AI

Login, administration, courses, users, permissions, documents, and existing local data remain available. New provider-dependent work stays queued with bounded retry. AI requests return a controlled temporary-unavailable response. Core health remains green.

### Fully offline embeddings

The gateway routes embedding requests to the private TEI service using the `local-bge-v1` profile. The external LLM is unavailable in this mode. TEI does not receive public ports in production.

## Security consequences

- Only `ai-gateway` joins `ai_egress`.
- Provider keys and destinations never enter the app environment, browser bundle, PostgreSQL, Qdrant, or logs.
- No installation-wide default school ID exists; every request is school-scoped by its caller.
- Requests are bounded by body size, input count, character count, output tokens, concurrency, and school-specific hourly quotas.
- Circuit breakers are separate for embeddings and LLM calls and recover after a bounded open interval.
- LLM prompts contain the smallest necessary authorized context and omit student and teacher identity fields.
- Course excerpts are treated as untrusted data, not instructions; secret-shaped input and excessive prompts fail closed.
- The gateway health endpoint checks only the local gateway process, not provider availability.

## Validation

Final review requires exact-head success for:

- AI Change Proof formatting and focused Rust tests;
- Full Validation database, compile, Clippy, and tests;
- Package definition and release-bundle validation;
- Production Foundation topology validation;
- repeated migrations and backend-role verification;
- complete production-stack startup;
- exclusive AI egress membership;
- gateway authentication and school-header rejection;
- complete gateway outage while core app health remains green;
- gateway restart and recovery.

## Residual risks and later work

Provider approval, data-processing agreements, regional processing, retention, subprocessors, incident notification, and any Zero Data Retention requirement are governance obligations outside this code change. Host-level firewall or outbound proxy enforcement must complement Docker networking for destination-level egress enforcement. Air-gapped image packaging, signatures, provenance, backup/restore, and operational monitoring remain in later Plan V1 pull requests.
