# Controlled external AI threat model

## Scope

This document covers the local AI Gateway, application embedding and LLM clients, optional local TEI service, Qdrant profile separation, durable provider-outage handling, and the production `ai_internal` and `ai_egress` networks.

It does not authorize a provider, replace a data-processing agreement, or claim that Docker networking alone is a complete host firewall.

## Assets

- school and tenant isolation;
- provider API keys;
- the internal gateway credential;
- authorized document chunks and assignment context;
- embedding model/version/dimension integrity;
- Qdrant collection separation and exact asset filters;
- durable ingestion jobs and audit records;
- core application availability during provider failure.

## Trust boundaries

1. **Browser to local application:** browsers never receive provider keys, gateway credentials, provider destinations, or hidden authorization metadata.
2. **Application to PostgreSQL:** PostgreSQL is authoritative for school identity, user authorization, publication state, durable queue state, and exact authorized asset IDs.
3. **Application to AI Gateway:** requests require the internal bearer credential and an authoritative non-nil school ID. The internal protocol accepts no destination URL.
4. **AI Gateway to providers:** only the gateway joins `ai_egress`; connected mode uses fixed HTTPS origins and fixed reviewed models.
5. **Application to Qdrant:** PostgreSQL authorization happens first. Qdrant receives the exact authorized asset IDs plus school, publication, and embedding-profile filters.
6. **Gateway to local TEI:** offline embedding mode uses only `http://embedding:80/v1/` on the private `ai_internal` network.

## Threats and controls

### Arbitrary egress or SSRF

Threat: a caller changes a base URL, follows a redirect, supplies credentials in a URL, or reaches another internal/external service.

Controls:

- provider origins are constants in gateway code;
- the app gateway origin is exactly `http://ai-gateway:8090`;
- URLs containing credentials, query strings, or fragments fail closed;
- redirects are disabled in app clients and gateway provider clients;
- only `ai-gateway` joins the non-internal `ai_egress` network;
- production preflight rejects operator-defined provider allowlists and destination drift.

Residual risk: host DNS, routing, or firewall compromise can bypass container-level intent. Production hosts must enforce outbound rules or a controlled proxy for the two approved origins.

### Provider credential disclosure

Threat: provider keys enter the browser, application container, logs, artifacts, database, or Qdrant.

Controls:

- provider keys are gateway-only environment inputs;
- the app receives only the internal gateway token;
- rendered Compose validation rejects provider keys and destinations in the app environment;
- response bodies, prompts, and keys are not logged;
- secret-shaped prompt input is rejected;
- production secret generation never prints values.

### Cross-school context

Threat: a request is charged to or populated with another school’s data, or an installation-wide fallback identity hides missing context.

Controls:

- no default school ID is generated or accepted in production configuration;
- embedding compatibility methods fail closed without an explicit school ID;
- LLM personalization uses the school ID carried by the database-derived student context;
- nil or conflicting school IDs are rejected;
- gateway quotas are keyed by school and operation;
- governed retrieval authorizes in PostgreSQL before Qdrant and filters by school plus exact authorized asset IDs.

### Prompt injection and excessive disclosure

Threat: a PDF or course excerpt instructs the model to reveal secrets, ignore policy, or include unrelated records.

Controls:

- excerpts are explicitly described as untrusted reference data;
- only a bounded number of authorized excerpts and observations are included;
- student IDs, names, teacher names, hidden authorization metadata, and unrelated profiles are omitted from the external prompt;
- credential-shaped content is rejected;
- prompt size, message count, body size, and output tokens are bounded;
- JSON response structure is validated before local persistence.

Residual risk: language-model behavior is probabilistic. Generated content still requires the product’s existing review and authorization workflow.

### Embedding-space corruption

Threat: vectors from different models or dimensions enter one collection, or a model changes without re-indexing.

Controls:

- immutable profile registry binds provider, model, version, dimensions, and collection;
- local and OpenAI profiles use different collection names;
- app configuration, gateway requests, provider responses, and Qdrant operations validate model and dimensions;
- non-finite or wrong-sized vectors are rejected;
- profile metadata is written to governed Qdrant points and included in search and lifecycle filters;
- automatic model failover is prohibited.

### Provider outage, rate limit, or malformed response

Threat: provider failure stops the school system, loses ingestion work, permanently fails valid documents, or causes retry storms.

Controls:

- the app has no startup or health dependency on the gateway;
- gateway health does not call external providers;
- retries apply only to bounded transient conditions;
- per-operation circuit breakers open after repeated failures and recover after a bounded interval;
- school quotas and concurrency semaphores constrain load;
- rate-limit and unavailable responses use controlled error codes without provider bodies;
- durable embedding jobs are requeued in PostgreSQL with capped backoff without consuming their permanent content-failure budget;
- the production acceptance test stops the complete gateway, verifies core app health, restarts it, and verifies recovery.

### Gateway abuse from an internal container

Threat: another internal service obtains the token or submits unbounded requests.

Controls:

- only the app and gateway share `ai_internal` in connected mode; optional TEI is also present only for local embedding mode;
- requests require the internal token and non-nil school ID;
- constant-time token comparison, body limits, schema validation, school quotas, concurrency limits, and operation-specific circuits apply;
- gateway does not receive database, Qdrant, or Supabase credentials.

Residual risk: compromise of the application process exposes the internal gateway token. Rotation and mounted secret files are desirable operational improvements where deployment support permits them.

## Required verification

- invalid internal token returns 401;
- missing or nil school context returns 400 or fails locally;
- arbitrary app/gateway/provider URLs are rejected;
- only the gateway joins `ai_egress`;
- app and data services expose no provider credentials or destinations;
- quota exhaustion and HTTP 429 return bounded retry metadata;
- circuit opens and recovers;
- invalid JSON, wrong model, wrong role, wrong dimensions, non-finite vectors, and oversized responses fail closed;
- provider/gateway outage leaves core health green and durable work queued;
- local and OpenAI profiles cannot share collection or dimension contracts;
- exact-head CI artifacts contain no secret or document payload.

## Operational obligations

Before connected production use, the operator must complete provider security and privacy approval, configure protected provider secrets, enforce host-level egress, review retention and geographic processing, and document incident response. These are required manual controls and must not be inferred from green CI alone.
