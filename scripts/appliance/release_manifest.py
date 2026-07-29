#!/usr/bin/env python3
"""Generate and verify the immutable air-gapped appliance manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import sys
import tarfile
from pathlib import Path
from typing import Any

HEX_40 = re.compile(r"^[0-9a-f]{40}$")
DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
VERSION = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$")
DATABASE_URL = re.compile(rb"postgres(?:ql)?://[^\s:@/]+:[^\s@/]+@", re.IGNORECASE)
FORBIDDEN_SUFFIXES = {".key", ".pem", ".p12", ".pfx", ".pdf", ".dump", ".sql.gz"}
FORBIDDEN_NAMES = {".env", "id_rsa", "id_ed25519", "credentials.json"}
ALLOWED_DOTENV_PATHS = {"deploy/production/.env.edutalent.example"}
PRIVATE_KEY_MARKERS = (
    b"-----BEGIN " + b"PRIVATE KEY-----",
    b"-----BEGIN RSA " + b"PRIVATE KEY-----",
    b"-----BEGIN EC " + b"PRIVATE KEY-----",
    b"-----BEGIN DSA " + b"PRIVATE KEY-----",
    b"-----BEGIN OPENSSH " + b"PRIVATE KEY-----",
    b"-----BEGIN ENCRYPTED " + b"PRIVATE KEY-----",
    b"-----BEGIN PGP " + b"PRIVATE KEY BLOCK-----",
)
SCAN_CHUNK_SIZE = 1024 * 1024
SCAN_OVERLAP = 64 * 1024


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def relative_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if path.is_symlink():
            raise RuntimeError(f"bundle contains a symlink: {relative}")
        if path.is_dir():
            continue
        if not path.is_file():
            raise RuntimeError(f"bundle contains a non-regular entry: {relative}")
        files.append(path)
    return files


def is_manifest_input(relative: Path) -> bool:
    value = relative.as_posix()
    return value not in {
        "SHA256SUMS",
        "manifests/release-manifest.json",
    } and not value.startswith("signatures/")


def scan_forbidden_content(path: Path) -> None:
    overlap = b""
    with path.open("rb") as handle:
        while chunk := handle.read(SCAN_CHUNK_SIZE):
            data = overlap + chunk
            if any(marker in data for marker in PRIVATE_KEY_MARKERS):
                raise RuntimeError(f"private key material entered release: {path}")
            for match in DATABASE_URL.finditer(data):
                candidate = match.group(0)
                if b"${" not in candidate and b"{{" not in candidate:
                    raise RuntimeError(f"credentialed database URL entered release: {path}")
            overlap = data[-SCAN_OVERLAP:]


def reject_forbidden_file(root: Path, path: Path) -> None:
    relative = path.relative_to(root)
    name = path.name.lower()
    value = relative.as_posix().lower()
    dotenv = name == ".env" or (
        name.startswith(".env.")
        and name != ".env.example"
        and value not in ALLOWED_DOTENV_PATHS
    )
    if dotenv or name in FORBIDDEN_NAMES or any(value.endswith(suffix) for suffix in FORBIDDEN_SUFFIXES):
        raise RuntimeError(f"forbidden release file: {relative}")
    if "target/" in f"/{value}" or "node_modules/" in f"/{value}":
        raise RuntimeError(f"build output entered release: {relative}")
    scan_forbidden_content(path)


def docker_archive_identity(path: Path) -> tuple[str, set[str]]:
    try:
        with tarfile.open(path, mode="r:gz") as archive:
            try:
                manifest_handle = archive.extractfile("manifest.json")
            except KeyError:
                manifest_handle = None
            if manifest_handle is None:
                raise RuntimeError("Docker archive manifest.json is missing")
            entries = json.loads(manifest_handle.read())
            if not isinstance(entries, list) or len(entries) != 1:
                raise RuntimeError("each appliance archive must contain exactly one image")
            entry = entries[0]
            config_name = entry.get("Config", "")
            if not isinstance(config_name, str):
                raise RuntimeError("Docker archive image config is invalid")
            legacy = re.fullmatch(r"([0-9a-f]{64})\.json", config_name)
            content_store = re.fullmatch(r"blobs/sha256/([0-9a-f]{64})", config_name)
            match = legacy or content_store
            if match is None:
                raise RuntimeError("Docker archive image config is invalid")
            declared_digest = match.group(1)
            try:
                config_handle = archive.extractfile(config_name)
            except KeyError:
                config_handle = None
            if config_handle is None:
                raise RuntimeError("Docker archive image config is missing")
            config = config_handle.read()
            digest = hashlib.sha256(config).hexdigest()
            if declared_digest != digest:
                raise RuntimeError("Docker archive config name does not match its content digest")
            tags = entry.get("RepoTags") or []
            if not isinstance(tags, list) or not all(isinstance(tag, str) for tag in tags):
                raise RuntimeError("Docker archive RepoTags are invalid")
            return f"sha256:{digest}", set(tags)
    except (tarfile.TarError, json.JSONDecodeError) as error:
        raise RuntimeError(f"invalid Docker image archive: {path.name}") from error

def file_inventory(root: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for path in relative_files(root):
        relative = path.relative_to(root)
        reject_forbidden_file(root, path)
        if not is_manifest_input(relative):
            continue
        rows.append(
            {
                "path": relative.as_posix(),
                "sha256": sha256_file(path),
                "size": path.stat().st_size,
                "mode": format(stat.S_IMODE(path.stat().st_mode), "04o"),
            }
        )
    return rows


def validate_images(root: Path, images: dict[str, Any], platform: str) -> list[dict[str, Any]]:
    if images.get("schema_version") != 1:
        raise RuntimeError("unsupported image inventory schema")
    records = images.get("images")
    if not isinstance(records, list) or not records:
        raise RuntimeError("image inventory is empty")
    local_tags: set[str] = set()
    archives: set[str] = set()
    components: set[str] = set()
    normalized: list[dict[str, Any]] = []
    for record in records:
        required = {
            "component",
            "services",
            "source_ref",
            "source_digest",
            "local_tag",
            "archive",
            "platform",
            "sbom",
        }
        missing = required.difference(record)
        if missing:
            raise RuntimeError(f"image record missing fields: {sorted(missing)}")
        if record["component"] in components:
            raise RuntimeError(f"duplicate image component: {record['component']}")
        if record["platform"] != platform:
            raise RuntimeError(f"image platform mismatch for {record['component']}")
        if not DIGEST.fullmatch(record["source_digest"]):
            raise RuntimeError(f"invalid source digest for {record['component']}")
        if record["local_tag"].endswith(":latest") or "@" in record["local_tag"]:
            raise RuntimeError(f"non-immutable local tag for {record['component']}")
        if record["local_tag"] in local_tags:
            raise RuntimeError(f"duplicate local tag: {record['local_tag']}")
        if record["archive"] in archives:
            raise RuntimeError(f"duplicate image archive: {record['archive']}")
        components.add(record["component"])
        local_tags.add(record["local_tag"])
        archives.add(record["archive"])
        archive = root / record["archive"]
        sbom = root / record["sbom"]
        if not archive.is_file() or not sbom.is_file():
            raise RuntimeError(f"missing image payload for {record['component']}")
        image_id, archive_tags = docker_archive_identity(archive)
        if record["local_tag"] not in archive_tags:
            raise RuntimeError(f"archive does not contain declared tag for {record['component']}")
        supplied_image_id = record.get("image_id")
        if supplied_image_id is not None and supplied_image_id != image_id:
            raise RuntimeError(f"supplied image ID mismatch for {record['component']}")
        normalized.append(
            {
                **record,
                "image_id": image_id,
                "archive_sha256": sha256_file(archive),
                "archive_size": archive.stat().st_size,
                "sbom_sha256": sha256_file(sbom),
            }
        )
    return sorted(normalized, key=lambda row: row["component"])


def load_model(root: Path, lock_path: Path) -> dict[str, Any]:
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    revision = lock.get("revision", "")
    if not HEX_40.fullmatch(revision):
        raise RuntimeError("model lock revision is not immutable")
    model_root = root / "models" / lock["profile"]
    metadata_path = model_root / "MODEL_METADATA.json"
    sums_path = model_root / "MODEL_SHA256SUMS"
    primary = model_root / lock["primary_weight"]["path"]
    if not metadata_path.is_file() or not sums_path.is_file() or not primary.is_file():
        raise RuntimeError("offline model payload is incomplete")
    observed = sha256_file(primary)
    if observed != lock["primary_weight"]["sha256"]:
        raise RuntimeError("offline model primary weight checksum mismatch")
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    if metadata.get("revision") != revision or metadata.get("dimensions") != lock["dimensions"]:
        raise RuntimeError("offline model metadata does not match lock")
    return {
        "profile": lock["profile"],
        "repository": lock["repository"],
        "revision": revision,
        "served_model_name": lock["served_model_name"],
        "dimensions": lock["dimensions"],
        "license": lock["license"],
        "primary_weight": lock["primary_weight"],
        "checksums": "models/{}/MODEL_SHA256SUMS".format(lock["profile"]),
        "checksums_sha256": sha256_file(sums_path),
    }


def write_sums(root: Path) -> None:
    rows: list[str] = []
    for path in relative_files(root):
        relative = path.relative_to(root)
        value = relative.as_posix()
        if value == "SHA256SUMS" or value.startswith("signatures/"):
            continue
        rows.append(f"{sha256_file(path)}  {value}")
    (root / "SHA256SUMS").write_text("\n".join(rows) + "\n", encoding="utf-8")


def generate(args: argparse.Namespace) -> None:
    root = args.bundle.resolve()
    if not root.is_dir():
        raise RuntimeError("bundle root does not exist")
    if not VERSION.fullmatch(args.version):
        raise RuntimeError("version must be a safe tag and path component")
    if not HEX_40.fullmatch(args.git_sha):
        raise RuntimeError("git SHA must be a full lowercase commit SHA")
    if args.platform not in {"linux/amd64", "linux/arm64"}:
        raise RuntimeError("unsupported appliance platform")
    images = json.loads(args.images.read_text(encoding="utf-8"))
    manifest = {
        "schema_version": 1,
        "release": {
            "name": "edutalent-offline-appliance",
            "version": args.version,
            "git_sha": args.git_sha,
            "platform": args.platform,
            "signing_mode": args.signing_mode,
            "created_by": "scripts/appliance/build.sh",
        },
        "images": validate_images(root, images, args.platform),
        "model": load_model(root, args.model_lock),
        "files": file_inventory(root),
    }
    output = root / "manifests" / "release-manifest.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_sums(root)


def verify(args: argparse.Namespace) -> None:
    root = args.bundle.resolve()
    manifest_path = root / "manifests" / "release-manifest.json"
    sums_path = root / "SHA256SUMS"
    if not manifest_path.is_file() or not sums_path.is_file():
        raise RuntimeError("release manifest or SHA256SUMS is missing")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        raise RuntimeError("unsupported release manifest schema")
    release = manifest.get("release", {})
    if not VERSION.fullmatch(release.get("version", "")):
        raise RuntimeError("release manifest version is invalid")
    if not HEX_40.fullmatch(release.get("git_sha", "")):
        raise RuntimeError("release manifest git SHA is invalid")
    if release.get("platform") not in {"linux/amd64", "linux/arm64"}:
        raise RuntimeError("release manifest platform is invalid")
    if release.get("signing_mode") not in {"ephemeral", "keyless"}:
        raise RuntimeError("release manifest signing mode is invalid")
    expected_files = {row["path"]: row for row in manifest.get("files", [])}
    if not expected_files:
        raise RuntimeError("release file inventory is empty")
    actual_inputs = {
        path.relative_to(root).as_posix()
        for path in relative_files(root)
        if is_manifest_input(path.relative_to(root))
    }
    if actual_inputs != set(expected_files):
        extra = sorted(actual_inputs.difference(expected_files))
        missing = sorted(set(expected_files).difference(actual_inputs))
        raise RuntimeError(f"release inventory mismatch; extra={extra}, missing={missing}")
    for relative, row in expected_files.items():
        path = root / relative
        reject_forbidden_file(root, path)
        actual_mode = format(stat.S_IMODE(path.stat().st_mode), "04o")
        if (
            sha256_file(path) != row["sha256"]
            or path.stat().st_size != row["size"]
            or actual_mode != row.get("mode")
        ):
            raise RuntimeError(f"release file integrity failure: {relative}")
    sums: dict[str, str] = {}
    for line in sums_path.read_text(encoding="utf-8").splitlines():
        digest, relative = line.split("  ", 1)
        if relative in sums:
            raise RuntimeError(f"duplicate checksum entry: {relative}")
        sums[relative] = digest
    expected_sums = set(expected_files) | {"manifests/release-manifest.json"}
    if set(sums) != expected_sums:
        raise RuntimeError("SHA256SUMS does not exactly cover the release inventory")
    for relative, digest in sums.items():
        path = root / relative
        if not path.is_file() or sha256_file(path) != digest:
            raise RuntimeError(f"SHA256SUMS verification failed: {relative}")
    image_tags: set[str] = set()
    for image in manifest.get("images", []):
        if not DIGEST.fullmatch(image.get("source_digest", "")):
            raise RuntimeError("invalid source image digest in release manifest")
        if not DIGEST.fullmatch(image.get("image_id", "")):
            raise RuntimeError("invalid image content ID in release manifest")
        if image["local_tag"] in image_tags:
            raise RuntimeError("duplicate image tag in release manifest")
        image_tags.add(image["local_tag"])
        archive = root / image["archive"]
        if sha256_file(archive) != image["archive_sha256"]:
            raise RuntimeError(f"image archive integrity failure: {image['component']}")
        archive_image_id, archive_tags = docker_archive_identity(archive)
        if archive_image_id != image["image_id"] or image["local_tag"] not in archive_tags:
            raise RuntimeError(f"Docker archive identity failure: {image['component']}")
    print(
        "Verified immutable appliance manifest for "
        f"{release.get('version')} {release.get('platform')} at {release.get('git_sha')}"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    generate_parser = subparsers.add_parser("generate")
    generate_parser.add_argument("--bundle", required=True, type=Path)
    generate_parser.add_argument("--version", required=True)
    generate_parser.add_argument("--git-sha", required=True)
    generate_parser.add_argument("--platform", required=True)
    generate_parser.add_argument(
        "--signing-mode", required=True, choices=("ephemeral", "keyless")
    )
    generate_parser.add_argument("--images", required=True, type=Path)
    generate_parser.add_argument("--model-lock", required=True, type=Path)
    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("--bundle", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "generate":
        generate(args)
    else:
        verify(args)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError, RuntimeError) as error:
        print(f"appliance manifest error: {error}", file=sys.stderr)
        raise SystemExit(1)
