#!/usr/bin/env python3
"""Render compact per-service hardening evidence from Docker Compose JSON."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

DIGEST_RE = re.compile(r"^.+@sha256:[0-9a-f]{64}$")


def volume_summary(service: dict[str, Any]) -> tuple[list[str], list[str]]:
    writable: list[str] = []
    readonly: list[str] = []
    for mount in service.get("volumes", []) or []:
        if not isinstance(mount, dict):
            continue
        target = str(mount.get("target", ""))
        if not target:
            continue
        if mount.get("read_only") is True:
            readonly.append(target)
        else:
            writable.append(target)
    return sorted(writable), sorted(readonly)


def resource_value(service: dict[str, Any], key: str) -> Any:
    return (
        (service.get("deploy", {}) or {})
        .get("resources", {})
        .get("limits", {})
        .get(key)
    )


def summarize_service(name: str, service: dict[str, Any]) -> dict[str, Any]:
    writable, readonly = volume_summary(service)
    networks_raw = service.get("networks", {}) or {}
    networks = sorted(networks_raw if isinstance(networks_raw, dict) else networks_raw)
    ports: list[dict[str, Any] | str] = []
    for item in service.get("ports", []) or []:
        if isinstance(item, dict):
            ports.append(
                {
                    "host_ip": item.get("host_ip"),
                    "published": item.get("published"),
                    "target": item.get("target"),
                    "protocol": item.get("protocol", "tcp"),
                }
            )
        else:
            ports.append(str(item))
    image = str(service.get("image", ""))
    security_opt = sorted(str(value) for value in service.get("security_opt", []) or [])
    cap_add = sorted(str(value).upper() for value in service.get("cap_add", []) or [])
    cap_drop = sorted(str(value).upper() for value in service.get("cap_drop", []) or [])
    user = str(service.get("user", ""))
    return {
        "service": name,
        "image": image,
        "image_is_digest": bool(DIGEST_RE.match(image)),
        "user": user or None,
        "explicit_non_root_user": bool(user and user not in {"0", "0:0", "root"}),
        "privileged": bool(service.get("privileged", False)),
        "read_only_root": service.get("read_only") is True,
        "cap_add": cap_add,
        "cap_drop": cap_drop,
        "drops_all_capabilities": "ALL" in cap_drop and not cap_add,
        "pids_limit": service.get("pids_limit"),
        "memory_limit": resource_value(service, "memory"),
        "cpu_limit": resource_value(service, "cpus"),
        "restart": service.get("restart"),
        "healthcheck_present": bool(service.get("healthcheck")),
        "security_opt": security_opt,
        "no_new_privileges": any(
            value.lower().replace("=", ":") == "no-new-privileges:true"
            for value in security_opt
        ),
        "networks": networks,
        "published_ports": ports,
        "writable_paths": writable,
        "read_only_paths": readonly,
    }


def build_inventory(document: dict[str, Any], *, require_digests: bool) -> dict[str, Any]:
    services = document.get("services")
    if not isinstance(services, dict) or not services:
        raise ValueError("rendered Compose JSON must contain services")
    rows = [
        summarize_service(name, service)
        for name, service in sorted(services.items())
        if isinstance(service, dict)
    ]
    missing_digests = [row["service"] for row in rows if not row["image_is_digest"]]
    privileged = [row["service"] for row in rows if row["privileged"]]
    host_network = [
        name
        for name, service in sorted(services.items())
        if isinstance(service, dict) and service.get("network_mode") == "host"
    ]
    docker_socket = []
    for name, service in sorted(services.items()):
        if not isinstance(service, dict):
            continue
        for mount in service.get("volumes", []) or []:
            if isinstance(mount, dict) and str(mount.get("source", "")).endswith("docker.sock"):
                docker_socket.append(name)
                break
    failed: list[str] = []
    if privileged:
        failed.append("privileged-services")
    if host_network:
        failed.append("host-network-services")
    if docker_socket:
        failed.append("docker-socket-mounts")
    if require_digests and missing_digests:
        failed.append("non-digest-images")
    return {
        "schema_version": 1,
        "status": "fail" if failed else "pass",
        "require_digests": require_digests,
        "service_count": len(rows),
        "services": rows,
        "summary": {
            "non_digest_images": missing_digests,
            "privileged_services": privileged,
            "host_network_services": host_network,
            "docker_socket_services": docker_socket,
            "failed_checks": failed,
        },
        "note": (
            "Source-topology inventory may contain pinned tags. Final release/target-host "
            "acceptance must rerun this inventory against the immutable locked release with "
            "--require-digests and attach the resulting evidence."
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("compose_json", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--require-digests", action="store_true")
    args = parser.parse_args()
    document = json.loads(args.compose_json.read_text(encoding="utf-8"))
    inventory = build_inventory(document, require_digests=args.require_digests)
    text = json.dumps(inventory, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8")
    print(text, end="")
    if inventory["status"] != "pass":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
