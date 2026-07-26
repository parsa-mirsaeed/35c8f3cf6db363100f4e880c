# EduTalent Mode 2 — Local/private app + Qdrant + local embeddings, no local LLM

Mode 2 keeps semantic search and recommendation vectors inside the private deployment while avoiding a local LLM runtime. The application talks to:

1. the EduTalent app container;
2. a private Qdrant vector database;
3. a local OpenAI-compatible embedding service powered by Hugging Face Text Embeddings Inference.

The current application still requires its existing PostgreSQL/Supabase Auth configuration. This mode removes hosted embedding calls and avoids configuring a hosted or local LLM API key.

## Minimum rental target

For a small school demo or first production pilot, start with:

- **2 vCPU / 4 GB RAM / 40 GB SSD** for the app, Qdrant, and CPU embeddings on one VM.
- **4 vCPU / 8 GB RAM / 80 GB SSD** if multiple schools will test at the same time or you expect bulk material uploads.

Use the `BAAI/bge-small-en-v1.5` model first because it has a 384-dimensional vector size and runs acceptably on CPU.

## Workflow

```bash
# 1. Build and tag the app image from the repository root.
docker build -t edutalent:mode2 .

# 2. Prepare private runtime configuration.
cp deployments/mode2-local/.env.example deployments/mode2-local/.env
$EDITOR deployments/mode2-local/.env

# 3. Start the private stack.
docker compose --env-file deployments/mode2-local/.env \
  -f deployments/mode2-local/docker-compose.yml up -d

# 4. Smoke test the workflow.
APP_URL=http://localhost:10000 \
QDRANT_URL=http://localhost:6333 \
EMBEDDING_URL=http://localhost:8080 \
./scripts/smoke-test-mode2.sh
```

## Configuration contract

The application must run with these values in Mode 2:

- `EMBEDDING_PROVIDER=local`
- `EMBEDDING_BASE_URL=http://embedding:8080/v1`
- `EMBEDDING_MODEL=BAAI/bge-small-en-v1.5`
- `EMBEDDING_VECTOR_SIZE=384`
- `QDRANT_URL=http://qdrant:6334`
- `QDRANT_VECTOR_SIZE=384`
- no `DEEPSEEK_API_KEY`

If you change the embedding model, update both `EMBEDDING_VECTOR_SIZE` and `QDRANT_VECTOR_SIZE` to match the model output dimensions before creating a Qdrant collection.

## Expected response time

On the minimum 2 vCPU / 4 GB RAM VM, expect:

- app page/API responses: usually sub-second after the app is warm;
- one short embedding request: usually 200 ms to 1.5 s on CPU;
- first request after container start: slower because the embedding model loads into memory;
- bulk vectorization: minutes for large document sets, depending on file size and CPU.

## Troubleshooting

- If Qdrant rejects vectors, check that `EMBEDDING_VECTOR_SIZE` and `QDRANT_VECTOR_SIZE` match the embedding model.
- If embedding calls fail, check `docker compose logs embedding` and confirm `/v1/embeddings` is reachable.
- If the app cannot authenticate users, verify the existing Supabase/Auth database variables. Auth migration is a separate workstream from Mode 2 vectors.
