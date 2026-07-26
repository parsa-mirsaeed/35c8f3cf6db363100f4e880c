#!/usr/bin/env python3
"""Test semantic search and show retrieved content"""
import os
import json
import requests

# Load env vars
VOYAGE_API_KEY = os.environ.get('VOYAGE_API_KEY')
QDRANT_URL = os.environ.get('QDRANT_URL')
QDRANT_API_KEY = os.environ.get('QDRANT_API_KEY')

def get_query_embedding(query):
    resp = requests.post(
        'https://api.voyageai.com/v1/embeddings',
        headers={
            'Authorization': f'Bearer {VOYAGE_API_KEY}',
            'Content-Type': 'application/json'
        },
        json={
            'model': 'voyage-3-large',
            'input': [query],
            'input_type': 'query'
        }
    )
    return resp.json()['data'][0]['embedding']

def search_qdrant(embedding, limit=3):
    resp = requests.post(
        f'{QDRANT_URL}/collections/edutalent_materials/points/search',
        headers={
            'api-key': QDRANT_API_KEY,
            'Content-Type': 'application/json'
        },
        json={
            'vector': embedding,
            'limit': limit,
            'with_payload': True
        }
    )
    return resp.json()['result']

# Test query
query = "How do I balance a chemical equation?"
print(f"🔍 Query: '{query}'")
print("=" * 60)

# Get embedding and search
embedding = get_query_embedding(query)
results = search_qdrant(embedding)

for i, result in enumerate(results, 1):
    payload = result['payload']
    print(f"\n📄 Result {i} (Score: {result['score']:.4f})")
    print(f"   Title: {payload['material_title']}")
    print(f"   Chunk: {payload['chunk_index']}")
    print(f"\n   Content Retrieved:")
    print(f"   {'-' * 50}")
    # Show the actual content
    content = payload['chunk_text']
    print(f"   {content[:500]}...")
    if len(content) > 500:
        print(f"   ... ({len(content)} total chars)")
