# EduTalent

EduTalent is a Rust/Dioxus full-stack application with PostgreSQL, Qdrant, and a durable knowledge-ingestion worker.

The repository has one command surface for local development, production-like execution, Docker packaging, and GitHub packaging:

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

The checked-in environment template contains placeholder Supabase credentials. The health endpoint and repository-owned services can run with the template, but login and user provisioning require real Supabase values in `.env`.

## Unified commands

| Command | Purpose |
| --- | --- |
| `make dev` | Complete hot-reload development stack |
| `make up` | Production-like stack using the final runtime image |
| `make down` | Stop the stack |
| `make logs` | Follow app logs |
| `make ps` | Show stack status |
| `make migrate` | Apply all canonical and incremental migrations transactionally |
| `make build` | Build `edutalent:local` |
| `make package` | Create a source-free image bundle under `dist/` |
| `make smoke` | Start the packaged stack and verify `/healthz` |
| `make validate` | Validate shell and Compose packaging definitions |
| `make clean` | Stop the stack and remove local database/vector/model volumes |

Pass a version through `ARGS`:

```bash
make package ARGS=v0.4.0
```

The same interface is available without Make:

```bash
bash edutalent dev
bash edutalent package v0.4.0
```

## What runs locally

`compose.yaml` defines two application modes over the same dependencies:

- `dev`: source-mounted Dioxus hot reload;
- `app`: the same final runtime image used for packaging and deployment.

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

## Release bundle

```bash
make package ARGS=v0.4.0
```

This creates:

```text
dist/edutalent-v0.4.0.tar.gz
```

The archive contains a compressed Docker image, a source-free release Compose file, an environment template, and a SHA-256 checksum.

To run the bundle on another Docker host:

```bash
tar -xzf edutalent-v0.4.0.tar.gz
cd edutalent-v0.4.0
cp .env.example .env
# Set production Supabase credentials and replace local-development passwords.
gzip -dc edutalent-image.tar.gz | docker load
docker compose up -d
```

The release Compose file uses only images; the repository source is not required on the target host.

## Container build

The multi-stage `Dockerfile` performs the entire build from source:

1. installs Dioxus CLI `0.7.2` and the WASM target;
2. runs `dx bundle --web --release --package web`;
3. copies the Dioxus `server` executable and public assets into a slim runtime;
4. packages all migrations and the canonical migration runner;
5. exposes `/healthz` and starts the durable knowledge worker with the web server.

The old manually committed `bin-build` path is no longer used. `build-for-render.sh` remains only as a compatibility wrapper around the unified command.

## GitHub packages and artifacts

The packaging workflow validates the definitions, builds the same runtime image through `bash edutalent package`, and uploads the release archive as a workflow artifact. Tag builds can also publish the image to GitHub Container Registry.

No production secrets are stored in the repository or release bundle.

## Configuration boundaries

Repository-owned local services are centralized, but production still needs environment-specific values for:

- Supabase authentication and administration;
- production PostgreSQL connection and RLS role model;
- managed Qdrant when not using the bundled service;
- managed or private OpenAI-compatible embedding service when not using the bundled CPU service;
- storage, malware scanning, retention, and signed URLs for governed manager-submitted PDFs.

The packaging work does not reintroduce teacher PDF uploads, bypass the durable ingestion queue, or weaken PostgreSQL and Qdrant retrieval authorization.
# sync test Fri Jul 17 11:38:27 PM +0330 2026
