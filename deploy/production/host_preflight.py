#!/usr/bin/env python3
"""Collect and evaluate the supported EduTalent production-host baseline."""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
from pathlib import Path
from typing import Any


PRODUCTION_DIR = Path(__file__).resolve().parent
DEFAULT_BASELINE = PRODUCTION_DIR / "host-baseline.json"


def fail(message: str) -> None:
    print(f"host preflight failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def parse_version(value: str) -> tuple[int, ...]:
    match = re.match(r"^\s*v?(\d+(?:\.\d+)*)", value or "")
    if not match:
        raise ValueError(f"unable to parse version {value!r}")
    return tuple(int(part) for part in match.group(1).split("."))


def version_at_least(actual: str, minimum: str) -> bool:
    left = list(parse_version(actual))
    right = list(parse_version(minimum))
    width = max(len(left), len(right))
    left.extend([0] * (width - len(left)))
    right.extend([0] * (width - len(right)))
    return tuple(left) >= tuple(right)


def command_output(command: list[str]) -> str | None:
    try:
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=15,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if completed.returncode != 0:
        return None
    return completed.stdout.strip()


def read_os_release() -> dict[str, str]:
    values: dict[str, str] = {}
    path = Path("/etc/os-release")
    if not path.exists():
        return values
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" not in line or line.lstrip().startswith("#"):
            continue
        key, value = line.split("=", 1)
        values[key] = value.strip().strip('"')
    return values


def mode_string(path: Path) -> str | None:
    try:
        return f"{stat.S_IMODE(path.stat().st_mode):04o}"
    except OSError:
        return None


def root_filesystem_type() -> str | None:
    value = command_output(["findmnt", "-n", "-o", "FSTYPE", "/"])
    return value.splitlines()[0].strip() if value else None


def docker_rootless() -> bool | None:
    value = command_output(["docker", "info", "--format", "{{json .SecurityOptions}}"])
    if not value:
        return None
    try:
        options = json.loads(value)
    except json.JSONDecodeError:
        return None
    return any("rootless" in str(item).lower() for item in options)


def ntp_synchronized() -> bool | None:
    value = command_output(["timedatectl", "show", "-p", "NTPSynchronized", "--value"])
    if value is None:
        return None
    lowered = value.strip().lower()
    if lowered in {"yes", "true", "1"}:
        return True
    if lowered in {"no", "false", "0"}:
        return False
    return None


def firewall_detected() -> bool | None:
    if shutil.which("ufw"):
        output = command_output(["ufw", "status"])
        if output is not None:
            return "status: active" in output.lower()
    if shutil.which("nft"):
        try:
            completed = subprocess.run(
                ["nft", "list", "ruleset"],
                check=False,
                capture_output=True,
                text=True,
                timeout=15,
            )
        except (OSError, subprocess.TimeoutExpired):
            return None
        if completed.returncode == 0:
            return bool(completed.stdout.strip())
        if "operation not permitted" in completed.stderr.lower():
            return None
    return None


def memory_bytes() -> int:
    path = Path("/proc/meminfo")
    if not path.exists():
        return 0
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1]) * 1024
    return 0


def collect_live_facts() -> dict[str, Any]:
    os_release = read_os_release()
    root_usage = shutil.disk_usage("/")
    vfs = os.statvfs("/")
    docker_version = command_output(
        ["docker", "version", "--format", "{{.Server.Version}}"]
    )
    compose_version = command_output(["docker", "compose", "version", "--short"])

    backup_dir_raw = os.environ.get("EDUTALENT_BACKUP_DIR", "")
    passphrase_raw = os.environ.get("EDUTALENT_BACKUP_PASSPHRASE_FILE", "")
    backup_dir = Path(backup_dir_raw).expanduser() if backup_dir_raw else None
    passphrase = Path(passphrase_raw).expanduser() if passphrase_raw else None
    production_device = PRODUCTION_DIR.stat().st_dev
    backup_device: int | None = None
    backup_free_bytes: int | None = None
    backup_writable: bool | None = None
    if backup_dir and backup_dir.exists():
        try:
            backup_device = backup_dir.stat().st_dev
            backup_free_bytes = shutil.disk_usage(backup_dir).free
            backup_writable = os.access(backup_dir, os.W_OK)
        except OSError:
            pass

    return {
        "system": platform.system().lower(),
        "os_id": os_release.get("ID", "").lower(),
        "os_version_id": os_release.get("VERSION_ID", ""),
        "architecture": platform.machine(),
        "kernel": platform.release(),
        "cpu_count": os.cpu_count() or 0,
        "memory_bytes": memory_bytes(),
        "root_free_bytes": root_usage.free,
        "root_free_inodes": int(vfs.f_favail),
        "root_fstype": root_filesystem_type(),
        "docker_engine_version": docker_version,
        "docker_compose_version": compose_version,
        "docker_rootless": docker_rootless(),
        "operator_uid": os.geteuid(),
        "ntp_synchronized": ntp_synchronized(),
        "firewall_detected": firewall_detected(),
        "production_device": production_device,
        "backup_dir": str(backup_dir.resolve()) if backup_dir and backup_dir.exists() else backup_dir_raw or None,
        "backup_device": backup_device,
        "backup_free_bytes": backup_free_bytes,
        "backup_writable": backup_writable,
        "backup_passphrase_file": str(passphrase.resolve()) if passphrase and passphrase.exists() else passphrase_raw or None,
        "backup_passphrase_mode": mode_string(passphrase) if passphrase else None,
    }


def add_check(
    results: list[dict[str, str]], identifier: str, ok: bool, detail: str
) -> None:
    results.append(
        {"id": identifier, "status": "pass" if ok else "fail", "detail": detail}
    )


def add_manual(
    results: list[dict[str, str]],
    identifier: str,
    detail: str,
    *,
    satisfied: bool = False,
) -> None:
    results.append(
        {
            "id": identifier,
            "status": "pass" if satisfied else "manual",
            "detail": detail,
        }
    )


def evaluate(
    baseline: dict[str, Any], facts: dict[str, Any], *, require_operations: bool
) -> dict[str, Any]:
    automatic: list[dict[str, str]] = []
    manual_checks: list[dict[str, str]] = []
    os_policy = baseline["operating_system"]
    runtime = baseline["container_runtime"]
    capacity = baseline["capacity"]
    filesystems = baseline["filesystems"]
    operations = baseline["operations"]

    add_check(automatic, "linux-host", facts.get("system") == "linux", f"system={facts.get('system')!r}")
    add_check(
        automatic,
        "supported-os",
        facts.get("os_id") == os_policy["id"]
        and facts.get("os_version_id") == os_policy["version_id"],
        f"os={facts.get('os_id')} {facts.get('os_version_id')}; required={os_policy['id']} {os_policy['version_id']}",
    )
    add_check(
        automatic,
        "supported-architecture",
        facts.get("architecture") in os_policy["architectures"],
        f"architecture={facts.get('architecture')}; supported={','.join(os_policy['architectures'])}",
    )
    try:
        kernel_ok = version_at_least(str(facts.get("kernel") or ""), os_policy["minimum_kernel"])
    except ValueError:
        kernel_ok = False
    add_check(automatic, "minimum-kernel", kernel_ok, f"kernel={facts.get('kernel')}; minimum={os_policy['minimum_kernel']}")
    add_check(automatic, "minimum-cpu", int(facts.get("cpu_count") or 0) >= capacity["minimum_cpu_cores"], f"cpus={facts.get('cpu_count')}; minimum={capacity['minimum_cpu_cores']}")
    add_check(automatic, "minimum-memory", int(facts.get("memory_bytes") or 0) >= capacity["minimum_memory_bytes"], f"memory_bytes={facts.get('memory_bytes')}; minimum={capacity['minimum_memory_bytes']}")
    add_check(automatic, "minimum-root-free-space", int(facts.get("root_free_bytes") or 0) >= capacity["minimum_root_free_bytes"], f"root_free_bytes={facts.get('root_free_bytes')}; minimum={capacity['minimum_root_free_bytes']}")
    add_check(automatic, "minimum-root-free-inodes", int(facts.get("root_free_inodes") or 0) >= capacity["minimum_root_free_inodes"], f"root_free_inodes={facts.get('root_free_inodes')}; minimum={capacity['minimum_root_free_inodes']}")
    add_check(automatic, "supported-root-filesystem", facts.get("root_fstype") in filesystems["allowed_root_types"], f"root_fstype={facts.get('root_fstype')}; allowed={','.join(filesystems['allowed_root_types'])}")

    docker_engine = facts.get("docker_engine_version")
    try:
        docker_engine_ok = bool(docker_engine) and version_at_least(str(docker_engine), runtime["minimum_docker_engine"])
    except ValueError:
        docker_engine_ok = False
    add_check(automatic, "minimum-docker-engine", docker_engine_ok, f"docker_engine={docker_engine}; minimum={runtime['minimum_docker_engine']}")

    compose = facts.get("docker_compose_version")
    try:
        compose_ok = bool(compose) and version_at_least(str(compose), runtime["minimum_docker_compose"])
    except ValueError:
        compose_ok = False
    add_check(automatic, "minimum-docker-compose", compose_ok, f"docker_compose={compose}; minimum={runtime['minimum_docker_compose']}")
    add_check(automatic, "unprivileged-operator", int(facts.get("operator_uid", 0)) != 0, f"operator_uid={facts.get('operator_uid')}")

    ntp = facts.get("ntp_synchronized")
    if ntp is None:
        add_manual(manual_checks, "time-synchronization", "NTP synchronization could not be verified automatically; record clock-skew/time-source evidence on the target host.")
    else:
        add_check(automatic, "time-synchronization", ntp is True, f"ntp_synchronized={ntp}")

    if facts.get("docker_rootless") is True:
        add_manual(manual_checks, "docker-daemon-mode", "Docker rootless mode detected.", satisfied=True)
    else:
        add_manual(manual_checks, "docker-daemon-mode", "Rootless Docker was not proven. A rootful daemon is permitted only with the tailored CIS/host security review and restricted Docker-socket access.")

    firewall = facts.get("firewall_detected")
    add_manual(
        manual_checks,
        "host-firewall",
        "Host firewall policy detected automatically." if firewall is True else "Record target-host firewall evidence proving only approved ingress and restricted administration; absence of tool visibility is not a pass.",
        satisfied=firewall is True,
    )
    add_manual(manual_checks, "data-at-rest-encryption", "Record filesystem/block-device encryption evidence for all data-bearing application, database, document, WAL and backup media.")

    if require_operations:
        backup_dir = facts.get("backup_dir")
        passphrase = facts.get("backup_passphrase_file")
        add_check(automatic, "backup-directory-configured", bool(backup_dir), f"backup_dir={backup_dir!r}")
        add_check(automatic, "backup-directory-writable", facts.get("backup_writable") is True, f"backup_writable={facts.get('backup_writable')}")
        add_check(
            automatic,
            "backup-separate-filesystem",
            bool(facts.get("backup_device")) and facts.get("backup_device") != facts.get("production_device"),
            f"production_device={facts.get('production_device')}; backup_device={facts.get('backup_device')}",
        )
        add_check(automatic, "backup-free-space", int(facts.get("backup_free_bytes") or 0) >= operations["minimum_backup_free_bytes"], f"backup_free_bytes={facts.get('backup_free_bytes')}; minimum={operations['minimum_backup_free_bytes']}")
        add_check(automatic, "backup-passphrase-configured", bool(passphrase), f"backup_passphrase_file={passphrase!r}")
        add_check(automatic, "backup-passphrase-mode", str(facts.get("backup_passphrase_mode") or "") in operations["backup_passphrase_allowed_modes"], f"backup_passphrase_mode={facts.get('backup_passphrase_mode')}; allowed={operations['backup_passphrase_allowed_modes']}")
        add_manual(manual_checks, "backup-passphrase-escrow", "Record evidence that passphrase escrow is separate from encrypted backup media.")
        add_manual(manual_checks, "off-host-copy", "Record encrypted off-host backup and WAL copy location, verification and retention.")

    failed = [item for item in automatic if item["status"] == "fail"]
    pending_manual = [item for item in manual_checks if item["status"] == "manual"]
    return {
        "schema_version": 1,
        "baseline_id": baseline["baseline_id"],
        "automatic_status": "fail" if failed else "pass",
        "manual_status": "pending" if pending_manual else "pass",
        "require_operations": require_operations,
        "facts": facts,
        "automatic_checks": automatic,
        "manual_checks": manual_checks,
        "failed_check_ids": [item["id"] for item in failed],
        "pending_manual_check_ids": [item["id"] for item in pending_manual],
        "availability": baseline["availability"],
        "note": "Automatic PASS is not target-host acceptance. Manual evidence and the replacement-host qualification record remain required.",
    }


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        fail(f"{path} must contain a JSON object")
    return value


def validate_baseline(baseline: dict[str, Any]) -> None:
    required = {
        "schema_version",
        "baseline_id",
        "operating_system",
        "container_runtime",
        "capacity",
        "filesystems",
        "time",
        "network",
        "operations",
        "availability",
        "security_benchmarks",
    }
    missing = sorted(required - set(baseline))
    if missing:
        fail(f"baseline is missing required keys: {', '.join(missing)}")
    if baseline["schema_version"] != 1:
        fail(f"unsupported baseline schema_version={baseline['schema_version']}")
    if baseline["availability"].get("high_availability") is not False:
        fail("the first-release baseline must explicitly state high_availability=false")
    if baseline["availability"].get("architecture") != "single-node":
        fail("the first-release baseline must explicitly identify the single-node topology")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    parser.add_argument("--facts", type=Path, help="Evaluate deterministic facts JSON instead of probing the live host.")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--require-operations", action="store_true", help="Require separate backup filesystem/passphrase checks.")
    parser.add_argument("--definition-only", action="store_true", help="Validate the baseline schema without probing a host.")
    args = parser.parse_args()

    baseline = load_json(args.baseline)
    validate_baseline(baseline)
    if args.definition_only:
        payload = {
            "schema_version": 1,
            "baseline_id": baseline["baseline_id"],
            "definition_status": "pass",
            "note": "Definition validation is not target-host acceptance.",
        }
        text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(text, encoding="utf-8")
        print(text, end="")
        return

    facts = load_json(args.facts) if args.facts else collect_live_facts()
    payload = evaluate(baseline, facts, require_operations=args.require_operations)
    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8")
    print(text, end="")
    if payload["automatic_status"] != "pass":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
