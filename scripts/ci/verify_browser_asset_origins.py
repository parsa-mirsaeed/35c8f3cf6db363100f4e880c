#!/usr/bin/env python3
"""Fail closed if production browser sources contain external runtime origins or secrets."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCAN_ROOTS = [ROOT / "packages" / "web", ROOT / "deploy"]
TEXT_SUFFIXES = {".rs", ".css", ".html", ".js", ".ts", ".tsx", ".json", ".toml", ".yaml", ".yml", ".svg"}
IGNORED_PARTS = {"target", ".git", "node_modules"}

REMOTE_ORIGIN = re.compile(r"(?:https?:)?//(?:[A-Za-z0-9.-]+)(?::\d+)?(?:/|$)")
PRIVATE_ORIGIN = re.compile(
    r"https?://(?:localhost|127\.0\.0\.1|10\.|172\.(?:1[6-9]|2\d|3[0-1])\.|192\.168\.)"
)
SECRET = re.compile(
    r'''(?i)(?:sk-[A-Za-z0-9_-]{12,}|(?:api[_-]?key|secret|private[_-]?key|service[_-]?role|access[_-]?token)\s*[:=]\s*['\"][^'\"]{12,})'''
)


def iter_sources() -> list[Path]:
    files: list[Path] = []
    for root in SCAN_ROOTS:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if not path.is_file() or path.suffix.lower() not in TEXT_SUFFIXES:
                continue
            if any(part in IGNORED_PARTS for part in path.parts):
                continue
            files.append(path)
    return files


def main() -> int:
    violations: list[str] = []
    files = iter_sources()
    for path in files:
        text = path.read_text(encoding="utf-8", errors="ignore")
        for match in REMOTE_ORIGIN.finditer(text):
            violations.append(f"{path.relative_to(ROOT)}: external browser origin {match.group(0)}")
        if PRIVATE_ORIGIN.search(text):
            violations.append(f"{path.relative_to(ROOT)}: private service origin")
        if SECRET.search(text):
            violations.append(f"{path.relative_to(ROOT)}: possible provider credential or secret")

    if violations:
        print("Browser asset origin/secret verification failed:")
        print("\n".join(sorted(set(violations))))
        return 1

    print(f"Browser asset origin/secret verification passed for {len(files)} source files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
