#!/usr/bin/env python3
"""Download and verify the immutable local embedding model snapshot."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import stat
import sys
from pathlib import Path


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def reject_unsafe_tree(root: Path) -> None:
    for path in root.rglob("*"):
        relative = path.relative_to(root)
        if path.is_symlink():
            raise RuntimeError(f"model snapshot contains a symlink: {relative}")
        if path.is_file():
            mode = stat.S_IMODE(path.stat().st_mode)
            if mode & 0o111:
                raise RuntimeError(f"model snapshot contains an executable file: {relative}")


def write_checksums(root: Path) -> None:
    rows: list[str] = []
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        relative = path.relative_to(root)
        if relative.as_posix() == "MODEL_SHA256SUMS":
            continue
        rows.append(f"{sha256_file(path)}  {relative.as_posix()}")
    (root / "MODEL_SHA256SUMS").write_text("\n".join(rows) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lock", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--cache", type=Path)
    args = parser.parse_args()

    lock = json.loads(args.lock.read_text(encoding="utf-8"))
    if lock.get("schema_version") != 1:
        raise RuntimeError("unsupported model lock schema")
    revision = lock.get("revision", "")
    if len(revision) != 40 or any(char not in "0123456789abcdef" for char in revision):
        raise RuntimeError("model revision must be a full lowercase commit SHA")

    try:
        from huggingface_hub import snapshot_download
    except ImportError as exc:
        raise RuntimeError(
            "huggingface_hub is required on the connected release builder"
        ) from exc

    output = args.output.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)

    snapshot_download(
        repo_id=lock["repository"],
        revision=revision,
        allow_patterns=lock["allow_patterns"],
        local_dir=output,
        cache_dir=args.cache,
        local_dir_use_symlinks=False,
    )

    cache_dir = output / ".cache"
    if cache_dir.exists():
        shutil.rmtree(cache_dir)

    primary = output / lock["primary_weight"]["path"]
    if not primary.is_file():
        raise RuntimeError(f"missing primary model weight: {primary.name}")
    observed = sha256_file(primary)
    expected = lock["primary_weight"]["sha256"]
    if observed != expected:
        raise RuntimeError(
            f"primary model weight checksum mismatch: {observed} != {expected}"
        )

    reject_unsafe_tree(output)
    metadata = {
        "schema_version": 1,
        "profile": lock["profile"],
        "repository": lock["repository"],
        "revision": revision,
        "served_model_name": lock["served_model_name"],
        "dimensions": lock["dimensions"],
        "license": lock["license"],
        "primary_weight_sha256": observed,
    }
    (output / "MODEL_METADATA.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    write_checksums(output)
    print(f"Pinned model snapshot prepared at {output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError, RuntimeError) as error:
        print(f"model preparation failed: {error}", file=sys.stderr)
        raise SystemExit(1)
