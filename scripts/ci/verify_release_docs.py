#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RELEASE = ROOT / "docs" / "release"
CAPABILITIES = RELEASE / "product-capabilities.json"
ENDPOINTS = ROOT / "packages" / "api" / "endpoint_authorization_manifest.psv"
README = ROOT / "README.md"

REQUIRED_DOCS = (
    "README.md",
    "feature-matrix.md",
    "operator-manual-v1.0.md",
    "guide-administrators.md",
    "guide-teacher.md",
    "guide-parent.md",
    "guide-student.md",
    "api-security-inventory.md",
    "privacy-governance-draft.md",
    "security-organization.md",
    "procurement-security-questionnaire.md",
    "support-service-definition.md",
    "contract-feature-schedule.md",
    "customer-terms-draft.md",
    "documentation-reconciliation.md",
)

REQUIRED_ENABLED_ENDPOINTS = {
    "assignments/my_assignments": "StudentOnly",
    "submissions/submit": "StudentOnly",
    "teacher/submissions/grade": "TeacherOnly",
    "classes/student/grades": "StudentOnly",
    "parent/child/grades": "ParentOnly",
}

SECRET_PATTERNS = {
    "private key": re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
    "GitHub token": re.compile(r"\bgh[pousr]_[A-Za-z0-9_]{20,}\b"),
    "AWS access key": re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
    "provider key": re.compile(r"\b(?:sk|rk)-[A-Za-z0-9_-]{24,}\b"),
}
EMAIL = re.compile(r"\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b", re.I)
LINK = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
DRIFT_DOCS = (
    ROOT / "README.md",
    ROOT / "deploy" / "production" / "README.md",
    ROOT / "docs" / "security" / "production-threat-model.md",
)
STALE_ARCHITECTURE = (
    "intentional `BYPASSRLS`",
    "intentionally has `BYPASSRLS`",
    "future AI gateway",
    "only the future AI gateway",
    "issue #8 tracks",
)


def fail(message: str) -> None:
    raise AssertionError(message)


def read(path: Path) -> str:
    if not path.is_file():
        fail(f"required file missing: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def disabled_capabilities() -> list[str]:
    payload = json.loads(read(CAPABILITIES))
    if not isinstance(payload, dict):
        fail("product-capabilities.json must be an object")
    disabled: list[str] = []
    for key, value in payload.items():
        if isinstance(value, bool):
            if value is False:
                disabled.append(key)
        elif isinstance(value, dict):
            enabled = value.get("enabled")
            if enabled is False:
                disabled.append(key)
    if not disabled:
        fail("capability manifest exposed no disabled capabilities; verify schema")
    return sorted(disabled)


def endpoint_policies() -> dict[str, str]:
    lines = read(ENDPOINTS).splitlines()
    header = lines[0].split("|")
    for required in ("endpoint", "policy"):
        if required not in header:
            fail(f"endpoint manifest missing column: {required}")
    endpoint_i = header.index("endpoint")
    policy_i = header.index("policy")
    result: dict[str, str] = {}
    for line in lines[1:]:
        if not line.strip():
            continue
        cols = line.split("|")
        if len(cols) != len(header):
            fail(f"malformed endpoint manifest row: {line}")
        result[cols[endpoint_i]] = cols[policy_i]
    return result


def verify_feature_truth() -> None:
    matrix = read(RELEASE / "feature-matrix.md")
    for key in disabled_capabilities():
        row = re.compile(rf"^\|\s*`{re.escape(key)}`\s*\|.*\|\s*Disabled\s*\|", re.M)
        if not row.search(matrix):
            fail(f"disabled capability {key!r} is not explicitly Disabled in feature-matrix.md")

    policies = endpoint_policies()
    for endpoint, expected_policy in REQUIRED_ENABLED_ENDPOINTS.items():
        actual = policies.get(endpoint)
        if actual != expected_policy:
            fail(f"feature matrix enabled proof changed: {endpoint} policy={actual!r}, expected {expected_policy!r}")

    if "endpoint_authorization_manifest.psv" not in matrix:
        fail("feature matrix must cite endpoint_authorization_manifest.psv")

    inventory = read(RELEASE / "api-security-inventory.md")
    if "policy is `Disabled`" not in inventory:
        fail("API inventory must explain Disabled endpoint rows")


def verify_required_language() -> None:
    root_readme = read(README)
    stale = "That role intentionally has `BYPASSRLS`"
    if stale in root_readme or "tracked in issue #8" in root_readme:
        fail("root README still contains the retired BYPASSRLS architecture")
    for required in ("NOBYPASSRLS", "transaction-scoped"):
        if required not in root_readme:
            fail(f"root README must describe current {required} database boundary")

    privacy = read(RELEASE / "privacy-governance-draft.md")
    for phrase in (
        "Not legal advice",
        "controller/processor",
        "international transfers",
        "Data-subject request",
        "DPIA",
        "AI notice",
        "End-of-contract",
        "PR #16",
    ):
        if phrase.lower() not in privacy.lower():
            fail(f"privacy draft missing required concept: {phrase}")

    terms = read(RELEASE / "customer-terms-draft.md")
    for phrase in ("not an executable contract", "Proprietary deployment/use grant", "RPO", "RTO", "PR #16"):
        if phrase.lower() not in terms.lower():
            fail(f"customer terms draft missing required concept: {phrase}")

    support = read(RELEASE / "support-service-definition.md")
    for phrase in ("Pilot", "Contracted production", "Maintenance windows", "Availability measurement", "Rate limiting", "DoS"):
        if phrase.lower() not in support.lower():
            fail(f"support definition missing required concept: {phrase}")


def verify_markdown_style() -> None:
    files = [README] + [RELEASE / name for name in REQUIRED_DOCS]
    for path in files:
        text = read(path)
        if not text.endswith("\n"):
            fail(f"Markdown file lacks final newline: {path.relative_to(ROOT)}")
        if "\t" in text:
            fail(f"Markdown file contains tab characters: {path.relative_to(ROOT)}")
        for number, line in enumerate(text.splitlines(), 1):
            if line.rstrip() != line:
                fail(f"trailing whitespace in {path.relative_to(ROOT)}:{number}")
        nonblank = next((line for line in text.splitlines() if line.strip()), "")
        if not nonblank.startswith("# "):
            fail(f"Markdown file must begin with one H1: {path.relative_to(ROOT)}")
        if sum(1 for line in text.splitlines() if line.startswith("# ")) != 1:
            fail(f"Markdown file must contain exactly one H1: {path.relative_to(ROOT)}")


def verify_documentation_drift() -> None:
    for path in DRIFT_DOCS:
        text = read(path).lower()
        for phrase in STALE_ARCHITECTURE:
            if phrase.lower() in text:
                fail(f"stale production architecture phrase in {path.relative_to(ROOT)}: {phrase}")
    adr = read(ROOT / "docs" / "adr" / "0005-transaction-scoped-rls.md")
    if "Accepted and implemented" not in adr:
        fail("ADR 0005 must be recorded as accepted/implemented")


def verify_local_links_and_patterns() -> None:
    files = [README] + [RELEASE / name for name in REQUIRED_DOCS]
    for path in files:
        text = read(path)
        for label, pattern in SECRET_PATTERNS.items():
            if pattern.search(text):
                fail(f"possible {label} in {path.relative_to(ROOT)}")
        for address in EMAIL.findall(text):
            if not address.lower().endswith(".invalid"):
                fail(f"personal/real-looking email address in release docs: {address}")
        for target in LINK.findall(text):
            target = target.strip()
            if target.startswith(("http://", "https://", "mailto:", "#")):
                fail(f"external/non-local Markdown link is not allowlisted in {path.relative_to(ROOT)}: {target}")
            clean = target.split("#", 1)[0]
            if not clean:
                continue
            resolved = (path.parent / clean).resolve()
            try:
                resolved.relative_to(ROOT.resolve())
            except ValueError:
                fail(f"Markdown link escapes repository in {path.relative_to(ROOT)}: {target}")
            if not resolved.exists():
                fail(f"broken local Markdown link in {path.relative_to(ROOT)}: {target}")


def main() -> int:
    for name in REQUIRED_DOCS:
        read(RELEASE / name)
    verify_feature_truth()
    verify_required_language()
    verify_documentation_drift()
    verify_markdown_style()
    verify_local_links_and_patterns()
    print("Release documentation truthfulness, local links, secret/PII patterns, and required scope verified.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"release-docs verification failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
