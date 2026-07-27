#!/usr/bin/env python3
"""Fail closed on production Compose exposure, privilege, and AI egress drift."""

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


def mount_for_target(service: dict[str, Any], target: str) -> dict[str, Any] | None:
    return next(
        (mount for mount in volume_mounts(service) if mount.get("target") == target),
        None,
    )


def dependencies(service: dict[str, Any]) -> dict[str, Any]:
    value = service.get("depends_on", {}) or {}
    return value if isinstance(value, dict) else {}


def dependency_condition(service: dict[str, Any], dependency: str) -> str | None:
    entry = dependencies(service).get(dependency)
    if isinstance(entry, dict):
        value = entry.get("condition")
        return str(value) if value is not None else None
    return None


def attached_network_names(service: dict[str, Any]) -> set[str]:
    attached = service.get("networks", {}) or {}
    return set(attached) if isinstance(attached, dict) else set(attached)


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


def require_exact_network(
    services: dict[str, dict[str, Any]], network: str, expected: set[str]
) -> None:
    actual = {
        name
        for name, service in services.items()
        if network in attached_network_names(service)
    }
    if actual != expected:
        fail(f"network {network} membership must be {sorted(expected)}, got {sorted(actual)}")


def require_no_capabilities(service: dict[str, Any], name: str) -> None:
    if service.get("cap_add"):
        fail(f"{name} must not receive Linux capabilities")
    cap_drop = {str(value).upper() for value in service.get("cap_drop", []) or []}
    if "ALL" not in cap_drop:
        fail(f"{name} must drop all Linux capabilities")


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
    if 'chown -R "$${GATEWAY_UID}:$${GATEWAY_GID}" /data /config' not in overlay_text:
        fail("gateway initialization must own persistent Caddy state volumes")
    if "chmod 0700 /data /config" not in overlay_text:
        fail("gateway initialization must restrict persistent Caddy state volumes")
    if "AI_GATEWAY_DEFAULT_SCHOOL_ID" in overlay_text:
        fail("production topology must not contain a shared AI tenant fallback")
    if "AI_ALLOWED_EMBEDDING_BASE_URLS" in overlay_text or "AI_ALLOWED_LLM_BASE_URLS" in overlay_text:
        fail("production topology must not contain operator-defined AI allowlists")

    document = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    services = document.get("services")
    networks = document.get("networks")
    if not isinstance(services, dict) or not isinstance(networks, dict):
        fail("rendered document is missing services or networks")

    required_services = {
        "gateway-tls",
        "gateway",
        "app",
        "ai-gateway",
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

    allowed_external_memberships = {
        ("gateway", "ingress"),
        ("ai-gateway", "ai_egress"),
    }
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

        for network_key in attached_network_names(service):
            network = networks.get(network_key)
            if not isinstance(network, dict):
                fail(f"service {name} references missing network {network_key}")
            if network.get("internal") is True:
                continue
            if (name, network_key) not in allowed_external_memberships:
                fail(f"service {name} is attached to non-internal network {network_key}")

        if name in {"gateway", "app", "ai-gateway", "qdrant", "embedding"}:
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
            fail(f"gateway initialization needs writable state at {state_target}")
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
        fail(f"gateway user must match initialization owner {expected_gateway_user}")
    require_no_capabilities(gateway, "gateway")
    gateway_ports = gateway.get("ports", []) or []
    port_map = {
        str(item.get("published")): str(item.get("target"))
        for item in gateway_ports
        if isinstance(item, dict)
    }
    if port_map != {"80": "8080", "443": "8443"}:
        fail(f"gateway port map is invalid: {port_map}")
    if any(
        str(item.get("host_ip", "")) != "0.0.0.0"
        for item in gateway_ports
        if isinstance(item, dict)
    ):
        fail("gateway host ports must bind explicitly on all IPv4 interfaces")
    tls_mount = mount_for_target(gateway, "/etc/caddy/tls")
    if (
        not tls_mount
        or tls_mount.get("type") != "volume"
        or tls_mount.get("read_only") is not True
    ):
        fail("gateway must read TLS from a read-only Docker volume")
    if mount_for_target(gateway, "/etc/caddy/tls/privkey.pem"):
        fail("gateway must not bind-mount the operator private key directly")
    for target, staged_mount in staged_state_mounts.items():
        runtime_mount = mount_for_target(gateway, target)
        if (
            not runtime_mount
            or runtime_mount.get("type") != "volume"
            or runtime_mount.get("read_only") is True
            or runtime_mount.get("source") != staged_mount.get("source")
        ):
            fail(f"gateway state volume mismatch at {target}")
    if dependency_condition(gateway, "gateway-tls") != "service_completed_successfully":
        fail("gateway must wait for successful TLS initialization")
    if gateway.get("entrypoint") != ["/etc/caddy/tls/caddy"]:
        fail("gateway must execute the staged capability-free Caddy binary")

    for network_name in (
        "edutalent-edge",
        "edutalent-supabase-api",
        "edutalent-data",
        "edutalent-admin",
        "edutalent-ai-internal",
    ):
        matches = [value for value in networks.values() if value.get("name") == network_name]
        if len(matches) != 1 or matches[0].get("internal") is not True:
            fail(f"network {network_name} must exist and be internal")
    non_internal = {
        key
        for key, value in networks.items()
        if not isinstance(value, dict) or value.get("internal") is not True
    }
    if non_internal != {"ingress", "ai_egress"}:
        fail(f"only ingress and ai_egress may be non-internal, got {sorted(non_internal)}")
    for key, expected_name in (
        ("ingress", "edutalent-ingress"),
        ("ai_egress", "edutalent-ai-egress"),
    ):
        network = networks.get(key, {})
        if network.get("name") != expected_name or network.get("driver") != "bridge":
            fail(f"{key} must be the named bridge {expected_name}")

    require_exact_network(services, "ingress", {"gateway"})
    require_exact_network(services, "ai_egress", {"ai-gateway"})
    require_exact_network(
        services,
        "ai_internal",
        {"app", "ai-gateway"} | ({"embedding"} if "embedding" in services else set()),
    )

    auth_env = services["auth"].get("environment", {}) or {}
    for key, expected in {
        "GOTRUE_DISABLE_SIGNUP": "true",
        "GOTRUE_EXTERNAL_EMAIL_ENABLED": "true",
        "GOTRUE_EXTERNAL_ANONYMOUS_USERS_ENABLED": "false",
        "GOTRUE_EXTERNAL_PHONE_ENABLED": "false",
    }.items():
        actual = str(auth_env.get(key, "")).lower()
        if actual != expected:
            fail(f"Supabase Auth setting {key} must be {expected}, got {actual or 'missing'}")

    pooler_nofile = (services["supavisor"].get("ulimits", {}) or {}).get("nofile", {}) or {}
    if {
        str(pooler_nofile.get("soft")),
        str(pooler_nofile.get("hard")),
    } != {"100000"}:
        fail("Supavisor must receive a 100000 soft/hard nofile limit")

    qdrant = services["qdrant"]
    qdrant_env = qdrant.get("environment", {}) or {}
    qdrant_key = str(qdrant_env.get("QDRANT__SERVICE__API_KEY", ""))
    if not qdrant_key or "replace" in qdrant_key.lower():
        fail("Qdrant API key is missing or still a placeholder")
    if qdrant.get("healthcheck"):
        fail("Qdrant must not rely on unavailable in-image shell health tooling")

    migrate = services["migrate"]
    migrate_user = parse_database_user(
        str((migrate.get("environment", {}) or {}).get("DATABASE_URL", "")),
        "migration DATABASE_URL",
    )
    if migrate_user != "postgres":
        fail("migration service must use the bootstrap postgres identity")

    database_access = services["database-access"]
    if database_access.get("restart") not in {"no", "none", False}:
        fail("database-access must be one-shot")
    if dependency_condition(database_access, "migrate") != "service_completed_successfully":
        fail("database-access must wait for migrations")
    access_env = database_access.get("environment", {}) or {}
    if parse_database_user(str(access_env.get("DATABASE_ADMIN_URL", "")), "admin URL") != "postgres":
        fail("database-access must use postgres")
    app_role = str(access_env.get("DATABASE_APP_USER", ""))
    app_password = str(access_env.get("DATABASE_APP_PASSWORD", ""))
    if not app_role or app_role == "postgres":
        fail("database-access must configure a distinct application role")
    if len(app_password) < 32 or "replace" in app_password.lower():
        fail("application database password is missing or unsafe")

    app = services["app"]
    app_env = app.get("environment", {}) or {}
    if dependency_condition(app, "database-access") != "service_completed_successfully":
        fail("app must wait for role configuration")
    if "ai-gateway" in dependencies(app):
        fail("app must start independently of AI gateway health")
    if dependency_condition(app, "qdrant") != "service_started":
        fail("Qdrant readiness must remain degradable")
    if "DATABASE_ADMIN_URL" in app_env or "POSTGRES_PASSWORD" in app_env:
        fail("app must not receive database bootstrap credentials")
    app_database_user = parse_database_user(str(app_env.get("DATABASE_URL", "")), "app URL")
    if app_database_user != app_role or app_database_user == "postgres":
        fail("app must use the generated backend role")
    if app_password not in str(app_env.get("DATABASE_URL", "")):
        fail("app DATABASE_URL must use the generated credential")
    if app_env.get("SUPABASE_URL") != "http://kong:8000":
        fail("app must use private Kong")
    if app_env.get("QDRANT_URL") != "http://qdrant:6334":
        fail("app must use private Qdrant")
    if app_env.get("AI_GATEWAY_URL") != "http://ai-gateway:8090":
        fail("app must use only the internal AI gateway")
    forbidden_app_ai_keys = {
        "OPENAI_API_KEY",
        "LLM_API_KEY",
        "AI_EMBEDDING_BASE_URL",
        "AI_LLM_BASE_URL",
        "AI_ALLOWED_EMBEDDING_BASE_URLS",
        "AI_ALLOWED_LLM_BASE_URLS",
        "AI_GATEWAY_DEFAULT_SCHOOL_ID",
    }
    leaked = sorted(forbidden_app_ai_keys & set(app_env))
    if leaked:
        fail(f"provider credentials/destinations or shared tenant identity reached app: {leaked}")

    ai_gateway = services["ai-gateway"]
    ai_env = ai_gateway.get("environment", {}) or {}
    if ai_gateway.get("command") != ["ai-gateway"]:
        fail("AI gateway must run the dedicated gateway binary")
    if str(ai_gateway.get("user", "")) in {"", "0", "0:0", "root"}:
        fail("AI gateway must run as a numeric non-root identity")
    require_no_capabilities(ai_gateway, "AI gateway")
    if ai_gateway.get("read_only") is not True:
        fail("AI gateway root filesystem must be read-only")
    if not ai_gateway.get("healthcheck"):
        fail("AI gateway must expose a provider-independent local health check")
    for forbidden in (
        "DATABASE_URL",
        "DATABASE_ADMIN_URL",
        "POSTGRES_PASSWORD",
        "QDRANT_API_KEY",
        "SUPABASE_SECRET_KEY",
        "AI_GATEWAY_DEFAULT_SCHOOL_ID",
        "AI_ALLOWED_EMBEDDING_BASE_URLS",
        "AI_ALLOWED_LLM_BASE_URLS",
    ):
        if forbidden in ai_env:
            fail(f"AI gateway must not receive {forbidden}")
    if ai_env.get("AI_GATEWAY_INTERNAL_TOKEN") != app_env.get("AI_GATEWAY_INTERNAL_TOKEN"):
        fail("app and AI gateway internal credentials must match")
    token = str(ai_env.get("AI_GATEWAY_INTERNAL_TOKEN", ""))
    if len(token) < 24 or "replace" in token.lower():
        fail("AI gateway internal credential is missing or unsafe")

    profile = str(app_env.get("EMBEDDING_PROFILE", ""))
    profile_contracts = {
        "local-bge-v1": (
            "offline",
            "BAAI/bge-small-en-v1.5",
            "384",
            "edutalent_local_bge_v1",
        ),
        "openai-v1": (
            "connected",
            "text-embedding-3-small",
            "1536",
            "edutalent_openai_v1",
        ),
    }
    if profile not in profile_contracts:
        fail(f"unsupported production embedding profile {profile!r}")
    expected_mode, expected_model, expected_size, expected_collection = profile_contracts[profile]
    observed = (
        str(ai_env.get("AI_GATEWAY_MODE", "")),
        str(app_env.get("EMBEDDING_MODEL", "")),
        str(app_env.get("EMBEDDING_VECTOR_SIZE", "")),
        str(app_env.get("QDRANT_COLLECTION", "")),
    )
    expected = (expected_mode, expected_model, expected_size, expected_collection)
    if observed != expected:
        fail(f"embedding profile contract mismatch: expected {expected}, got {observed}")
    if str(app_env.get("QDRANT_VECTOR_SIZE", "")) != expected_size:
        fail("Qdrant vector size must match the active embedding profile")
    if str(ai_env.get("EMBEDDING_PROFILE", "")) != profile:
        fail("app and AI gateway embedding profiles must match")

    if expected_mode == "connected":
        for key in ("OPENAI_API_KEY", "LLM_API_KEY"):
            value = str(ai_env.get(key, ""))
            if len(value) < 24 or "replace" in value.lower():
                fail(f"connected mode requires safe {key}")
        if ai_env.get("AI_EMBEDDING_BASE_URL") != "https://api.openai.com/v1/":
            fail("connected embeddings must use the exact approved OpenAI origin")
        if ai_env.get("AI_LLM_BASE_URL") != "https://api.deepseek.com/v1/":
            fail("connected LLM requests must use the exact approved LLM origin")
    else:
        if ai_env.get("AI_EMBEDDING_BASE_URL") != "http://embedding:80/v1/":
            fail("offline profile must use the internal TEI service")
        if "embedding" not in services:
            fail("offline profile must render the local TEI service")

    print("Rendered production Compose security and AI egress invariants verified.")


if __name__ == "__main__":
    main()
