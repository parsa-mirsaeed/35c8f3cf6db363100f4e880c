#!/usr/bin/env python3
"""Fail closed on production Compose exposure and privilege regressions."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit


def fail(message: str) -> None:
    print(f"production compose validation failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def service_cpu_limit(service: dict[str, Any]) -> float | None:
    raw = (
        service.get("deploy", {})
        .get("resources", {})
        .get("limits", {})
        .get("cpus")
    )
    if raw is None:
        return None
    try:
        limit = float(raw)
    except (TypeError, ValueError) as error:
        fail(f"invalid CPU limit {raw!r}")
        raise AssertionError("unreachable") from error
    if limit <= 0:
        fail(f"CPU limit must be greater than zero, got {limit}")
    return limit


def volume_mounts(service: dict[str, Any]) -> list[dict[str, Any]]:
    mounts: list[dict[str, Any]] = []
    for volume in service.get("volumes", []) or []:
        if not isinstance(volume, dict):
            fail(f"service volume must be rendered as an object, got {volume!r}")
        mounts.append(volume)
    return mounts


def mount_for_target(
    service: dict[str, Any], target: str
) -> dict[str, Any] | None:
    return next(
        (mount for mount in volume_mounts(service) if mount.get("target") == target),
        None,
    )


def dependency_condition(service: dict[str, Any], dependency: str) -> str | None:
    dependencies = service.get("depends_on", {}) or {}
    entry = dependencies.get(dependency)
    if isinstance(entry, dict):
        value = entry.get("condition")
        return str(value) if value is not None else None
    return None


def parse_database_user(database_url: str, label: str) -> str:
    try:
        parsed = urlsplit(database_url)
    except ValueError as error:
        fail(f"{label} is not a valid PostgreSQL URL: {error}")
        raise AssertionError("unreachable") from error
    if parsed.scheme not in {"postgres", "postgresql"}:
        fail(f"{label} must use the PostgreSQL URL scheme")
    if parsed.hostname != "db" or parsed.port != 5432:
        fail(f"{label} must target the private Supabase db service on port 5432")
    if not parsed.username:
        fail(f"{label} is missing a database username")
    return parsed.username


def main() -> None:
    if len(sys.argv) not in {2, 3}:
        fail(
            "usage: validate-rendered-compose.py "
            "<compose-config.json> [docker-host-cpus]"
        )

    host_cpus: float | None = None
    if len(sys.argv) == 3:
        try:
            host_cpus = float(sys.argv[2])
        except ValueError as error:
            fail(f"invalid Docker host CPU count: {sys.argv[2]!r}")
            raise AssertionError("unreachable") from error
        if host_cpus <= 0:
            fail(f"Docker host CPU count must be positive, got {host_cpus}")

    deployment_dir = Path(__file__).resolve().parent
    overlay_text = (deployment_dir / "compose.production.yaml").read_text(
        encoding="utf-8"
    )
    caddyfile_text = (deployment_dir / "Caddyfile").read_text(encoding="utf-8")
    if 'profiles: ["edge-functions"]' not in overlay_text:
        fail("Edge Functions source definition must use the explicit profile")
    if "/functions/v1" in caddyfile_text:
        fail("inactive Edge Functions must not have a public Caddy route")
    if "http_port 8080" not in caddyfile_text or "https_port 8443" not in caddyfile_text:
        fail("Caddy must listen on unprivileged internal ports 8080 and 8443")
    state_owner_command = (
        'chown -R "$${GATEWAY_UID}:$${GATEWAY_GID}" /data /config'
    )
    if state_owner_command not in overlay_text:
        fail("gateway initialization must own persistent Caddy state volumes")
    if "chmod 0700 /data /config" not in overlay_text:
        fail("gateway initialization must restrict persistent Caddy state volumes")

    document = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    services = document.get("services")
    networks = document.get("networks")
    if not isinstance(services, dict) or not isinstance(networks, dict):
        fail("rendered document is missing services or networks")

    required_services = {
        "gateway-tls",
        "gateway",
        "app",
        "migrate",
        "database-access",
        "qdrant",
        "studio",
        "kong",
        "auth",
        "rest",
        "realtime",
        "storage",
        "imgproxy",
        "meta",
        "db",
        "supavisor",
    }
    missing = sorted(required_services - set(services))
    if missing:
        fail(f"missing required services: {', '.join(missing)}")

    for name, service in services.items():
        if service.get("privileged"):
            fail(f"service {name} is privileged")
        if service.get("network_mode") == "host":
            fail(f"service {name} uses host networking")
        for volume in volume_mounts(service):
            source = str(volume.get("source", ""))
            if source in {"/var/run/docker.sock", "/run/docker.sock"} or source.endswith(
                "/docker.sock"
            ):
                fail(f"service {name} mounts the Docker socket")

        ports = service.get("ports", []) or []
        if name != "gateway" and ports:
            fail(f"service {name} publishes host ports: {ports}")

        attached_networks = service.get("networks", {}) or {}
        attached_names = (
            attached_networks.keys()
            if isinstance(attached_networks, dict)
            else attached_networks
        )
        for network_key in attached_names:
            network = networks.get(network_key)
            if not isinstance(network, dict) or network.get("internal") is not True:
                fail(f"service {name} is attached to non-internal network {network_key}")

        if name in {"gateway", "app", "qdrant", "embedding"}:
            limit = service_cpu_limit(service)
            if limit is None:
                fail(f"service {name} is missing an explicit CPU limit")
            if host_cpus is not None and limit > host_cpus:
                fail(
                    f"service {name} CPU limit {limit:g} exceeds "
                    f"Docker host capacity {host_cpus:g}"
                )

    gateway_tls = services["gateway-tls"]
    if gateway_tls.get("network_mode") != "none":
        fail("gateway initialization service must have networking disabled")
    if str(gateway_tls.get("user", "")) not in {"0", "0:0", "root"}:
        fail("gateway initialization must run as root only for bounded setup")
    if gateway_tls.get("restart") not in {"no", "none", False}:
        fail("gateway initialization service must be one-shot")
    staging_env = gateway_tls.get("environment", {}) or {}
    gateway_uid = str(staging_env.get("GATEWAY_UID", ""))
    gateway_gid = str(staging_env.get("GATEWAY_GID", ""))
    if not gateway_uid.isdigit() or int(gateway_uid) <= 0:
        fail("GATEWAY_UID must be a positive numeric non-root identity")
    if not gateway_gid.isdigit() or int(gateway_gid) <= 0:
        fail("GATEWAY_GID must be a positive numeric non-root identity")
    prepared_mount = mount_for_target(gateway_tls, "/prepared")
    if not prepared_mount or prepared_mount.get("type") != "volume":
        fail("gateway initialization must stage TLS in a Docker-managed volume")
    staged_state_mounts: dict[str, dict[str, Any]] = {}
    for state_target in ("/data", "/config"):
        state_mount = mount_for_target(gateway_tls, state_target)
        if (
            not state_mount
            or state_mount.get("type") != "volume"
            or state_mount.get("read_only") is True
        ):
            fail(
                "gateway initialization must mount writable Caddy state volume "
                f"{state_target}"
            )
        staged_state_mounts[state_target] = state_mount
    for source_target in ("/source/fullchain.pem", "/source/privkey.pem"):
        source_mount = mount_for_target(gateway_tls, source_target)
        if (
            not source_mount
            or source_mount.get("type") != "bind"
            or source_mount.get("read_only") is not True
        ):
            fail(f"TLS staging source {source_target} must be a read-only bind mount")

    gateway = services["gateway"]
    expected_gateway_user = f"{gateway_uid}:{gateway_gid}"
    if str(gateway.get("user", "")) != expected_gateway_user:
        fail(
            "gateway user must match the numeric owner prepared at initialization "
            f"({expected_gateway_user})"
        )
    if gateway.get("cap_add"):
        fail("gateway must not require Linux capabilities")
    gateway_ports = gateway.get("ports", []) or []
    port_map = {
        str(item.get("published")): str(item.get("target"))
        for item in gateway_ports
        if isinstance(item, dict)
    }
    expected_port_map = {"80": "8080", "443": "8443"}
    if port_map != expected_port_map:
        fail(
            "gateway must map host 80/443 to unprivileged container 8080/8443, "
            f"got {port_map}"
        )
    tls_mount = mount_for_target(gateway, "/etc/caddy/tls")
    if (
        not tls_mount
        or tls_mount.get("type") != "volume"
        or tls_mount.get("read_only") is not True
    ):
        fail("gateway must read TLS material from a read-only Docker volume")
    if mount_for_target(gateway, "/etc/caddy/tls/privkey.pem"):
        fail("gateway must not bind-mount the operator private-key file directly")
    for state_target, staged_state_mount in staged_state_mounts.items():
        gateway_state_mount = mount_for_target(gateway, state_target)
        if (
            not gateway_state_mount
            or gateway_state_mount.get("type") != "volume"
            or gateway_state_mount.get("read_only") is True
        ):
            fail(f"gateway requires writable persistent state at {state_target}")
        if gateway_state_mount.get("source") != staged_state_mount.get("source"):
            fail(
                "gateway initialization and runtime must share the same volume at "
                f"{state_target}"
            )
    if dependency_condition(gateway, "gateway-tls") != "service_completed_successfully":
        fail("gateway must wait for successful state and TLS initialization")
    if gateway.get("entrypoint") != ["/etc/caddy/tls/caddy"]:
        fail("gateway must execute the capability-free Caddy binary staged by initialization")

    for network_name in (
        "edutalent-edge",
        "edutalent-supabase-api",
        "edutalent-data",
        "edutalent-admin",
    ):
        matches = [
            value for value in networks.values() if value.get("name") == network_name
        ]
        if len(matches) != 1 or matches[0].get("internal") is not True:
            fail(f"network {network_name} must exist and be internal")

    gateway_env = gateway.get("environment", {}) or {}
    if not str(gateway_env.get("ADMIN_ALLOWED_CIDRS", "")).strip():
        fail("administration source-network allowlist is missing")

    functions_service = services.get("functions")
    if functions_service is not None:
        functions_profiles = functions_service.get("profiles", []) or []
        if functions_profiles != ["edge-functions"]:
            fail("Edge Functions must remain disabled behind the explicit profile")

    pooler_ulimits = services["supavisor"].get("ulimits", {}) or {}
    pooler_nofile = pooler_ulimits.get("nofile", {}) or {}
    if not isinstance(pooler_nofile, dict) or {
        str(pooler_nofile.get("soft")),
        str(pooler_nofile.get("hard")),
    } != {"100000"}:
        fail("Supavisor must receive a 100000 soft/hard nofile limit without extra capabilities")

    qdrant_service = services["qdrant"]
    qdrant_env = qdrant_service.get("environment", {}) or {}
    qdrant_key = qdrant_env.get("QDRANT__SERVICE__API_KEY", "")
    if not qdrant_key or "replace" in str(qdrant_key).lower():
        fail("Qdrant API key is missing or still a placeholder")
    if qdrant_service.get("healthcheck"):
        fail("Qdrant must not rely on unavailable in-image shell health tooling")

    migrate_service = services["migrate"]
    migrate_env = migrate_service.get("environment", {}) or {}
    migrate_user = parse_database_user(
        str(migrate_env.get("DATABASE_URL", "")), "migration DATABASE_URL"
    )
    if migrate_user != "postgres":
        fail("migration service must use the bootstrap postgres identity")

    database_access = services["database-access"]
    if database_access.get("restart") not in {"no", "none", False}:
        fail("database-access must be a one-shot service")
    if dependency_condition(database_access, "migrate") != "service_completed_successfully":
        fail("database-access must run only after migrations complete")
    access_env = database_access.get("environment", {}) or {}
    admin_user = parse_database_user(
        str(access_env.get("DATABASE_ADMIN_URL", "")),
        "database-access DATABASE_ADMIN_URL",
    )
    if admin_user != "postgres":
        fail("database-access must use the bootstrap postgres identity")
    app_role = str(access_env.get("DATABASE_APP_USER", ""))
    app_password = str(access_env.get("DATABASE_APP_PASSWORD", ""))
    if not app_role or app_role == "postgres":
        fail("database-access must configure a distinct application role")
    if len(app_password) < 32 or "replace" in app_password.lower():
        fail("database-access application password is missing or unsafe")

    app_service = services["app"]
    if dependency_condition(app_service, "database-access") != "service_completed_successfully":
        fail("app must wait for dedicated database role configuration")
    qdrant_dependency = dependency_condition(app_service, "qdrant")
    if qdrant_dependency != "service_started":
        fail("core app startup must treat Qdrant readiness as degradable")

    app_env = app_service.get("environment", {}) or {}
    if "DATABASE_ADMIN_URL" in app_env or "POSTGRES_PASSWORD" in app_env:
        fail("long-running app must not receive database bootstrap credentials")
    database_url = str(app_env.get("DATABASE_URL", ""))
    app_database_user = parse_database_user(database_url, "app DATABASE_URL")
    if app_database_user != app_role:
        fail("app DATABASE_URL must use the generated backend role")
    if app_database_user == "postgres":
        fail("long-running app must never connect as postgres")
    if str(app_env.get("DATABASE_APP_USER", "")) != app_role:
        fail("app role metadata must match its DATABASE_URL identity")
    if app_password not in database_url:
        fail("app DATABASE_URL must use the generated application credential")
    if app_env.get("SUPABASE_URL") != "http://kong:8000":
        fail("EduTalent server must use private-network Supabase Kong")
    if not str(app_env.get("SUPABASE_JWT_ISSUER", "")).endswith("/auth/v1"):
        fail("EduTalent self-hosted Supabase JWT issuer is missing")
    if not str(app_env.get("JWT_SECRET", "")).strip():
        fail("EduTalent legacy JWT secret is missing")
    if app_env.get("QDRANT_URL") != "http://qdrant:6334":
        fail("EduTalent must use private-network Qdrant")

    print("Rendered production Compose security invariants verified.")


if __name__ == "__main__":
    main()
