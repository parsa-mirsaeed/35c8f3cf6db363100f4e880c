#!/usr/bin/env python3
"""Apply the three final controlled-AI review fixes once, fail closed on drift."""

from pathlib import Path
import re
import subprocess


def load(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def save(path: str, text: str) -> None:
    Path(path).write_text(text, encoding="utf-8")


OLD_COLLECTION = "edutalent_local_bge_v1"
LEGACY_COLLECTION = "edutalent_materials_local_v1"
collection_paths = subprocess.check_output(
    ["git", "grep", "-l", OLD_COLLECTION, "--", ":(exclude).github/workflows"],
    text=True,
).splitlines()
for path in collection_paths:
    save(path, load(path).replace(OLD_COLLECTION, LEGACY_COLLECTION))

path = "packages/api/src/services/knowledge_vector_store_service.rs"
text = load(path)
text, count = re.subn(
    r'^\s*Condition::matches\("embedding_profile", self\.profile\.id\.to_string\(\)\),\n',
    "",
    text,
    flags=re.MULTILINE,
)
if count not in (0, 3):
    raise SystemExit(f"expected zero or three knowledge profile filters, found {count}")
save(path, text)

path = "packages/api/src/services/vector_store_service.rs"
text = load(path)
old_filter = re.compile(
    r'''        let mut required = vec!\[Condition::matches\(\n'''
    r'''            "embedding_profile",\n'''
    r'''            self\.profile\.id\.to_string\(\),\n'''
    r'''        \)\];'''
)
new_filter = (
    "        // Collection and dimensions are the immutable profile boundary. Legacy\n"
    "        // points in the unchanged local collection predate this payload field.\n"
    "        let mut required = Vec::new();"
)
text, count = old_filter.subn(new_filter, text)
if count not in (0, 1):
    raise SystemExit(f"expected zero or one material profile filter, found {count}")
if count == 0 and new_filter not in text:
    raise SystemExit("material profile filter contract drifted")
save(path, text)

path = "packages/api/src/services/embedding_profile.rs"
text = load(path)
marker = "    #[test]\n    fn matching_overrides_are_accepted() {"
test = '''    #[test]
    fn local_profile_preserves_the_existing_production_collection() {
        assert_eq!(LOCAL_BGE_V1.collection, "edutalent_materials_local_v1");
        assert_ne!(LOCAL_BGE_V1.collection, OPENAI_V1.collection);
    }

'''
if test not in text:
    if text.count(marker) != 1:
        raise SystemExit("embedding profile test marker changed")
    text = text.replace(marker, test + marker, 1)
save(path, text)

path = "README.md"
text = load(path)
old = "Changing profiles requires a distinct collection and complete re-index; automatic fallback between vector spaces is forbidden."
new = "The unchanged local BGE profile deliberately retains the existing production collection name `edutalent_materials_local_v1`, so upgrades keep previously indexed local vectors available. The OpenAI profile uses its own collection. Changing either model or dimensions still requires a distinct collection and complete re-index; automatic fallback between vector spaces is forbidden."
if new not in text:
    if text.count(old) != 1:
        raise SystemExit("README profile note changed")
    text = text.replace(old, new, 1)
save(path, text)

path = "docs/adr/0002-controlled-external-ai.md"
text = load(path)
old = "A profile change requires a distinct collection and complete re-index. Automatic fallback between profiles is forbidden because it would mix vector spaces. Qdrant writes and searches validate the active collection and dimensions, and governed knowledge queries retain PostgreSQL authorization before exact Qdrant asset filtering."
new = "The unchanged local BGE profile retains the existing production collection name `edutalent_materials_local_v1` so an upgrade does not hide previously indexed vectors. Its collection and 384-dimensional contract remain the profile boundary even for legacy points that predate the `embedding_profile` payload field. A model or dimension change requires a distinct collection and complete re-index. Automatic fallback between profiles is forbidden, and governed knowledge queries retain PostgreSQL authorization before exact Qdrant asset filtering."
if new not in text:
    if text.count(old) != 1:
        raise SystemExit("ADR profile note changed")
    text = text.replace(old, new, 1)
save(path, text)

path = "edutalent"
text = load(path)
old_dev = '        exec docker compose --env-file "${ENV_FILE}" -f "${DEV_COMPOSE}" --profile dev up --build dev'
new_dev = '        exec docker compose --env-file "${ENV_FILE}" -f "${DEV_COMPOSE}" --profile dev up --build ai-gateway dev'
if old_dev in text:
    if text.count(old_dev) != 1:
        raise SystemExit("dev startup command is ambiguous")
    text = text.replace(old_dev, new_dev, 1)
elif new_dev not in text:
    raise SystemExit("dev startup command drifted")
old_app = "        compose_dev --profile app up --build --detach app"
new_app = "        compose_dev --profile app up --build --detach ai-gateway app"
if old_app in text:
    if text.count(old_app) != 2:
        raise SystemExit("app startup command is ambiguous")
    text = text.replace(old_app, new_app)
elif text.count(new_app) != 2:
    raise SystemExit("app startup commands drifted")
anchor = "        grep -Fq 'production-ai-check' edutalent"
checks = "\n        grep -Fq -- '--profile dev up --build ai-gateway dev' edutalent\n        grep -Fq -- '--profile app up --build --detach ai-gateway app' edutalent"
if checks.strip() not in text:
    if text.count(anchor) != 1:
        raise SystemExit("command validation marker changed")
    text = text.replace(anchor, anchor + checks, 1)
save(path, text)

path = "packages/api/src/ai_gateway_runtime.rs"
text = load(path)
start = text.index("async fn bounded_body(")
end = text.index("fn validate_embedding_response(", start)
current = text[start:end]
if ".chunk()" not in current:
    function = '''async fn bounded_body(
    mut response: reqwest::Response,
    maximum: usize,
) -> Result<Vec<u8>, ProviderFailure> {
    if response
        .content_length()
        .is_some_and(|length| length > maximum as u64)
    {
        return Err(ProviderFailure::ResponseTooLarge);
    }

    let initial_capacity = response
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or_default()
        .min(maximum);
    let mut body = Vec::with_capacity(initial_capacity);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ProviderFailure::InvalidResponse)?
    {
        let next_length = body
            .len()
            .checked_add(chunk.len())
            .ok_or(ProviderFailure::ResponseTooLarge)?;
        if next_length > maximum {
            return Err(ProviderFailure::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

'''
    text = text[:start] + function + text[end:]

import_line = "    use std::sync::atomic::{AtomicUsize, Ordering};"
io_import = "    use tokio::io::{AsyncReadExt, AsyncWriteExt};"
if io_import not in text:
    if text.count(import_line) != 1:
        raise SystemExit("gateway test import marker changed")
    text = text.replace(import_line, import_line + "\n" + io_import, 1)

marker = "    async fn spawn_embedding_mock("
test_name = "chunked_provider_body_stops_at_the_configured_limit"
if test_name not in text:
    if text.count(marker) != 1:
        raise SystemExit("provider mock marker changed")
    addition = '''    async fn spawn_chunked_response(chunks: Vec<&'static [u8]>) -> Url {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind chunked response");
        let address = listener.local_addr().expect("chunked response address");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut request = [0_u8; 1_024];
            let _ = socket.read(&mut request).await;
            socket
                .write_all(b"HTTP/1.1 200 OK\\r\\nTransfer-Encoding: chunked\\r\\nConnection: close\\r\\n\\r\\n")
                .await
                .expect("write response headers");
            for chunk in chunks {
                socket
                    .write_all(format!("{:X}\\r\\n", chunk.len()).as_bytes())
                    .await
                    .expect("write chunk size");
                socket.write_all(chunk).await.expect("write chunk");
                socket.write_all(b"\\r\\n").await.expect("finish chunk");
            }
            socket.write_all(b"0\\r\\n\\r\\n").await.expect("finish response");
        });
        Url::parse(&format!("http://{address}/")).expect("chunked response URL")
    }

    #[tokio::test]
    async fn chunked_provider_body_stops_at_the_configured_limit() {
        let response = reqwest::get(
            spawn_chunked_response(vec![&b"abcd"[..], &b"efgh"[..]]).await,
        )
        .await
        .expect("chunked response");
        assert!(matches!(
            bounded_body(response, 7).await,
            Err(ProviderFailure::ResponseTooLarge)
        ));
    }

'''
    text = text.replace(marker, addition + marker, 1)
save(path, text)
