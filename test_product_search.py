#!/usr/bin/env python3
"""
Quick RAG Test - Product Matching (Rate Limit Friendly)
"""
import os
import json
import requests
import hashlib
import time

VOYAGE_API_KEY = os.environ.get('VOYAGE_API_KEY')
QDRANT_URL = os.environ.get('QDRANT_URL')
QDRANT_API_KEY = os.environ.get('QDRANT_API_KEY')

# Just 2 very different products
PRODUCTS = [
    {
        "id": "laptop-gaming",
        "title": "TechPro X500 Gaming Laptop",
        "content": """TechPro X500 Gaming Laptop - Price: $1,299.
Features: NVIDIA RTX 4060 GPU with 8GB VRAM for gaming and video editing.
32GB RAM, 1TB SSD. 15.6 inch 4K display. Weight: 2.1 kg. Battery: 8 hours.
Best for: Gamers, video editors, 3D artists who need powerful graphics."""
    },
    {
        "id": "laptop-travel", 
        "title": "TechPro UltraLight Travel Laptop",
        "content": """TechPro UltraLight Travel Laptop - Price: $899.
Features: Ultra lightweight at only 0.98 kg! 14 hours battery life.
16GB RAM, 512GB SSD. 13.3 inch Full HD. Fanless silent design.
Best for: Travelers, students, remote workers who prioritize portability."""
    }
]

def get_embedding(text, input_type="document"):
    print("   ⏳ Waiting for rate limit...")
    time.sleep(22)
    resp = requests.post(
        'https://api.voyageai.com/v1/embeddings',
        headers={
            'Authorization': f'Bearer {VOYAGE_API_KEY}',
            'Content-Type': 'application/json'
        },
        json={'model': 'voyage-3-large', 'input': [text], 'input_type': input_type}
    )
    data = resp.json()
    if 'data' not in data:
        raise Exception(f"API Error: {data}")
    return data['data'][0]['embedding']

def upsert_to_qdrant(point_id, embedding, payload):
    return requests.put(
        f'{QDRANT_URL}/collections/edutalent_materials/points',
        headers={'api-key': QDRANT_API_KEY, 'Content-Type': 'application/json'},
        json={'points': [{'id': point_id, 'vector': embedding, 'payload': payload}]}
    ).json()

def search_qdrant(embedding):
    return requests.post(
        f'{QDRANT_URL}/collections/edutalent_materials/points/search',
        headers={'api-key': QDRANT_API_KEY, 'Content-Type': 'application/json'},
        json={'vector': embedding, 'limit': 2, 'with_payload': True}
    ).json()['result']

print("=" * 60)
print("📦 UPLOADING 2 PRODUCTS")
print("=" * 60)

for product in PRODUCTS:
    print(f"\n⬆️  {product['title']}")
    embedding = get_embedding(product['content'])
    point_id = int(hashlib.md5(product['id'].encode()).hexdigest()[:15], 16)
    result = upsert_to_qdrant(point_id, embedding, {
        'chunk_text': product['content'],
        'material_title': product['title'],
        'material_id': product['id']
    })
    print(f"   ✅ Done" if result.get('status') == 'ok' else f"   ❌ {result}")

print("\n" + "=" * 60)
print("🔍 TESTING QUERIES")
print("=" * 60)

queries = [
    ("I travel a lot and need something lightweight with good battery", "UltraLight"),
    ("I want to play games and edit videos, need strong GPU", "Gaming"),
    ("Which one is cheaper and good for students?", "UltraLight"),
]

for query, expected in queries:
    print(f"\n🔍 \"{query}\"")
    print(f"   Expected: {expected}")
    
    embedding = get_embedding(query, "query")
    results = search_qdrant(embedding)
    
    top = results[0]
    title = top['payload']['material_title']
    score = top['score']
    matched = "✅ CORRECT" if expected in title else "❌ WRONG"
    
    print(f"   Got: {title} (Score: {score:.4f}) {matched}")
    print(f"   Content: {top['payload']['chunk_text'][:100]}...")

print("\n✅ TEST COMPLETE")
