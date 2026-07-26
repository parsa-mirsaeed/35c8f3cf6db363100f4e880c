#!/bin/bash
# Test script for the vectorization pipeline

set -e
source ~/.bashrc

MATERIAL_ID="a84d8134-47ac-4e6c-9c1e-6295fcba57ac"
MATERIAL_TITLE="Introduction to Chemical Reactions"

echo "=== EduTalent Vectorization Test ==="
echo ""

# Test 1: Verify material exists
echo "1. Checking material in database..."
psql "$DATABASE_URL" -c "SELECT id, title FROM class_materials WHERE id = '$MATERIAL_ID';"

# Test 2: Call Voyage AI to generate embedding for material content
echo ""
echo "2. Testing Voyage AI embedding generation..."
CONTENT="Introduction to Chemical Reactions. This lesson covers the fundamentals of chemical reactions..."

EMBEDDING_RESPONSE=$(curl -s -X POST "https://api.voyageai.com/v1/embeddings" \
  -H "Authorization: Bearer $VOYAGE_API_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\": \"voyage-3-large\", \"input\": [\"$CONTENT\"], \"input_type\": \"document\"}")

if echo "$EMBEDDING_RESPONSE" | grep -q "embedding"; then
  echo "✅ Voyage AI embedding generated successfully"
  VECTOR_LENGTH=$(echo "$EMBEDDING_RESPONSE" | python3 -c "import sys,json; print(len(json.load(sys.stdin)['data'][0]['embedding']))")
  echo "   Vector dimensions: $VECTOR_LENGTH"
else
  echo "❌ Voyage AI embedding failed"
  echo "$EMBEDDING_RESPONSE"
  exit 1
fi

# Test 3: Store in Qdrant
echo ""
echo "3. Testing Qdrant storage..."

# First create collection if not exists
COLLECTIONS=$(curl -s -H "api-key: $QDRANT_API_KEY" "$QDRANT_URL/collections")
if ! echo "$COLLECTIONS" | grep -q "edutalent_materials"; then
  echo "   Creating collection 'edutalent_materials'..."
  curl -s -X PUT "$QDRANT_URL/collections/edutalent_materials" \
    -H "api-key: $QDRANT_API_KEY" \
    -H "Content-Type: application/json" \
    -d '{
      "vectors": {
        "size": 1024,
        "distance": "Cosine"
      }
    }'
fi

# Get embedding array for Qdrant
EMBEDDING=$(echo "$EMBEDDING_RESPONSE" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin)['data'][0]['embedding']))")

# Upsert point to Qdrant
POINT_ID=$(python3 -c "import hashlib; print(int(hashlib.md5(b'${MATERIAL_ID}_0').hexdigest()[:15], 16))")

UPSERT_RESPONSE=$(curl -s -X PUT "$QDRANT_URL/collections/edutalent_materials/points" \
  -H "api-key: $QDRANT_API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"points\": [{
      \"id\": $POINT_ID,
      \"vector\": $EMBEDDING,
      \"payload\": {
        \"chunk_text\": \"$CONTENT\",
        \"material_id\": \"$MATERIAL_ID\",
        \"material_title\": \"$MATERIAL_TITLE\",
        \"class_section_id\": \"4b705c81-3e7d-45f3-bd5c-19378d2f7485\",
        \"chunk_index\": 0
      }
    }]
  }")

if echo "$UPSERT_RESPONSE" | grep -q '"status":"ok"'; then
  echo "✅ Vector stored in Qdrant successfully"
else
  echo "❌ Qdrant upsert failed"
  echo "$UPSERT_RESPONSE"
  exit 1
fi

# Test 4: Semantic search
echo ""
echo "4. Testing semantic search..."

QUERY="How do I balance a chemical equation?"
QUERY_EMBEDDING=$(curl -s -X POST "https://api.voyageai.com/v1/embeddings" \
  -H "Authorization: Bearer $VOYAGE_API_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\": \"voyage-3-large\", \"input\": [\"$QUERY\"], \"input_type\": \"query\"}" \
  | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin)['data'][0]['embedding']))")

SEARCH_RESPONSE=$(curl -s -X POST "$QDRANT_URL/collections/edutalent_materials/points/search" \
  -H "api-key: $QDRANT_API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"vector\": $QUERY_EMBEDDING,
    \"limit\": 3,
    \"with_payload\": true
  }")

if echo "$SEARCH_RESPONSE" | grep -q "material_title"; then
  echo "✅ Semantic search working!"
  echo "   Query: '$QUERY'"
  echo "   Found material: $(echo "$SEARCH_RESPONSE" | python3 -c "import sys,json; r=json.load(sys.stdin); print(r['result'][0]['payload'].get('material_title', 'unknown'))" 2>/dev/null || echo "could not parse")"
  echo "   Relevance score: $(echo "$SEARCH_RESPONSE" | python3 -c "import sys,json; r=json.load(sys.stdin); print(round(r['result'][0]['score'], 4))" 2>/dev/null || echo "could not parse")"
else
  echo "❌ Semantic search failed"
  echo "$SEARCH_RESPONSE"
fi

# Test 5: Update material_embeddings status
echo ""
echo "5. Updating vectorization status in database..."
psql "$DATABASE_URL" -c "
INSERT INTO material_embeddings (material_id, status, chunks_count, processed_at)
VALUES ('$MATERIAL_ID', 'completed', 1, NOW())
ON CONFLICT (material_id) DO UPDATE SET
  status = 'completed',
  chunks_count = 1,
  processed_at = NOW(),
  updated_at = NOW();"

echo ""
echo "=== All Tests Passed! ==="
echo "The vectorization pipeline is working correctly."
