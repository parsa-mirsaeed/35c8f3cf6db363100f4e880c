# EduTalent production foundation

This deployment is separate from the lightweight development stack. It uses the complete official self-hosted Supabase Docker topology at the immutable commit recorded in `SUPABASE_UPSTREAM`, one authoritative Supabase PostgreSQL database, private Qdrant, static TLS, an internal AI Gateway, and a single public Caddy reverse proxy.

Only the Caddy gateway joins the dedicated non-internal `edutalent-ingress` bridge required for host-port publication. Only the AI Gateway joins the approved external AI egress bridge. Application, Supabase, administration, Qdrant, database, document and internal AI traffic remain on the corresponding private/internal networks.

## Supported host and qualification

The first target-host qualification baseline is version controlled in `host-baseline.json` and explained in `HOST_BASELINE.md`. It currently defines Ubuntu Server 24.04 LTS on x86_64, kernel 6.8.0 or newer, Docker Engine 24.0 or newer, Docker Compose 2.24.4 or newer, at least 4 CPU cores, 8 GiB RAM, SSD-backed supported filesystems, and explicit free-space/inode floors.

This topology is **single-node and is not highly available**. Capacity, RPO and RTO must be measured on the target school workload; they are not contractual guarantees merely because a threshold exists in CI or a runbook.

Before target-host acceptance, retain the live host-preflight evidence:

```bash
python3 deploy/production/host_preflight.py \
  --require-operations \
  --output /var/lib/edutalent/operations/host-preflight.json
```

Automatic host checks do not prove disk encryption, firewall policy, rootful-daemon governance, off-host backup separation, passphrase escrow, replacement-host recovery, or school-scale load. Those are recorded in `operations/TARGET_HOST_ACCEPTANCE.md`. See `operations/CIS_DOCKER_1_8_0.md` for the tailored Docker benchmark evidence map and `systemd/README.md` for installable maintenance timers/services.

## Preparation

Requirements in addition to the supported-host baseline:

- Git, OpenSSL, Python 3, Node.js, Bash, GNU core utilities and the Docker CLI/Compose plugin;
- three distinct DNS names for app, Supabase API and administration;
- an operator-supplied TLS certificate/private key covering all three names and having at least 14 days of remaining validity;
- the exact EduTalent release image/artifacts already built or loaded locally for offline production startup.

Materialize the exact official Supabase deployment on a connected preparation host:

```bash
make production-bootstrap
```

This fetches only the immutable commit in `SUPABASE_UPSTREAM`. Production startup itself performs no Git fetch. Bootstrap is idempotent for the same pin and deliberately never deletes or replaces an existing runtime. A Supabase version change requires reviewed backup, upgrade, validation and rollback steps.

## Initial configuration

Create the operator environment template:

```bash
make production-init
```

The first invocation creates `deploy/production/.env.edutalent` and stops. Configure at least:

- `APP_DOMAIN`;
- `SUPABASE_DOMAIN`;
- `ADMIN_DOMAIN`;
- `ADMIN_ALLOWED_CIDRS` to the exact management VPN/network ranges;
- absolute `TLS_CERT_FILE` and `TLS_KEY_FILE` paths;
- `GATEWAY_UID` and `GATEWAY_GID` when the default numeric identity conflicts with local policy;
- `DATABASE_APP_USER` when the default `edutalent_app` conflicts with local naming policy;
- the approved embedding/AI profile and any release-specific resource limits.

Do not use `0.0.0.0/0` or `::/0` for administration. Keep the operator TLS private key mode `0600`. A bounded one-shot, network-disabled initialization service copies TLS material into a Docker-managed volume and assigns Caddy's persistent `/data` and `/config` volumes to the configured non-root gateway UID/GID. The long-running Caddy container never bind-mounts the operator private-key path directly.

Run initialization again after editing the template:

```bash
make production-init
```

Initialization invokes the pinned official Supabase key generators while suppressing secret output, creates asymmetric JWT/API keys, generates separate Qdrant, backend database and AI-Gateway credentials, sets strict authentication/network defaults, and stores environment files with mode `0600`. A failed initialization removes partial output; an existing successful Supabase environment is not overwritten because key rotation is a separate controlled procedure.

## Database authorization boundary

PostgreSQL duties are separated:

- the bootstrap `postgres` identity is available only to migrations and the one-shot database-role configurator;
- the long-running web server and durable worker connect as the generated `DATABASE_APP_USER` and never receive `POSTGRES_PASSWORD` or an administrator URL;
- the application role is `NOSUPERUSER`, `NOINHERIT`, `NOCREATEDB`, `NOCREATEROLE`, `NOREPLICATION` and **`NOBYPASSRLS`**;
- protected repository queries execute inside request/job-scoped transactions carrying the authorized database context;
- the runtime role has only the data/function/sequence privileges required for application operation and cannot create schema objects or modify the migration registry.

`production-migrate` applies migrations with bootstrap authority and then reruns the idempotent backend-role configurator so grants cover newly created application objects without expanding long-running runtime authority.

`production-database-check` connects from the live app container and verifies the exact backend identity, `rolbypassrls = false`, absence of superuser/role/database/schema-creation authority, and protection of the migration registry.

## Validate and start

```bash
make production-validate
make production-up
make production-ps
make production-database-check
make production-gateway-check
make production-ai-check
make production-qdrant-check
```

The production preflight and rendered-topology validator prove, among other invariants:

- the materialized Supabase commit matches the repository pin;
- secret files and the TLS private key are not group/world readable;
- upstream placeholder secrets are gone and application/bootstrap database credentials are distinct;
- public signup/anonymous/phone and other unsafe authentication defaults remain disabled;
- application/Supabase/admin domains are distinct and admin CIDRs are not internet-wide;
- TLS certificate/key match, cover every hostname and are not near expiry;
- configured service CPU limits do not exceed host capacity;
- only Caddy publishes host ports 80/443, mapped to unprivileged internal ports 8080/8443;
- no production service is privileged, uses host networking, or mounts the Docker socket;
- application/data/admin/internal-AI networks are internal;
- only Caddy has ingress membership and only the AI Gateway has external AI egress membership;
- the gateway TLS/state ownership boundary and capability-free long-running identity are preserved;
- migrations use bootstrap authority and the app waits for the constrained database-role configurator;
- the app receives neither provider credentials nor database bootstrap authority;
- Qdrant remains private/authenticated and degradable rather than a core startup dependency;
- disabled-by-default Edge Functions have no public route.

`production-gateway-check` proves the live gateway is non-root, capability-free, can read only the staged TLS material, and owns only its intended writable state volumes. `production-ai-check` proves exclusive AI egress membership, internal gateway authentication, tenant-header requirements, and core health during AI-Gateway/provider outage. `production-qdrant-check` probes authenticated private Qdrant readiness from the application network without turning Qdrant into a core-health dependency.

Useful commands:

```bash
make production-logs
make production-migrate
make production-database-check
make production-gateway-check
make production-ai-check
make production-qdrant-check
make production-down
```

`production-down` does not delete database, Storage, Qdrant, TLS staging or Caddy data volumes. There is deliberately no convenience command that destroys production data.

## Container hardening inventory

For source/topology review, render the effective configuration and record the per-container security posture:

```bash
bash edutalent production-config > /var/lib/edutalent/operations/rendered-compose.json
python3 deploy/production/container_hardening_inventory.py \
  /var/lib/edutalent/operations/rendered-compose.json \
  --output /var/lib/edutalent/operations/container-hardening.json
```

The inventory records image identity, user/group, root filesystem mode, capabilities, writable/read-only paths, PID/resource limits, restart/health behavior, networks, published ports, and security options. At final locked-release acceptance rerun it with `--require-digests`; a tag-only release image is then a failure.

## Operations and maintenance

The operations layer is documented in `operations/README.md` and includes:

- encrypted full backups with immediate verification;
- separate restore drills;
- continuous WAL reception and PITR proof;
- local health/resource/backup/WAL/TLS monitoring and alerts;
- Qdrant snapshot recovery or explicit reindex choice;
- bounded load and restart/fault tests;
- deployment/upgrade/rollback procedures.

Checked-in hardened systemd reference units in `systemd/` schedule monitoring, daily backup, weekly restore verification and periodic WAL verification while the WAL receiver is maintained across boot. Use a dedicated unprivileged `edutalent-operator`; rootless Docker is preferred. A rootful Docker deployment requires an explicit tailored host/CIS review rather than being silently treated as equivalent.

Backups must use a separate protected filesystem/device or controlled backup-host mount. The backup passphrase must remain mode `0400`/`0600` and be escrowed separately from encrypted backup media. Encrypted backups and WAL must have an off-host copy frequency consistent with the accepted measured RPO.

## Public surfaces

- `https://APP_DOMAIN`: EduTalent;
- `https://SUPABASE_DOMAIN`: approved Auth, REST, Realtime, Storage and GraphQL prefixes;
- `https://ADMIN_DOMAIN`: Supabase administration, restricted first by source CIDR and then by generated dashboard authentication.

PostgreSQL, Supavisor, Qdrant, Studio, Auth, PostgREST, Realtime, Storage, Edge Runtime and metadata services publish no direct host ports.

## Optional Edge Functions

The pinned Supabase Edge Runtime definition remains coordinated in the upstream configuration but is assigned to the explicit `edge-functions` profile. It is not part of the default production startup and `/functions/v1` is not exposed by Caddy. Enabling it later requires a separate authentication, ingress, health, resource and release-scope review.

## Authentication policy

Public signup, anonymous users, phone signup and unauthenticated Edge Functions are disabled. The email provider remains enabled because Supabase Auth uses it for both registration and password login; `DISABLE_SIGNUP=true` independently blocks public registration while permitting administratively provisioned users to authenticate. A school-local SMTP relay may be added only under its approved local network policy.

EduTalent validates Supabase ES256 tokens against the local JWKS endpoint and explicit self-hosted issuer `https://SUPABASE_DOMAIN/auth/v1`. Mixed JWKS are parsed safely, but only the matching ES256 key is accepted for user-token validation.

## AI and embedding profiles

Production uses the server-side AI Gateway boundary. The supported profile registry includes the offline local BGE profile (`local-bge-v1`) and the approved connected profile (`openai-v1`) with fixed model/vector/collection contracts. The application never receives provider API keys or external provider base URLs. Different embedding dimensions/models use distinct versioned Qdrant collections.

External AI availability is not a core health requirement. Connected-provider loss is handled as a degraded AI condition while core local school operations continue.

## Air-gapped release

Air-gapped packaging is implemented separately from this source-mode production bootstrap. The release/appliance flow inventories required images/model artifacts and produces immutable manifest/checksum/SBOM/signature/provenance evidence plus no-pull startup validation. Target-host acceptance must use the exact frozen signed artifact and record its digest in `operations/TARGET_HOST_ACCEPTANCE.md`; source-topology success alone is not release acceptance.
