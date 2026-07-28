#!/usr/bin/env python3
"""Stage production definitions without copying live installation state."""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path


def reject_symlinks(source: Path) -> None:
    for path in source.rglob("*"):
        if path.is_symlink():
            raise RuntimeError(
                f"production source contains a symlink: {path.relative_to(source)}"
            )


def stage_production(source: Path, destination: Path) -> None:
    source = source.resolve()
    destination = destination.resolve()
    if not source.is_dir():
        raise RuntimeError("production source does not exist")
    if destination.exists():
        raise RuntimeError("production staging destination already exists")

    reject_symlinks(source)

    def ignore(directory: str, names: list[str]) -> set[str]:
        relative = Path(directory).resolve().relative_to(source)
        ignored: set[str] = set()
        if relative == Path(".") and ".env.edutalent" in names:
            ignored.add(".env.edutalent")
        if relative == Path("runtime") and "supabase" in names:
            ignored.add("supabase")
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
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    stage_production(args.source, args.destination)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError) as error:
        print(f"production staging error: {error}", file=sys.stderr)
        raise SystemExit(1)
