#!/usr/bin/env python3
"""Fail-closed DNS/port/time checks for a pre-start EduTalent target host."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import socket
import subprocess
from pathlib import Path
from typing import Any

PRODUCTION_DIR = Path(__file__).resolve().parent
DEFAULT_BASELINE = PRODUCTION_DIR / "host-baseline.json"
DEFAULT_APP_ENV = PRODUCTION_DIR / ".env.edutalent"


def fail(message: str) -> None:
    raise SystemExit(f"host network preflight failed: {message}")


def read_env(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.exists():
        return values
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key] = value
    return values


def dns_resolves(name: str) -> bool:
    try:
        return bool(socket.getaddrinfo(name, None, type=socket.SOCK_STREAM))
    except socket.gaierror:
        return False


def listening_ports() -> set[int]:
    ports: set[int] = set()
    for filename in ("/proc/net/tcp", "/proc/net/tcp6"):
        path = Path(filename)
        if not path.exists():
            continue
        for line in path.read_text(encoding="utf-8").splitlines()[1:]:
            fields = line.split()
            if len(fields) < 4 or fields[3] != "0A":
                continue
            try:
                port_hex = fields[1].rsplit(":", 1)[1]
                ports.add(int(port_hex, 16))
            except (ValueError, IndexError):
                continue
    return ports


def command_output(command: list[str]) -> str | None:
    try:
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    return completed.stdout.strip() if completed.returncode == 0 else None


def ntp_synchronized() -> bool | None:
    value = command_output(["timedatectl", "show", "-p", "NTPSynchronized", "--value"])
    if value is None:
        return None
    lowered = value.lower()
    if lowered in {"yes", "true", "1"}:
        return True
    if lowered in {"no", "false", "0"}:
        return False
    return None


def chrony_skew_seconds() -> float | None:
    if not shutil.which("chronyc"):
        return None
    output = command_output(["chronyc", "tracking"])
    if not output:
        return None
    for line in output.splitlines():
        if line.strip().lower().startswith("system time"):
            match = re.search(r":\s*([+-]?[0-9.]+)\s+seconds", line, flags=re.I)
            if match:
                try:
                    return abs(float(match.group(1)))
                except ValueError:
                    return None
    return None


def collect_live_facts(app_env: Path, required_ports: list[int]) -> dict[str, Any]:
    env = read_env(app_env)
    names = [env.get(key, "") for key in ("APP_DOMAIN", "SUPABASE_DOMAIN", "ADMIN_DOMAIN")]
    occupied = listening_ports()
    return {
        "app_env_present": app_env.exists(),
        "domains": {name: dns_resolves(name) for name in names if name},
        "configured_domain_count": len([name for name in names if name]),
        "required_ports": required_ports,
        "occupied_required_ports": sorted(port for port in required_ports if port in occupied),
        "ntp_synchronized": ntp_synchronized(),
        "clock_skew_seconds": chrony_skew_seconds(),
    }


def evaluate(baseline: dict[str, Any], facts: dict[str, Any]) -> dict[str, Any]:
    automatic: list[dict[str, str]] = []
    manual: list[dict[str, str]] = []

    def check(identifier: str, condition: bool, detail: str) -> None:
        automatic.append(
            {"id": identifier, "status": "pass" if condition else "fail", "detail": detail}
        )

    def pending(identifier: str, detail: str) -> None:
        manual.append({"id": identifier, "status": "manual", "detail": detail})

    network = baseline["network"]
    time_policy = baseline["time"]
    expected_dns_count = int(network["dns_names_required"])
    configured_count = int(facts.get("configured_domain_count") or 0)
    check(
        "dns-domain-count",
        configured_count == expected_dns_count,
        f"configured_domain_count={configured_count}; required={expected_dns_count}",
    )
    domains = facts.get("domains") or {}
    unresolved = sorted(name for name, resolved in domains.items() if not resolved)
    check(
        "dns-resolution",
        configured_count == expected_dns_count and not unresolved,
        f"unresolved={unresolved}",
    )

    expected_ports = sorted(int(value) for value in network["required_public_tcp_ports"])
    fact_ports = sorted(int(value) for value in facts.get("required_ports") or [])
    check(
        "required-port-contract",
        fact_ports == expected_ports,
        f"observed_required_ports={fact_ports}; baseline={expected_ports}",
    )
    occupied = sorted(int(value) for value in facts.get("occupied_required_ports") or [])
    check(
        "prestart-public-ports-free",
        not occupied,
        f"occupied_required_ports={occupied}; run this check before production-up",
    )

    ntp = facts.get("ntp_synchronized")
    if ntp is False:
        check("time-synchronization", False, "NTPSynchronized=false")
    elif ntp is True:
        check("time-synchronization", True, "NTPSynchronized=true")
    else:
        pending(
            "time-synchronization",
            "NTP synchronization state was not observable; record the approved time source manually.",
        )

    skew = facts.get("clock_skew_seconds")
    if skew is None:
        pending(
            "clock-skew",
            "Quantified clock skew was not observable (for example chronyc tracking unavailable); record measured skew and source manually.",
        )
    else:
        maximum = float(time_policy["maximum_clock_skew_seconds"])
        check(
            "clock-skew",
            abs(float(skew)) <= maximum,
            f"clock_skew_seconds={float(skew):.6f}; maximum={maximum:g}",
        )

    failed = [item["id"] for item in automatic if item["status"] == "fail"]
    pending_ids = [item["id"] for item in manual]
    return {
        "schema_version": 1,
        "baseline_id": baseline["baseline_id"],
        "automatic_status": "fail" if failed else "pass",
        "manual_status": "pending" if pending_ids else "pass",
        "automatic_checks": automatic,
        "manual_checks": manual,
        "failed_check_ids": failed,
        "pending_manual_check_ids": pending_ids,
        "facts": facts,
        "note": "This is a pre-start DNS/port/time check. Firewall policy remains separate target-host evidence.",
    }


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        fail(f"{path} must contain a JSON object")
    return value


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    parser.add_argument("--app-env", type=Path, default=DEFAULT_APP_ENV)
    parser.add_argument("--facts", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    baseline = load_json(args.baseline)
    required_ports = [int(value) for value in baseline["network"]["required_public_tcp_ports"]]
    facts = load_json(args.facts) if args.facts else collect_live_facts(args.app_env, required_ports)
    result = evaluate(baseline, facts)
    text = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8")
    print(text, end="")
    if result["automatic_status"] != "pass":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
