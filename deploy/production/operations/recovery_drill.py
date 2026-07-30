#!/usr/bin/env python3
"""Run isolated PostgreSQL PITR, rollback, and Qdrant snapshot recovery drills."""

from __future__ import annotations

import argparse
import json
import os
import secrets
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


def run(*args: str, capture: bool = False, check: bool = True, env: dict[str, str] | None = None) -> str:
    completed = subprocess.run(
        args,
        check=check,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.STDOUT if capture else None,
        env=env,
    )
    return completed.stdout.strip() if capture and completed.stdout else ""


def docker(*args: str, **kwargs: Any) -> str:
    return run("docker", *args, **kwargs)


def wait_postgres(container: str, password: str, timeout: float = 90.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            ["docker", "exec", "-e", f"PGPASSWORD={password}", container, "pg_isready", "-h", "127.0.0.1", "-U", "postgres"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if result.returncode == 0:
            return
        time.sleep(1)
    raise RuntimeError(f"PostgreSQL did not become ready: {container}")


def psql(container: str, password: str, sql: str, database: str = "postgres", *, check: bool = True) -> str:
    return docker(
        "exec",
        "-e",
        f"PGPASSWORD={password}",
        container,
        "psql",
        "-h",
        "127.0.0.1",
        "-U",
        "postgres",
        "-d",
        database,
        "-v",
        "ON_ERROR_STOP=1",
        "-At",
        "-c",
        sql,
        capture=True,
        check=check,
    )


def wait_for_archive(volume: str, minimum_files: int, timeout: float = 60.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        count = int(
            docker(
                "run",
                "--rm",
                "-v",
                f"{volume}:/archive:ro",
                "alpine:3.22.1",
                "sh",
                "-c",
                "find /archive -maxdepth 1 -type f | wc -l",
                capture=True,
            )
        )
        if count >= minimum_files:
            return
        time.sleep(1)
    raise RuntimeError("PostgreSQL WAL archive did not receive the required segments")


def postgres_drill(prefix: str) -> dict[str, Any]:
    source = f"{prefix}-pg-source"
    restored = f"{prefix}-pg-restored"
    data_volume = f"{prefix}-pg-source-data"
    base_volume = f"{prefix}-pg-base"
    archive_volume = f"{prefix}-pg-archive"
    password = secrets.token_urlsafe(24)
    image = "postgres:17-alpine"
    for volume in (data_volume, base_volume, archive_volume):
        docker("volume", "create", volume, capture=True)

    docker(
        "run",
        "--detach",
        "--name",
        source,
        "-e",
        f"POSTGRES_PASSWORD={password}",
        "-v",
        f"{data_volume}:/var/lib/postgresql/data",
        "-v",
        f"{base_volume}:/backup",
        "-v",
        f"{archive_volume}:/archive",
        image,
        "postgres",
        "-c",
        "wal_level=replica",
        "-c",
        "archive_mode=on",
        "-c",
        "archive_timeout=2s",
        "-c",
        "archive_command=test ! -f /archive/%f && cp %p /archive/%f",
        capture=True,
    )
    wait_postgres(source, password)
    docker("exec", "--user", "0", source, "chown", "-R", "postgres:postgres", "/backup", "/archive")
    psql(source, password, "CREATE TABLE recovery_probe(id integer PRIMARY KEY, marker text NOT NULL); INSERT INTO recovery_probe VALUES (1, 'base');")
    docker(
        "exec",
        "-e",
        f"PGPASSWORD={password}",
        "--user",
        "postgres",
        source,
        "pg_basebackup",
        "-h",
        "127.0.0.1",
        "-U",
        "postgres",
        "-D",
        "/backup",
        "--format=plain",
        "--wal-method=none",
        "--checkpoint=fast",
        "--no-password",
    )
    psql(source, password, "INSERT INTO recovery_probe VALUES (2, 'before-target');")
    target_time = psql(source, password, "SELECT clock_timestamp();")
    psql(source, password, "SELECT pg_switch_wal();")
    wait_for_archive(archive_volume, 1)
    time.sleep(1.2)
    psql(source, password, "INSERT INTO recovery_probe VALUES (3, 'after-target');")
    psql(source, password, "SELECT pg_switch_wal();")
    wait_for_archive(archive_volume, 2)
    docker("stop", source, capture=True)

    config = (
        "restore_command = 'cp /archive/%f %p'\n"
        f"recovery_target_time = '{target_time}'\n"
        "recovery_target_action = 'promote'\n"
    )
    docker(
        "run",
        "--rm",
        "--user",
        "0",
        "-v",
        f"{base_volume}:/restore",
        "-v",
        f"{archive_volume}:/archive:ro",
        "--entrypoint",
        "sh",
        image,
        "-eu",
        "-c",
        f"printf %s {json.dumps(config)} >> /restore/postgresql.auto.conf; touch /restore/recovery.signal; chown -R postgres:postgres /restore",
    )
    docker(
        "run",
        "--detach",
        "--name",
        restored,
        "-e",
        f"POSTGRES_PASSWORD={password}",
        "-v",
        f"{base_volume}:/var/lib/postgresql/data",
        "-v",
        f"{archive_volume}:/archive:ro",
        image,
        capture=True,
    )
    wait_postgres(restored, password)
    deadline = time.monotonic() + 90
    rows = ""
    while time.monotonic() < deadline:
        rows = psql(restored, password, "SELECT string_agg(id::text || ':' || marker, ',' ORDER BY id) FROM recovery_probe;", check=False)
        if rows:
            break
        time.sleep(1)
    if rows != "1:base,2:before-target":
        raise RuntimeError(f"PITR target mismatch: {rows!r}")

    failed = subprocess.run(
        [
            "docker",
            "exec",
            "-e",
            f"PGPASSWORD={password}",
            restored,
            "psql",
            "-h",
            "127.0.0.1",
            "-U",
            "postgres",
            "-d",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            "BEGIN; CREATE TABLE failed_migration_probe(id integer); INSERT INTO failed_migration_probe VALUES (1); SELECT 1/0; COMMIT;",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if failed.returncode == 0:
        raise RuntimeError("deliberately failed migration unexpectedly succeeded")
    rollback_state = psql(restored, password, "SELECT to_regclass('public.failed_migration_probe') IS NULL;")
    if rollback_state != "t":
        raise RuntimeError("failed migration left a partial table behind")
    return {"target_time": target_time, "restored_rows": rows, "failed_migration_rolled_back": True}


def http_json(url: str, method: str = "GET", payload: dict[str, Any] | None = None, headers: dict[str, str] | None = None) -> dict[str, Any]:
    body = json.dumps(payload).encode() if payload is not None else None
    request_headers = {"Content-Type": "application/json"} if body is not None else {}
    request_headers.update(headers or {})
    request = urllib.request.Request(url, data=body, method=method, headers=request_headers)
    with urllib.request.urlopen(request, timeout=15) as response:
        return json.loads(response.read().decode())


def wait_http(url: str, timeout: float = 90.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=2) as response:
                if response.status < 500:
                    return
        except (OSError, urllib.error.URLError):
            pass
        time.sleep(1)
    raise RuntimeError(f"HTTP service did not become ready: {url}")


def upload_snapshot(url: str, snapshot: bytes) -> dict[str, Any]:
    boundary = f"----edutalent{secrets.token_hex(12)}"
    body = (
        f"--{boundary}\r\nContent-Disposition: form-data; name=\"snapshot\"; filename=\"snapshot.snapshot\"\r\n"
        "Content-Type: application/octet-stream\r\n\r\n"
    ).encode() + snapshot + f"\r\n--{boundary}--\r\n".encode()
    request = urllib.request.Request(
        url,
        data=body,
        method="POST",
        headers={"Content-Type": f"multipart/form-data; boundary={boundary}", "Content-Length": str(len(body))},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.loads(response.read().decode())


def qdrant_drill(prefix: str) -> dict[str, Any]:
    container = f"{prefix}-qdrant"
    image = "qdrant/qdrant:v1.18.2"
    docker("run", "--detach", "--name", container, "-p", "127.0.0.1::6333", image, capture=True)
    port = docker("port", container, "6333/tcp", capture=True).rsplit(":", 1)[1]
    base = f"http://127.0.0.1:{port}"
    wait_http(f"{base}/readyz")
    collection = "recovery_drill"
    http_json(f"{base}/collections/{collection}", "PUT", {"vectors": {"size": 4, "distance": "Cosine"}})
    http_json(
        f"{base}/collections/{collection}/points?wait=true",
        "PUT",
        {"points": [{"id": 1, "vector": [0.1, 0.2, 0.3, 0.4], "payload": {"school_id": "00000000-0000-4000-8000-000000000001", "marker": "restore"}}]},
    )
    snapshot_name = http_json(f"{base}/collections/{collection}/snapshots", "POST")["result"]["name"]
    with urllib.request.urlopen(f"{base}/collections/{collection}/snapshots/{snapshot_name}", timeout=30) as response:
        snapshot = response.read()
    http_json(f"{base}/collections/{collection}", "DELETE")
    upload_snapshot(f"{base}/collections/{collection}/snapshots/upload?priority=snapshot", snapshot)
    point = http_json(f"{base}/collections/{collection}/points/1")["result"]
    if point.get("payload", {}).get("marker") != "restore":
        raise RuntimeError("Qdrant restored point payload mismatch")
    return {"snapshot_name": snapshot_name, "restored_point_id": point["id"]}


def cleanup(prefix: str) -> None:
    for suffix in ("pg-source", "pg-restored", "qdrant"):
        subprocess.run(["docker", "rm", "--force", f"{prefix}-{suffix}"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for suffix in ("pg-source-data", "pg-base", "pg-archive"):
        subprocess.run(["docker", "volume", "rm", "--force", f"{prefix}-{suffix}"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    prefix = f"edutalent-drill-{secrets.token_hex(4)}"
    started = time.monotonic()
    try:
        result = {
            "schema_version": 1,
            "postgres": postgres_drill(prefix),
            "qdrant": qdrant_drill(prefix),
            "duration_seconds": time.monotonic() - started,
        }
        args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    finally:
        cleanup(prefix)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError, urllib.error.URLError, json.JSONDecodeError) as error:
        print(f"recovery drill failed: {error}", file=sys.stderr)
        raise SystemExit(1)
