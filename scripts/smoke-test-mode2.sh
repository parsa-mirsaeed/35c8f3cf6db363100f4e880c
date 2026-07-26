#!/usr/bin/env bash
set -euo pipefail

APP_URL="${APP_URL:-http://localhost:10000}"
QDRANT_URL="${QDRANT_URL:-http://localhost:6333}"
EMBEDDING_URL="${EMBEDDING_URL:-http://localhost:8080}"
EMBEDDING_MODEL="${EMBEDDING_MODEL:-BAAI/bge-small-en-v1.5}"
EXPECTED_VECTOR_SIZE="${EXPECTED_VECTOR_SIZE:-384}"

echo "Checking EduTalent app at ${APP_URL}/healthz"
curl --fail --silent --show-error "${APP_URL}/healthz" >/dev/null

echo "Checking Qdrant collections at ${QDRANT_URL}/collections"
curl --fail --silent --show-error "${QDRANT_URL}/collections" >/dev/null

echo "Checking local embedding endpoint at ${EMBEDDING_URL}/v1/embeddings"
response="$(curl --fail --silent --show-error \
  -H 'Content-Type: application/json' \
  -d "{\"model\":\"${EMBEDDING_MODEL}\",\"input\":[\"EduTalent private embedding smoke test\"]}" \
  "${EMBEDDING_URL}/v1/embeddings")"

EMBEDDING_RESPONSE="$response" python3 - "$EXPECTED_VECTOR_SIZE" <<'PY'
import json
import os
import sys

expected = int(sys.argv[1])
payload = json.loads(os.environ["EMBEDDING_RESPONSE"])
vector = payload["data"][0]["embedding"]
actual = len(vector)
if actual != expected:
    raise SystemExit(f"Expected embedding size {expected}, got {actual}")
print(f"Embedding vector size OK: {actual}")
PY

echo "Mode 2 smoke test passed."
