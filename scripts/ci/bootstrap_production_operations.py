#!/usr/bin/env python3
"""Materialize the reviewed production-operations tree from API transfer chunks."""

from __future__ import annotations

import base64
import hashlib
import io
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARTS = ROOT / ".bootstrap"
EXPECTED_SHA256 = "4f3fc1cb3e804e8e6c5251b422b9e5d5663df938e425e31fe617565bf4c9641e"
EXPECTED_LENGTH = 45_716

encoded = "".join(
    (PARTS / f"part{number:02d}").read_text(encoding="ascii")
    for number in range(6)
)
if len(encoded) != EXPECTED_LENGTH:
    raise SystemExit(f"unexpected transfer length: {len(encoded)}")
raw = base64.b64decode(encoded, validate=True)
observed = hashlib.sha256(raw).hexdigest()
if observed != EXPECTED_SHA256:
    raise SystemExit(f"production-operations archive checksum mismatch: {observed}")

with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as archive:
    for member in archive.getmembers():
        path = Path(member.name)
        if path.is_absolute() or ".." in path.parts or member.issym() or member.islnk():
            raise SystemExit(f"unsafe bootstrap member: {member.name}")
        if not (member.isfile() or member.isdir()):
            raise SystemExit(f"unsupported bootstrap member: {member.name}")
    archive.extractall(ROOT, filter="data")
