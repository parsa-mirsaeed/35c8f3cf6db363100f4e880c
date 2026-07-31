#!/usr/bin/env python3
"""Deterministic local operations helpers for the EduTalent appliance."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import http.server
import json
import os
import ssl
import statistics
import sys
import threading
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

MANIFEST_NAME = "backup-manifest.json"


def utc_timestamp() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def iter_payload_files(root: Path) -> Iterable[Path]:
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise RuntimeError(f"backup payload contains a symlink: {path.relative_to(root)}")
        if path.is_dir():
            continue
        if not path.is_file():
            raise RuntimeError(f"backup payload contains a non-regular entry: {path.relative_to(root)}")
        if path.name != MANIFEST_NAME:
            yield path


def normalize_payload_permissions(root: Path) -> dict[str, int]:
    root = root.resolve()
    if not root.is_dir():
        raise RuntimeError(f"backup payload is not a directory: {root}")
    root.chmod(0o700)
    directories = 1
    files = 0
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise RuntimeError(
                f"backup payload contains a symlink: {path.relative_to(root)}"
            )
        if path.is_dir():
            path.chmod(0o700)
            directories += 1
            continue
        if not path.is_file():
            raise RuntimeError(
                f"backup payload contains a non-regular entry: {path.relative_to(root)}"
            )
        path.chmod(0o600)
        files += 1
    return {"directories": directories, "files": files}


def verify_backup_metadata(metadata_path: Path, backup_dir: Path) -> dict[str, Any]:
    backup_dir = backup_dir.resolve()
    if metadata_path.is_symlink() or not metadata_path.is_file():
        raise RuntimeError(f"backup metadata is not a regular file: {metadata_path}")
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    if metadata.get("schema_version") != 1:
        raise RuntimeError("unsupported backup metadata schema")
    archive_name = metadata.get("archive")
    if not isinstance(archive_name, str) or Path(archive_name).name != archive_name:
        raise RuntimeError("backup metadata contains an invalid archive name")
    archive = (backup_dir / archive_name).resolve()
    if archive.parent != backup_dir or archive.is_symlink() or not archive.is_file():
        raise RuntimeError(f"backup archive is missing or unsafe: {archive_name}")
    expected = metadata.get("sha256")
    if not isinstance(expected, str) or len(expected) != 64 or any(
        character not in "0123456789abcdef" for character in expected.lower()
    ):
        raise RuntimeError("backup metadata contains an invalid SHA-256 digest")
    observed = sha256_file(archive)
    if observed != expected.lower():
        raise RuntimeError(f"backup archive checksum mismatch: {archive_name}")
    created_at = metadata.get("created_at")
    if not isinstance(created_at, str) or parse_iso8601_epoch(created_at) is None:
        raise RuntimeError("backup metadata contains an invalid creation timestamp")
    return metadata


def create_manifest(root: Path, metadata: dict[str, Any] | None = None) -> dict[str, Any]:
    root = root.resolve()
    if not root.is_dir():
        raise RuntimeError(f"backup payload is not a directory: {root}")
    normalize_payload_permissions(root)
    files = []
    for path in iter_payload_files(root):
        stat = path.stat()
        files.append(
            {
                "path": path.relative_to(root).as_posix(),
                "size": stat.st_size,
                "sha256": sha256_file(path),
                "mode": stat.st_mode & 0o777,
            }
        )
    manifest = {
        "schema_version": 1,
        "created_at": utc_timestamp(),
        "metadata": metadata or {},
        "files": files,
    }
    (root / MANIFEST_NAME).write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return manifest


def verify_manifest(root: Path) -> dict[str, Any]:
    root = root.resolve()
    manifest_path = root / MANIFEST_NAME
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        raise RuntimeError("unsupported backup manifest schema")
    expected = {row["path"]: row for row in manifest.get("files", [])}
    observed_paths = {path.relative_to(root).as_posix() for path in iter_payload_files(root)}
    if observed_paths != set(expected):
        missing = sorted(set(expected) - observed_paths)
        extra = sorted(observed_paths - set(expected))
        raise RuntimeError(f"backup inventory mismatch; missing={missing}, extra={extra}")
    for relative, row in expected.items():
        path = root / relative
        if path.stat().st_size != row["size"]:
            raise RuntimeError(f"backup size mismatch: {relative}")
        if sha256_file(path) != row["sha256"]:
            raise RuntimeError(f"backup checksum mismatch: {relative}")
        if path.stat().st_mode & 0o777 != row["mode"]:
            raise RuntimeError(f"backup mode mismatch: {relative}")
    return manifest


def normalize_compose_ps(value: Any) -> list[dict[str, Any]]:
    if isinstance(value, list):
        return [row for row in value if isinstance(row, dict)]
    if isinstance(value, dict):
        return [value]
    raise RuntimeError("Compose service state must be an object or array")


def validate_compose_security(config: dict[str, Any]) -> list[str]:
    violations: list[str] = []
    services = config.get("services") or {}
    networks = config.get("networks") or {}
    if not isinstance(services, dict) or not isinstance(networks, dict):
        return ["rendered Compose config is missing services or networks"]

    for service_name, service in services.items():
        if not isinstance(service, dict):
            violations.append(f"{service_name}: invalid service definition")
            continue
        if service.get("privileged"):
            violations.append(f"{service_name}: privileged containers are forbidden")
        if service.get("network_mode") == "host":
            violations.append(f"{service_name}: host networking is forbidden")
        volumes = service.get("volumes") or []
        for volume in volumes:
            source = volume.get("source") if isinstance(volume, dict) else str(volume).split(":", 1)[0]
            if source in {"/var/run/docker.sock", "/run/docker.sock"}:
                violations.append(f"{service_name}: Docker socket mount is forbidden")
        ports = service.get("ports") or []
        if ports and service_name != "gateway":
            violations.append(f"{service_name}: only gateway may publish host ports")
        if service_name == "gateway":
            published = set()
            for port in ports:
                if isinstance(port, dict):
                    published.add(str(port.get("published")))
                else:
                    published.add(str(port).split(":")[0])
            if published != {"80", "443"}:
                violations.append(f"gateway: expected only host ports 80 and 443, observed {sorted(published)}")
        service_networks = service.get("networks") or []
        if isinstance(service_networks, dict):
            service_networks = list(service_networks)
        if "ai_egress" in service_networks and service_name != "ai-gateway":
            violations.append(f"{service_name}: only ai-gateway may join ai_egress")
        security_opt = service.get("security_opt") or []
        if service_name not in {"gateway-tls"} and "no-new-privileges:true" not in security_opt:
            violations.append(f"{service_name}: no-new-privileges is required")

    for network_name in {"edge", "supabase_api", "data", "admin", "ai_internal"}:
        definition = networks.get(network_name)
        if not isinstance(definition, dict) or definition.get("internal") is not True:
            violations.append(f"{network_name}: internal network boundary is required")
    return violations


def parse_iso8601_epoch(value: str | None) -> float | None:
    if not value:
        return None
    normalized = value.strip().replace("Z", "+00:00")
    try:
        from datetime import datetime

        return datetime.fromisoformat(normalized).timestamp()
    except ValueError:
        return None


def evaluate_alerts(snapshot: dict[str, Any], policy: dict[str, Any]) -> list[dict[str, Any]]:
    alerts: list[dict[str, Any]] = []
    now = float(snapshot.get("collected_epoch", time.time()))
    required_services = set(policy.get("required_services", []))
    services = snapshot.get("services") or []
    states: dict[str, tuple[str, str]] = {}
    for row in services:
        if not isinstance(row, dict):
            continue
        name = str(row.get("Service") or row.get("Name") or row.get("service") or "")
        state = str(row.get("State") or row.get("state") or "unknown").lower()
        health = str(row.get("Health") or row.get("health") or "").lower()
        if name:
            states[name] = (state, health)
    for service in sorted(required_services):
        state, health = states.get(service, ("missing", ""))
        if state != "running" or health in {"unhealthy", "starting"}:
            alerts.append({"severity": "critical", "code": "service_unhealthy", "service": service, "state": state, "health": health})

    disk_free = int(snapshot.get("disk_free_bytes", -1))
    min_disk = int(policy.get("minimum_disk_free_bytes", 0))
    if disk_free < 0:
        alerts.append({"severity": "critical", "code": "disk_status_unknown"})
    elif disk_free < min_disk:
        alerts.append({"severity": "critical", "code": "disk_free_low", "observed": disk_free, "threshold": min_disk})

    if snapshot.get("backup_configured") is True:
        backup_disk_free = int(snapshot.get("backup_disk_free_bytes", -1))
        min_backup_disk = int(policy.get("minimum_backup_disk_free_bytes", min_disk))
        if backup_disk_free < 0:
            alerts.append({"severity": "critical", "code": "backup_disk_status_unknown"})
        elif backup_disk_free < min_backup_disk:
            alerts.append({"severity": "critical", "code": "backup_disk_free_low", "observed": backup_disk_free, "threshold": min_backup_disk})
        if snapshot.get("latest_backup_verified") is not True:
            alerts.append({"severity": "critical", "code": "backup_unverifiable"})

    cert_seconds = int(snapshot.get("tls_seconds_remaining", -1))
    cert_min = int(policy.get("minimum_tls_seconds_remaining", 0))
    if cert_seconds < 0:
        alerts.append({"severity": "critical", "code": "tls_status_unknown"})
    elif cert_seconds < cert_min:
        alerts.append({"severity": "critical", "code": "tls_expiring", "observed": cert_seconds, "threshold": cert_min})

    connections = int(snapshot.get("database_connections", -1))
    connection_limit = int(snapshot.get("database_connection_limit", -1))
    max_ratio = float(policy.get("maximum_database_connection_ratio", 0.85))
    if connections >= 0 and connection_limit > 0 and connections / connection_limit >= max_ratio:
        alerts.append({"severity": "warning", "code": "database_connections_high", "observed": connections, "limit": connection_limit})

    for key, code, max_age_key in (
        ("latest_backup_created_at", "backup_stale", "maximum_backup_age_seconds"),
        ("latest_wal_received_at", "wal_archive_stale", "maximum_wal_age_seconds"),
    ):
        maximum_age = int(policy.get(max_age_key, 0))
        observed = parse_iso8601_epoch(snapshot.get(key))
        if maximum_age > 0 and (observed is None or now - observed > maximum_age):
            alerts.append({"severity": "critical", "code": code, "observed": snapshot.get(key), "maximum_age_seconds": maximum_age})

    if snapshot.get("wal_receiver_running") is not True:
        alerts.append({"severity": "critical", "code": "wal_receiver_stopped"})
    if snapshot.get("core_health") is False:
        alerts.append({"severity": "critical", "code": "core_health_failed"})
    if snapshot.get("qdrant_health") is False:
        alerts.append({"severity": "warning", "code": "qdrant_unavailable"})
    if snapshot.get("ai_gateway_health") is False:
        alerts.append({"severity": "warning", "code": "ai_gateway_unavailable"})
    return alerts

@dataclass(frozen=True)
class LoadResult:
    requests: int
    failures: int
    duration_seconds: float
    latencies_ms: list[float]

    def as_dict(self) -> dict[str, Any]:
        sorted_latencies = sorted(self.latencies_ms)
        p95 = 0.0
        if sorted_latencies:
            index = min(len(sorted_latencies) - 1, max(0, int(len(sorted_latencies) * 0.95) - 1))
            p95 = sorted_latencies[index]
        return {
            "requests": self.requests,
            "failures": self.failures,
            "error_rate": self.failures / self.requests if self.requests else 1.0,
            "duration_seconds": self.duration_seconds,
            "requests_per_second": self.requests / self.duration_seconds if self.duration_seconds else 0.0,
            "latency_ms": {
                "min": min(sorted_latencies) if sorted_latencies else 0.0,
                "median": statistics.median(sorted_latencies) if sorted_latencies else 0.0,
                "p95": p95,
                "max": max(sorted_latencies) if sorted_latencies else 0.0,
            },
        }


def run_load_test(url: str, duration: float, concurrency: int, timeout: float, insecure: bool = False) -> LoadResult:
    stop_at = time.monotonic() + duration
    lock = threading.Lock()
    context = ssl._create_unverified_context() if insecure else None
    latencies: list[float] = []
    requests = 0
    failures = 0

    def worker() -> None:
        nonlocal requests, failures
        local_latencies: list[float] = []
        local_requests = 0
        local_failures = 0
        while time.monotonic() < stop_at:
            started = time.monotonic()
            try:
                request = urllib.request.Request(url, headers={"User-Agent": "edutalent-operations-load-test/1"})
                with urllib.request.urlopen(request, timeout=timeout, context=context) as response:
                    if not 200 <= response.status < 400:
                        local_failures += 1
                    response.read(1024)
            except (OSError, urllib.error.URLError, TimeoutError):
                local_failures += 1
            local_requests += 1
            local_latencies.append((time.monotonic() - started) * 1000.0)
        with lock:
            requests += local_requests
            failures += local_failures
            latencies.extend(local_latencies)

    started = time.monotonic()
    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = [executor.submit(worker) for _ in range(concurrency)]
        for future in futures:
            future.result()
    return LoadResult(requests, failures, time.monotonic() - started, latencies)


def parse_json_file(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def command_manifest_create(args: argparse.Namespace) -> None:
    metadata = parse_json_file(args.metadata) if args.metadata else {}
    manifest = create_manifest(args.root, metadata)
    print(json.dumps(manifest, sort_keys=True))


def command_manifest_verify(args: argparse.Namespace) -> None:
    manifest = verify_manifest(args.root)
    print(json.dumps({"verified": True, "files": len(manifest["files"])}, sort_keys=True))


def command_backup_metadata_verify(args: argparse.Namespace) -> None:
    metadata = verify_backup_metadata(args.metadata, args.backup_dir)
    print(metadata["created_at"])


def command_compose_security(args: argparse.Namespace) -> None:
    violations = validate_compose_security(parse_json_file(args.config))
    output = {"passed": not violations, "violations": violations}
    print(json.dumps(output, indent=2, sort_keys=True))
    if violations:
        raise SystemExit(1)


def command_evaluate_alerts(args: argparse.Namespace) -> None:
    alerts = evaluate_alerts(parse_json_file(args.snapshot), parse_json_file(args.policy))
    output = {"evaluated_at": utc_timestamp(), "alerts": alerts}
    text = json.dumps(output, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(text, encoding="utf-8")
    print(text, end="")
    if any(alert.get("severity") == "critical" for alert in alerts):
        raise SystemExit(2)


def command_load_test(args: argparse.Namespace) -> None:
    result = run_load_test(args.url, args.duration, args.concurrency, args.timeout, args.insecure)
    output = result.as_dict()
    text = json.dumps(output, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(text, encoding="utf-8")
    print(text, end="")
    if output["error_rate"] > args.maximum_error_rate:
        raise SystemExit("load test error rate exceeded threshold")
    if output["latency_ms"]["p95"] > args.maximum_p95_ms:
        raise SystemExit("load test p95 latency exceeded threshold")


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser()
    subparsers = root.add_subparsers(dest="command", required=True)

    create = subparsers.add_parser("manifest-create")
    create.add_argument("--root", type=Path, required=True)
    create.add_argument("--metadata", type=Path)
    create.set_defaults(func=command_manifest_create)

    verify = subparsers.add_parser("manifest-verify")
    verify.add_argument("--root", type=Path, required=True)
    verify.set_defaults(func=command_manifest_verify)

    metadata = subparsers.add_parser("backup-metadata-verify")
    metadata.add_argument("--metadata", type=Path, required=True)
    metadata.add_argument("--backup-dir", type=Path, required=True)
    metadata.set_defaults(func=command_backup_metadata_verify)

    security = subparsers.add_parser("compose-security")
    security.add_argument("--config", type=Path, required=True)
    security.set_defaults(func=command_compose_security)

    alerts = subparsers.add_parser("evaluate-alerts")
    alerts.add_argument("--snapshot", type=Path, required=True)
    alerts.add_argument("--policy", type=Path, required=True)
    alerts.add_argument("--output", type=Path)
    alerts.set_defaults(func=command_evaluate_alerts)

    load = subparsers.add_parser("load-test")
    load.add_argument("--url", required=True)
    load.add_argument("--duration", type=float, default=60.0)
    load.add_argument("--concurrency", type=int, default=16)
    load.add_argument("--timeout", type=float, default=5.0)
    load.add_argument("--maximum-error-rate", type=float, default=0.01)
    load.add_argument("--maximum-p95-ms", type=float, default=1000.0)
    load.add_argument("--output", type=Path)
    load.add_argument("--insecure", action="store_true")
    load.set_defaults(func=command_load_test)
    return root


def main() -> int:
    args = parser().parse_args()
    args.func(args)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"operations error: {error}", file=sys.stderr)
        raise SystemExit(1)
