#!/usr/bin/env python3
"""Render the immutable-image appliance overlay with external mutable state."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

DB_VOLUME = "edutalent-appliance-db-data"
STORAGE_VOLUME = "edutalent-appliance-storage-data"
SNIPPETS_VOLUME = "edutalent-appliance-studio-snippets"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--service-images", required=True, type=Path)
    parser.add_argument("--images", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def load_service_sources(path: Path) -> dict[str, str]:
    sources: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        service, source = line.split("\t", 1)
        if service in sources:
            raise RuntimeError(f"duplicate service image row: {service}")
        sources[service] = source
    if not sources:
        raise RuntimeError("service image inventory is empty")
    return sources


def load_source_tags(path: Path) -> dict[str, str]:
    rows = json.loads(path.read_text(encoding="utf-8"))["images"]
    source_tags = {row["source_ref"]: row["local_tag"] for row in rows}
    if len(source_tags) != len(rows):
        raise RuntimeError("duplicate source image references")
    return source_tags


def append_volume(
    lines: list[str], *, source: str, target: str, volume_type: str = "volume", read_only: bool = False
) -> None:
    lines.extend(
        [
            "    volumes:",
            f"      - type: {volume_type}",
            f"        source: {source}",
            f"        target: {target}",
        ]
    )
    if read_only:
        lines.append("        read_only: true")


def render(service_sources: dict[str, str], source_tags: dict[str, str]) -> str:
    missing = sorted(set(service_sources.values()) - set(source_tags))
    if missing:
        raise RuntimeError(f"service images are missing local tags: {missing}")

    lines = ["services:"]
    for service in sorted(service_sources):
        source = service_sources[service]
        lines.extend(
            [
                f"  {service}:",
                f"    image: {source_tags[source]}",
                "    pull_policy: never",
            ]
        )
        if service == "embedding":
            lines.extend(
                [
                    "    command:",
                    "      - --model-id",
                    "      - /models/local-bge-v1",
                    "      - --served-model-name",
                    "      - BAAI/bge-small-en-v1.5",
                ]
            )
            append_volume(
                lines,
                source="${EDUTALENT_APPLIANCE_MODEL_DIR:?EDUTALENT_APPLIANCE_MODEL_DIR is required}",
                target="/models/local-bge-v1",
                volume_type="bind",
                read_only=True,
            )
        elif service == "db":
            append_volume(lines, source=DB_VOLUME, target="/var/lib/postgresql/data")
        elif service in {"storage", "imgproxy"}:
            append_volume(lines, source=STORAGE_VOLUME, target="/var/lib/storage")
        elif service == "studio":
            append_volume(lines, source=SNIPPETS_VOLUME, target="/app/snippets")
        elif service == "functions":
            append_volume(
                lines,
                source="${EDUTALENT_APPLIANCE_FUNCTIONS_DIR:?EDUTALENT_APPLIANCE_FUNCTIONS_DIR is required}",
                target="/home/deno/functions",
                volume_type="bind",
                read_only=True,
            )

    lines.extend(
        [
            "volumes:",
            f"  {DB_VOLUME}:",
            f"  {STORAGE_VOLUME}:",
            f"  {SNIPPETS_VOLUME}:",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    service_sources = load_service_sources(args.service_images)
    source_tags = load_source_tags(args.images)
    args.output.write_text(render(service_sources, source_tags), encoding="utf-8")


if __name__ == "__main__":
    main()
