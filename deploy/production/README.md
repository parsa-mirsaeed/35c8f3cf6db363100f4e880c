# EduTalent production foundation

This deployment is separate from the lightweight development stack. It uses the
complete official self-hosted Supabase Docker topology at the immutable commit
recorded in `SUPABASE_UPSTREAM`, one authoritative Supabase PostgreSQL database,
private Qdrant, static TLS, and a single public reverse proxy.

## Capacity baseline

The complete stack is substantially larger than the development Compose file.
Use at least 4 CPU cores, 8 GB RAM, and SSD storage for a small production
installation; capacity must be load-tested against the expected schools,
concurrency, storage, Realtime use, and document workload. Service CPU and memory
ceilings are explicit environment settings, and preflight rejects any configured
CPU limit larger than the Docker host capacity.

## Preparation

Requirements:

- Linux host with Docker Engine and Docker Compose v2.24.4 or newer;
- Git, OpenSSL, Python 3, Node.js 16 or newer, Bash, and GNU core utilities;
- three distinct DNS names for app, Supabase API, and administration;
- an operator-supplied TLS certificate/private key covering all three names and
  having at least 14 days of remaining validity;
- the EduTalent image already built or loaded locally.

Materialize the exact official Supabase deployment on a connected preparation
host:

```bash
make production-bootstrap
```

This fetches only the immutable commit in `SUPABASE_UPSTREAM`. Production
startup itself performs no Git fetch. The bootstrap is idempotent for the same
pin and deliberately never deletes or replaces an existing runtime. A Supabase
version change requires reviewed backup, upgrade, validation, and rollback
steps.

## Initial configuration

Create the operator environment template:

```bash
make production-init
```

The first invocation creates `deploy/production/.env.edutalent` and stops. Edit:

- `APP_DOMAIN`;
- `SUPABASE_DOMAIN`;
- `ADMIN_DOMAIN`;
- `ADMIN_ALLOWED_CIDRS` to the exact management VPN/network ranges;
- absolute `TLS_CERT_FILE` and `TLS_KEY_FILE` paths;
- `GATEWAY_UID` and `GATEWAY_GID` when the default numeric identity conflicts
  with local policy;
- `DATABASE_APP_USER` when the default `edutalent_app` conflicts with local
  naming policy;
- application image/tag and resource-specific settings where needed.

Administration is loopback-only by default. Do not use `0.0.0.0/0` or `::/0`.
Keep the operator TLS private key mode 600. A one-shot, network-disabled
initialization container copies it into a Docker-managed volume and recursively
assigns Caddy's persistent `/data` and `/config` volumes to
`GATEWAY_UID:GATEWAY_GID`. It changes ownership only inside Docker-managed
volumes and then exits. The long-running Caddy gateway uses the same numeric
non-root identity and never bind-mounts the host private-key path directly.

Run initialization again:

```bash
make production-init
```

It invokes the pinned official Supabase key generators, suppresses their secret
output, creates asymmetric JWT/API keys, generates separate Qdrant and
`DATABASE_APP_PASSWORD` credentials, sets strict-network authentication
defaults, and stores environment files with mode 600. Local Node.js is required
so the upstream helper cannot silently pull its fallback container image. A
failed initialization removes the partial `.env` and restores the original
pinned Compose file. It intentionally refuses to overwrite an existing
successful Supabase `.env`; key rotation is a separate migration procedure.

## Database authorization boundary

PostgreSQL duties are separated:

- the Supabase `postgres` identity is available only to the migration container
  and the one-shot `database-access` role configurator;
- the long-running web server and durable worker connect as the generated
  `DATABASE_APP_USER` and never receive `POSTGRES_PASSWORD` or an administrator
  URL;
- that backend role is `NOSUPERUSER`, `NOINHERIT`, `NOCREATEDB`, `NOCREATEROLE`,
  and `NOREPLICATION`;
- it receives public-schema data manipulation, sequence, type, and function
  privileges, but no schema creation or migration-registry write privileges.

The backend role is deliberately `BYPASSRLS` today. EduTalent's repositories use
a shared direct PostgreSQL pool and perform authenticated server-side
membership/tenant authorization without setting transaction-local database
identity context. Making this role `NOBYPASSRLS` before that context exists would
break legitimate server operations rather than add reliable defense. Supabase
browser/client roles remain subject to the existing forced RLS policies, and the
backend role is materially narrower than a superuser. Replacing this residual
server-authority boundary with request-scoped `NOBYPASSRLS` enforcement is
tracked in issue #8.

`production-migrate` applies migrations with the bootstrap administrator and
then reruns the idempotent backend-role configurator, ensuring grants cover newly
created application objects while migration integrity tables remain protected.

## Validate and start

```bash
make production-validate
make production-up
make production-ps
make production-database-check
make production-gateway-check
make production-qdrant-check
```

Preflight verifies:

- Docker Compose is new enough for the required merge/reset semantics;
- the materialized Supabase commit matches the repository pin;
- secret files and the TLS private key are not group/world readable;
- upstream placeholder secrets are gone;
- application and bootstrap database credentials are present, distinct, and
  structurally valid;
- unsafe authentication defaults remain disabled;
- domains are distinct and the admin CIDRs are not internet-wide;
- the TLS certificate/key match, cover every hostname, and are not near expiry;
- service CPU limits fit the Docker host;
- the effective Compose configuration renders;
- only Caddy publishes host ports 80 and 443;
- host ports 80/443 map to unprivileged Caddy listener ports 8080/8443;
- every service network is internal or explicitly disabled for the bounded
  gateway initialization step;
- the gateway reads TLS files only from the staged read-only Docker volume;
- gateway initialization owns the persistent `/data` and `/config` volumes;
- the initialization owner and long-running gateway numeric UID/GID match;
- the long-running gateway requires no Linux capabilities;
- no service is privileged, uses host networking, or mounts the Docker socket;
- Edge Functions remain behind an explicit disabled-by-default profile and have
  no public `/functions/v1` route;
- migrations and role provisioning use the bootstrap identity, while the app
  waits for role provisioning and uses only the generated backend identity;
- EduTalent uses internal Kong, the explicit self-hosted JWT issuer, and private
  Qdrant.

`production-database-check` connects from the live app container and verifies the
current role is the configured backend identity, is not a superuser, cannot
create roles/databases/schema objects or modify migration registries, and has
only the explicitly documented `BYPASSRLS` server-authority flag.

`production-gateway-check` inspects the live container and proves that Caddy is
non-root, has a zero effective capability mask, can read the staged certificate
and private key, sees the private key as mode 600 with ownership matching its
running UID/GID, and owns writable persistent `/data` and `/config` volumes.

Qdrant readiness is intentionally not a core startup gate. Existing school
operations and authentication can start while vector search is temporarily
unavailable, and durable ingestion work remains retryable. The explicit
`production-qdrant-check` command probes Qdrant's authenticated `/readyz`
endpoint from inside the app container, using the same private network and key as
the application. This avoids modifying the official Qdrant image or depending on
shell utilities that are not present in it.

Useful commands:

```bash
make production-logs
make production-migrate
make production-database-check
make production-gateway-check
make production-qdrant-check
make production-down
```

`production-down` does not delete database, storage, Qdrant, TLS staging, or
Caddy volumes. There is deliberately no convenience command that destroys
production data.

## Public surfaces

- `https://APP_DOMAIN`: EduTalent;
- `https://SUPABASE_DOMAIN`: only approved Auth, REST, Realtime, Storage, and
  GraphQL prefixes;
- `https://ADMIN_DOMAIN`: Supabase administration, restricted first by source
  CIDR and then by Kong's generated dashboard basic authentication.

PostgreSQL, Supavisor, Qdrant, Studio, Auth, PostgREST, Realtime, Storage, Edge
Runtime, and metadata services publish no host ports.

## Optional Edge Functions

The official Edge Runtime definition remains in the coordinated Supabase
configuration but is assigned to the `edge-functions` Compose profile. It is not
started by the default production command and `/functions/v1` is not exposed by
Caddy. EduTalent does not depend on Edge Functions. Enabling that profile later
requires a separate health, function-authentication, ingress, and resource
acceptance review rather than silently expanding the public surface.

## Authentication policy

Public signup, anonymous users, phone signup, and unauthenticated Edge Functions
are disabled. Cloud SMTP is not required. Schools may provision users through
approved administrative flows and perform recovery locally. A local SMTP relay
can be configured later without granting general internet access.

EduTalent validates Supabase ES256 tokens against the local JWKS endpoint and the
explicit public self-hosted issuer `https://SUPABASE_DOMAIN/auth/v1`; it does not
assume the managed `*.supabase.co` issuer. Mixed JWKS containing the active ES256
key and the legacy symmetric compatibility key are parsed safely, but only the
matching ES256 key is accepted for user-token validation.

## Embeddings

The existing local TEI service is retained only as an explicit compatibility
profile. Set:

```dotenv
EMBEDDING_PROFILE=local-embedding
```

Vectors from different models must use separate versioned Qdrant collections.
The external OpenAI embedding and LLM gateway, egress policy, and provider outage
handling are implemented in the next dedicated PR; this foundation does not
pretend those controls already exist.

## Not yet an air-gapped release bundle

The checked-in bootstrap prepares the full upstream configuration, but Docker
images are not yet all exported into one offline appliance archive. The release
follow-up will include every image/model, immutable digest manifest, SBOM,
signatures, provenance, offline installer, and GHCR publication.
