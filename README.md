# EduTalent

EduTalent is a Rust/Dioxus full-stack application with PostgreSQL, Qdrant, and a durable knowledge-ingestion worker.

The repository has one command surface for local development, packaging, and the self-hosted production foundation:

```bash
make help
```

## Quick start

Requirements: Docker Engine with Docker Compose v2 and GNU Make.

```bash
make init
make dev
```

`make init` creates `.env` from `.env.example`. `make dev` builds and starts PostgreSQL, Qdrant, a local OpenAI-compatible Text Embeddings Inference service, applies migrations, and runs the Dioxus app with hot reload.

Open `http://localhost:8080`.

The development environment template contains placeholder Supabase credentials. The health endpoint and repository-owned services can run with the template, but login and user provisioning require valid values.

## Unified commands

| Command | Purpose |
| --- | --- |
| `make dev` | Complete hot-reload development stack |
| `make up` | Lightweight production-like stack using the final runtime image |
| `make down` | Stop the lightweight stack |
| `make logs` | Follow app logs |
| `make ps` | Show stack status |
| `make migrate` | Apply all canonical and incremental migrations transactionally |
| `make build` | Build `edutalent:local` |
| `make package` | Create a source-free application-image bundle under `dist/` |
| `make smoke` | Start the lightweight stack and verify `/healthz` |
| `make validate` | Validate shell and Compose definitions |
| `make clean` | Stop the lightweight stack and remove its local volumes |
| `make production-bootstrap` | Materialize the exact pinned official Supabase Docker runtime |
| `make production-init` | Generate production secrets after domains and TLS paths are configured |
| `make production-validate` | Verify TLS, secrets, topology, and exposure invariants |
| `make production-up` | Start the self-hosted production stack |
| `make production-down` | Stop production without deleting data |
| `make production-logs` | Follow production logs |
| `make production-ps` | Show production service state |
| `make production-migrate` | Re-run migrations and backend-role configuration |
| `make production-database-check` | Verify the app uses the constrained non-superuser backend role |
| `make production-gateway-check` | Verify the TLS gateway is non-root, capability-free, and using the staged mode-600 key |
| `make production-qdrant-check` | Verify authenticated Qdrant readiness over the private app network |

Pass a version through `ARGS`:

```bash
make package ARGS=v0.4.0
```

The same interface is available without Make:

```bash
bash edutalent dev
bash edutalent package v0.4.0
bash edutalent production-validate
```

## Development stack

`compose.yaml` defines two application modes over the same lightweight dependencies:

- `dev`: source-mounted Dioxus hot reload;
- `app`: the same final runtime image used for application packaging.

Both modes use:

- PostgreSQL 16;
- Qdrant;
- Hugging Face Text Embeddings Inference using `BAAI/bge-small-en-v1.5` and 384-dimensional vectors;
- the canonical migration runner in `scripts/ci/apply_migrations.sh`.

On ARM64, override the embedding image in `.env`:

```dotenv
TEI_IMAGE=ghcr.io/huggingface/text-embeddings-inference:cpu-arm64-1.9
```

Do not run `dev` and `app` simultaneously because both publish the application on the configured `EDUTALENT_PORT`.

## Self-hosted production foundation

Production does **not** use the lightweight standalone PostgreSQL/Supabase arrangement. It uses:

- the complete official self-hosted Supabase Docker topology pinned to an immutable upstream commit;
- Supabase PostgreSQL as the single authoritative database;
- a generated non-superuser backend database role for the long-running app, separated from the migration/bootstrap administrator;
- private Qdrant with API authentication;
- separate internal API, data, and administration networks;
- operator-supplied static TLS staged into a Docker-managed volume;
- a numeric non-root Caddy gateway with zero effective Linux capabilities;
- host ports 80/443 mapped to unprivileged container ports 8080/8443;
- generated asymmetric Supabase signing keys and opaque API keys;
- disabled public signup, anonymous/phone auth, cloud SMTP defaults, Studio AI, and default Edge Functions startup/ingress.

Start with:

```bash
make production-bootstrap
make production-init
# Edit deploy/production/.env.edutalent when prompted, then run init again.
make production-init
make production-validate
make production-up
make production-database-check
make production-gateway-check
make production-qdrant-check
```

The database check proves the app is not connected as `postgres`, cannot create roles/databases/schema objects or modify migration integrity state, and uses only the documented backend authority. That role intentionally has `BYPASSRLS` because the current Rust repository layer performs server-side authorization without transaction-local PostgreSQL request context; replacing it with a request-scoped `NOBYPASSRLS` role is tracked in issue #8. Supabase client roles remain governed by RLS.

The gateway check proves the long-running proxy is non-root, has no effective capabilities, and reads a mode-600 private key owned by its configured numeric UID/GID. Qdrant readiness is verified separately because temporary vector-service unavailability must not prevent the core school platform and authentication services from starting. Durable ingestion jobs remain retryable.

See [`deploy/production/README.md`](deploy/production/README.md), the [architecture decision](docs/adr/0001-offline-first-production-architecture.md), and the [production threat model](docs/security/production-threat-model.md).

The external OpenAI embedding and LLM gateway is intentionally a separate security-focused change. The current local TEI service remains only as an explicit optional production profile.

## Application-image release bundle

```bash
make package ARGS=v0.4.0
```

This creates:

```text
dist/edutalent-v0.4.0.tar.gz
```

The archive contains the EduTalent Docker image, a lightweight release Compose file, an environment template, and a SHA-256 checksum. It is **not yet** the complete air-gapped production appliance. The full offline release will additionally package every Supabase/Qdrant/gateway image, optional model artifacts, digest manifests, SBOMs, signatures, and provenance.

## Container build

The multi-stage `Dockerfile` performs the entire application build from source:

1. installs Dioxus CLI `0.7.2` and the WASM target;
2. runs `dx bundle --web --release --package web`;
3. copies the Dioxus `server` executable and public assets into a slim runtime;
4. packages migrations plus the migration and database-role configuration runners;
5. exposes `/healthz` and starts the durable knowledge worker with the web server.

The old manually committed `bin-build` path is no longer used. `build-for-render.sh` remains only as a compatibility wrapper around the unified command.

## GitHub packages and artifacts

The packaging workflow validates the definitions, builds the same runtime image through `bash edutalent package`, and uploads the release archive as a workflow artifact. Tag builds can publish the application image to GitHub Container Registry.

No production secrets are stored in the repository or release bundle.

## Security invariants

Production packaging and deployment must not:

- reintroduce teacher PDF uploads;
- bypass the durable ingestion queue;
- weaken PostgreSQL/RLS or application authorization;
- retrieve from Qdrant before database authorization;
- broaden exact authorized asset filters;
- expose unpublished or archived materials;
- place Supabase secret keys, database credentials, Qdrant keys, or future AI credentials in browser code;
- expose PostgreSQL, Supavisor, Qdrant, Studio, or internal Supabase services directly to the host network.
