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
from dataclasses import dataclass
from pathlib import Path
from typing import Any

PRODUCTION_DIR = Path(__file__).resolve().parent.parent
REPOSITORY_ROOT = PRODUCTION_DIR.parent.parent
SUPABASE_RUNTIME_DIR = PRODUCTION_DIR / "runtime" / "supabase"
SUPABASE_PIN = PRODUCTION_DIR / "SUPABASE_UPSTREAM"
PRODUCTION_PG_HBA = PRODUCTION_DIR / "pg_hba.conf"


@dataclass(frozen=True)
class SupabasePostgresRuntime:
    image: str
    upstream_commit: str
    runtime_dir: Path
    pg_hba: Path
    init_mounts: tuple[tuple[Path, str], ...]


def run(
    *args: str,
    capture: bool = False,
    check: bool = True,
    env: dict[str, str] | None = None,
) -> str:
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


def parse_compose_service_image(compose_path: Path, service: str) -> str:
    header = f"  {service}:"
    in_service = False
    for raw_line in compose_path.read_text(encoding="utf-8").splitlines():
        if raw_line == header:
            in_service = True
            continue
        if in_service and raw_line.startswith("  ") and not raw_line.startswith("    "):
            break
        if in_service and raw_line.startswith("    image:"):
            image = raw_line.split(":", 1)[1].strip().strip("'\"")
            if image:
                return image
    raise RuntimeError(f"materialized Supabase Compose is missing {service}.image")


def materialize_pinned_supabase_runtime() -> SupabasePostgresRuntime:
    run("bash", str(REPOSITORY_ROOT / "edutalent"), "production-bootstrap")
    compose_path = SUPABASE_RUNTIME_DIR / "docker-compose.yml"
    upstream_marker = SUPABASE_RUNTIME_DIR / "UPSTREAM_COMMIT"
    if not compose_path.is_file() or not upstream_marker.is_file():
        raise RuntimeError("pinned Supabase runtime was not materialized")

    expected_commit = SUPABASE_PIN.read_text(encoding="utf-8").strip()
    actual_commit = upstream_marker.read_text(encoding="utf-8").strip()
    if actual_commit != expected_commit:
        raise RuntimeError(
            f"materialized Supabase commit mismatch: {actual_commit} != {expected_commit}"
        )
    if len(actual_commit) != 40 or any(
        character not in "0123456789abcdef" for character in actual_commit
    ):
        raise RuntimeError(f"invalid pinned Supabase commit: {actual_commit}")

    image = parse_compose_service_image(compose_path, "db")
    if not image.startswith("supabase/postgres:") or image.endswith(":latest") or "${" in image:
        raise RuntimeError(f"Supabase database image is not an exact version: {image}")

    init_mounts = (
        (
            SUPABASE_RUNTIME_DIR / "volumes/db/realtime.sql",
            "/docker-entrypoint-initdb.d/migrations/99-realtime.sql",
        ),
        (
            SUPABASE_RUNTIME_DIR / "volumes/db/webhooks.sql",
            "/docker-entrypoint-initdb.d/init-scripts/98-webhooks.sql",
        ),
        (
            SUPABASE_RUNTIME_DIR / "volumes/db/roles.sql",
            "/docker-entrypoint-initdb.d/init-scripts/99-roles.sql",
        ),
        (
            SUPABASE_RUNTIME_DIR / "volumes/db/jwt.sql",
            "/docker-entrypoint-initdb.d/init-scripts/99-jwt.sql",
        ),
        (
            SUPABASE_RUNTIME_DIR / "volumes/db/_supabase.sql",
            "/docker-entrypoint-initdb.d/migrations/97-_supabase.sql",
        ),
        (
            SUPABASE_RUNTIME_DIR / "volumes/db/logs.sql",
            "/docker-entrypoint-initdb.d/migrations/99-logs.sql",
        ),
        (
            SUPABASE_RUNTIME_DIR / "volumes/db/pooler.sql",
            "/docker-entrypoint-initdb.d/migrations/99-pooler.sql",
        ),
    )
    missing = [str(source) for source, _ in init_mounts if not source.is_file()]
    if missing:
        raise RuntimeError(
            f"materialized Supabase database initialization is incomplete: {missing}"
        )
    if not PRODUCTION_PG_HBA.is_file():
        raise RuntimeError(
            f"production PostgreSQL HBA policy is missing: {PRODUCTION_PG_HBA}"
        )

    return SupabasePostgresRuntime(
        image=image,
        upstream_commit=actual_commit,
        runtime_dir=SUPABASE_RUNTIME_DIR,
        pg_hba=PRODUCTION_PG_HBA.resolve(),
        init_mounts=tuple((source.resolve(), target) for source, target in init_mounts),
    )


def wait_postgres(container: str, password: str, timeout: float = 180.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            [
                "docker",
                "exec",
                "-e",
                f"PGPASSWORD={password}",
                container,
                "pg_isready",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if result.returncode == 0:
            return
        time.sleep(1)
    logs = docker("logs", "--tail", "120", container, capture=True, check=False)
    raise RuntimeError(f"PostgreSQL did not become ready: {container}; logs: {logs}")


def psql(
    container: str,
    password: str,
    sql: str,
    database: str = "postgres",
    *,
    user: str = "postgres",
    check: bool = True,
) -> str:
    return docker(
        "exec",
        "-e",
        f"PGPASSWORD={password}",
        container,
        "psql",
        "-h",
        "127.0.0.1",
        "-U",
        user,
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


def wait_for_archive_file(
    volume: str,
    image: str,
    wal_file: str,
    timeout: float = 60.0,
) -> None:
    if len(wal_file) != 24 or any(
        character not in "0123456789ABCDEF" for character in wal_file
    ):
        raise RuntimeError(f"invalid WAL archive filename: {wal_file!r}")
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            [
                "docker",
                "run",
                "--rm",
                "-v",
                f"{volume}:/archive:ro",
                "--entrypoint",
                "sh",
                image,
                "-eu",
                "-c",
                'test -f "/archive/$1"',
                "archive-check",
                wal_file,
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if result.returncode == 0:
            return
        time.sleep(1)
    inventory = docker(
        "run",
        "--rm",
        "-v",
        f"{volume}:/archive:ro",
        "--entrypoint",
        "sh",
        image,
        "-c",
        "find /archive -maxdepth 1 -type f -printf '%f\\n' | sort",
        capture=True,
        check=False,
    )
    raise RuntimeError(
        f"PostgreSQL WAL archive did not receive switched segment {wal_file}; "
        f"observed={inventory!r}"
    )


def switch_wal_and_wait(
    container: str,
    password: str,
    archive_volume: str,
    image: str,
) -> str:
    wal_file = psql(
        container,
        password,
        "SELECT pg_walfile_name(pg_switch_wal());",
        user="supabase_admin",
    )
    wait_for_archive_file(archive_volume, image, wal_file)
    return wal_file


def postgres_environment(password: str, jwt_secret: str) -> list[str]:
    values = {
        "POSTGRES_HOST": "/var/run/postgresql",
        "PGPORT": "5432",
        "POSTGRES_PORT": "5432",
        "PGPASSWORD": password,
        "POSTGRES_PASSWORD": password,
        "PGDATABASE": "postgres",
        "POSTGRES_DB": "postgres",
        "JWT_SECRET": jwt_secret,
        "JWT_EXP": "3600",
    }
    arguments: list[str] = []
    for key, value in values.items():
        arguments.extend(("-e", f"{key}={value}"))
    return arguments


def postgres_command(*extra: str) -> list[str]:
    return [
        "postgres",
        "-c",
        "config_file=/etc/postgresql/postgresql.conf",
        "-c",
        "hba_file=/etc/postgresql/pg_hba.conf",
        "-c",
        "log_min_messages=fatal",
        *extra,
    ]


def verify_supabase_runtime_state(container: str, password: str) -> dict[str, Any]:
    raw = psql(
        container,
        password,
        "SELECT current_setting('config_file'), current_setting('hba_file'), "
        "current_setting('server_version_num'), "
        "COALESCE((SELECT rolsuper::text FROM pg_roles WHERE rolname='supabase_admin'), 'missing');",
    )
    parts = raw.split("|")
    if len(parts) != 4:
        raise RuntimeError(f"unexpected Supabase PostgreSQL runtime state: {raw!r}")
    config_file, hba_file, version_num, admin_super = parts
    if config_file != "/etc/postgresql/postgresql.conf":
        raise RuntimeError(f"unexpected PostgreSQL config file: {config_file}")
    if hba_file != "/etc/postgresql/pg_hba.conf":
        raise RuntimeError(f"unexpected PostgreSQL HBA file: {hba_file}")
    if not version_num.isdigit() or int(version_num) < 170000:
        raise RuntimeError(f"unexpected PostgreSQL version: {version_num}")
    if admin_super != "true":
        raise RuntimeError(
            f"Supabase administrative role is missing or constrained: {admin_super}"
        )
    return {
        "config_file": config_file,
        "hba_file": hba_file,
        "server_version_num": int(version_num),
        "supabase_admin_superuser": True,
    }


def verify_wal_switch_boundary(container: str, password: str) -> dict[str, bool]:
    raw = psql(
        container,
        password,
        "SELECT (SELECT rolsuper::text FROM pg_roles WHERE rolname='postgres'), "
        "has_function_privilege('postgres', 'pg_catalog.pg_switch_wal()', 'EXECUTE')::text, "
        "has_function_privilege('supabase_admin', 'pg_catalog.pg_switch_wal()', 'EXECUTE')::text;",
        user="supabase_admin",
    )
    if raw != "false|false|true":
        raise RuntimeError(f"unexpected WAL switch privilege boundary: {raw}")
    return {
        "postgres_superuser": False,
        "postgres_can_switch_wal": False,
        "supabase_admin_can_switch_wal": True,
    }


def postgres_drill(prefix: str, runtime: SupabasePostgresRuntime) -> dict[str, Any]:
    source = f"{prefix}-pg-source"
    restored = f"{prefix}-pg-restored"
    data_volume = f"{prefix}-pg-source-data"
    base_volume = f"{prefix}-pg-base"
    archive_volume = f"{prefix}-pg-archive"
    config_volume = f"{prefix}-pg-config"
    password = secrets.token_urlsafe(24)
    jwt_secret = secrets.token_urlsafe(48)
    for volume in (data_volume, base_volume, archive_volume, config_volume):
        docker("volume", "create", volume, capture=True)

    source_arguments = [
        "run",
        "--detach",
        "--name",
        source,
        *postgres_environment(password, jwt_secret),
        "-v",
        f"{data_volume}:/var/lib/postgresql/data",
        "-v",
        f"{base_volume}:/backup",
        "-v",
        f"{archive_volume}:/archive",
        "-v",
        f"{config_volume}:/etc/postgresql-custom",
        "-v",
        f"{runtime.pg_hba}:/etc/postgresql/pg_hba.conf:ro",
    ]
    for source_path, target in runtime.init_mounts:
        source_arguments.extend(("-v", f"{source_path}:{target}:ro"))
    source_arguments.extend(
        (
            runtime.image,
            *postgres_command(
                "-c",
                "wal_level=replica",
                "-c",
                "archive_mode=on",
                "-c",
                "archive_timeout=2s",
                "-c",
                "archive_command=test ! -f /archive/%f && cp %p /archive/%f",
            ),
        )
    )
    docker(*source_arguments, capture=True)
    wait_postgres(source, password)
    docker(
        "exec",
        "--user",
        "0",
        source,
        "chown",
        "-R",
        "postgres:postgres",
        "/backup",
        "/archive",
    )
    source_runtime_state = verify_supabase_runtime_state(source, password)
    wal_switch_boundary = verify_wal_switch_boundary(source, password)
    psql(
        source,
        password,
        "CREATE TABLE recovery_probe(id integer PRIMARY KEY, marker text NOT NULL); "
        "INSERT INTO recovery_probe VALUES (1, 'base');",
    )
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
    before_target_wal = switch_wal_and_wait(
        source, password, archive_volume, runtime.image
    )
    time.sleep(1.2)
    psql(source, password, "INSERT INTO recovery_probe VALUES (3, 'after-target');")
    after_target_wal = switch_wal_and_wait(
        source, password, archive_volume, runtime.image
    )
    if after_target_wal == before_target_wal:
        raise RuntimeError(
            f"forced WAL switches returned the same archive segment: {after_target_wal}"
        )
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
        runtime.image,
        "-eu",
        "-c",
        'printf "%s" "$1" >> /restore/postgresql.auto.conf; '
        "touch /restore/recovery.signal; chown -R postgres:postgres /restore",
        "recovery-config",
        config,
    )
    docker(
        "run",
        "--detach",
        "--name",
        restored,
        *postgres_environment(password, jwt_secret),
        "-v",
        f"{base_volume}:/var/lib/postgresql/data",
        "-v",
        f"{archive_volume}:/archive:ro",
        "-v",
        f"{config_volume}:/etc/postgresql-custom",
        "-v",
        f"{runtime.pg_hba}:/etc/postgresql/pg_hba.conf:ro",
        runtime.image,
        *postgres_command(),
        capture=True,
    )
    wait_postgres(restored, password)
    restored_runtime_state = verify_supabase_runtime_state(restored, password)
    deadline = time.monotonic() + 90
    rows = ""
    while time.monotonic() < deadline:
        rows = psql(
            restored,
            password,
            "SELECT string_agg(id::text || ':' || marker, ',' ORDER BY id) "
            "FROM recovery_probe;",
            check=False,
        )
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
            "BEGIN; CREATE TABLE failed_migration_probe(id integer); "
            "INSERT INTO failed_migration_probe VALUES (1); SELECT 1/0; COMMIT;",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if failed.returncode == 0:
        raise RuntimeError("deliberately failed migration unexpectedly succeeded")
    rollback_state = psql(
        restored,
        password,
        "SELECT to_regclass('public.failed_migration_probe') IS NULL;",
    )
    if rollback_state != "t":
        raise RuntimeError("failed migration left a partial table behind")
    return {
        "image": runtime.image,
        "upstream_commit": runtime.upstream_commit,
        "source_runtime": source_runtime_state,
        "restored_runtime": restored_runtime_state,
        "wal_switch_boundary": wal_switch_boundary,
        "archived_wal_segments": {
            "before_target": before_target_wal,
            "after_target": after_target_wal,
        },
        "target_time": target_time,
        "restored_rows": rows,
        "failed_migration_rolled_back": True,
    }


def http_json(
    url: str,
    method: str = "GET",
    payload: dict[str, Any] | None = None,
    headers: dict[str, str] | None = None,
) -> dict[str, Any]:
    body = json.dumps(payload).encode() if payload is not None else None
    request_headers = {"Content-Type": "application/json"} if body is not None else {}
    request_headers.update(headers or {})
    request = urllib.request.Request(
        url, data=body, method=method, headers=request_headers
    )
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
        f"--{boundary}\r\nContent-Disposition: form-data; name=\"snapshot\"; "
        'filename="snapshot.snapshot"\r\nContent-Type: application/octet-stream\r\n\r\n'
    ).encode() + snapshot + f"\r\n--{boundary}--\r\n".encode()
    request = urllib.request.Request(
        url,
        data=body,
        method="POST",
        headers={
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Content-Length": str(len(body)),
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.loads(response.read().decode())


def qdrant_drill(prefix: str) -> dict[str, Any]:
    container = f"{prefix}-qdrant"
    image = "qdrant/qdrant:v1.18.2"
    docker(
        "run",
        "--detach",
        "--name",
        container,
        "-p",
        "127.0.0.1::6333",
        image,
        capture=True,
    )
    port = docker("port", container, "6333/tcp", capture=True).rsplit(":", 1)[1]
    base = f"http://127.0.0.1:{port}"
    wait_http(f"{base}/readyz")
    collection = "recovery_drill"
    http_json(
        f"{base}/collections/{collection}",
        "PUT",
        {"vectors": {"size": 4, "distance": "Cosine"}},
    )
    http_json(
        f"{base}/collections/{collection}/points?wait=true",
        "PUT",
        {
            "points": [
                {
                    "id": 1,
                    "vector": [0.1, 0.2, 0.3, 0.4],
                    "payload": {
                        "school_id": "00000000-0000-4000-8000-000000000001",
                        "marker": "restore",
                    },
                }
            ]
        },
    )
    snapshot_name = http_json(
        f"{base}/collections/{collection}/snapshots", "POST"
    )["result"]["name"]
    with urllib.request.urlopen(
        f"{base}/collections/{collection}/snapshots/{snapshot_name}", timeout=30
    ) as response:
        snapshot = response.read()
    http_json(f"{base}/collections/{collection}", "DELETE")
    upload_snapshot(
        f"{base}/collections/{collection}/snapshots/upload?priority=snapshot",
        snapshot,
    )
    point = http_json(f"{base}/collections/{collection}/points/1")["result"]
    if point.get("payload", {}).get("marker") != "restore":
        raise RuntimeError("Qdrant restored point payload mismatch")
    return {"snapshot_name": snapshot_name, "restored_point_id": point["id"]}


def cleanup(prefix: str) -> None:
    for suffix in ("pg-source", "pg-restored", "qdrant"):
        subprocess.run(
            ["docker", "rm", "--force", f"{prefix}-{suffix}"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    for suffix in ("pg-source-data", "pg-base", "pg-archive", "pg-config"):
        subprocess.run(
            ["docker", "volume", "rm", "--force", f"{prefix}-{suffix}"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    prefix = f"edutalent-drill-{secrets.token_hex(4)}"
    started = time.monotonic()
    try:
        runtime = materialize_pinned_supabase_runtime()
        result = {
            "schema_version": 1,
            "postgres": postgres_drill(prefix, runtime),
            "qdrant": qdrant_drill(prefix),
            "duration_seconds": time.monotonic() - started,
        }
        args.output.write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    finally:
        cleanup(prefix)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        OSError,
        RuntimeError,
        subprocess.CalledProcessError,
        urllib.error.URLError,
        json.JSONDecodeError,
    ) as error:
        print(f"recovery drill failed: {error}", file=sys.stderr)
        raise SystemExit(1)
