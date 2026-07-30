#!/usr/bin/env python3
"""Stage production definitions without copying mutable installation state."""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path


SOURCE_MUTABLE_FILES = frozenset({Path(".env.edutalent")})
RELEASE_MUTABLE_FILES = frozenset(
    {
        Path(".env.edutalent"),
        Path("runtime/supabase/.env"),
    }
)
RELEASE_IGNORED_DIRECTORIES = frozenset({Path("runtime/supabase/.git")})
RELEASE_FORBIDDEN_BACKUPS = frozenset(
    {
        Path("runtime/supabase/.env.old"),
        Path("runtime/supabase/docker-compose.yml.old"),
        Path("runtime/supabase/docker-compose.yml.edutalent-backup"),
    }
)
RELEASE_ALLOWED_DOTENV = frozenset({Path(".env.edutalent.example")})


def is_within(relative: Path, roots: frozenset[Path]) -> bool:
    return any(relative == root or root in relative.parents for root in roots)


def validate_entries(source: Path) -> None:
    for path in source.rglob("*"):
        relative = path.relative_to(source)
        if path.is_symlink():
            raise RuntimeError(f"production source contains a symlink: {relative}")
        if not path.is_dir() and not path.is_file():
            raise RuntimeError(
                f"production source contains a non-regular entry: {relative}"
            )


def validate_release_source(source: Path) -> None:
    for path in source.rglob("*"):
        relative = path.relative_to(source)
        if is_within(relative, RELEASE_IGNORED_DIRECTORIES):
            continue
        if relative in RELEASE_FORBIDDEN_BACKUPS:
            raise RuntimeError(
                f"production source retains a generated backup: {relative}"
            )
        if not path.is_file() or relative in RELEASE_MUTABLE_FILES:
            continue
        name = path.name
        if name == ".env" or name.startswith(".env."):
            if name == ".env.example" or relative in RELEASE_ALLOWED_DOTENV:
                continue
            raise RuntimeError(
                f"production source contains an unexpected dotenv file: {relative}"
            )


def stage_production(source: Path, destination: Path, mode: str) -> None:
    source = source.resolve()
    destination = destination.resolve()
    if not source.is_dir():
        raise RuntimeError("production source does not exist")
    if destination.exists():
        raise RuntimeError("production staging destination already exists")

    validate_entries(source)
    if mode == "release":
        validate_release_source(source)

    def ignore(directory: str, names: list[str]) -> set[str]:
        relative = Path(directory).resolve().relative_to(source)
        ignored: set[str] = set()
        for name in names:
            candidate = relative / name
            if mode == "source":
                if candidate in SOURCE_MUTABLE_FILES:
                    ignored.add(name)
                elif relative == Path("runtime") and name == "supabase":
                    ignored.add(name)
            else:
                if candidate in RELEASE_MUTABLE_FILES:
                    ignored.add(name)
                elif candidate in RELEASE_IGNORED_DIRECTORIES:
                    ignored.add(name)
        return ignored

    shutil.copytree(
        source,
        destination,
        ignore=ignore,
        copy_function=shutil.copy2,
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--destination", required=True, type=Path)
    parser.add_argument(
        "--mode",
        choices=("source", "release"),
        default="source",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    stage_production(args.source, args.destination, args.mode)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError) as error:
        print(f"production staging error: {error}", file=sys.stderr)
        raise SystemExit(1)
