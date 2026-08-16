from __future__ import annotations

import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

OPERATIONS_DIR = Path(__file__).resolve().parent
PRODUCTION_DIR = OPERATIONS_DIR.parent
SCRIPT = PRODUCTION_DIR / "systemd" / "run-latest-offhost-copy"


class OffhostBackupCopyTests(unittest.TestCase):
    def test_copy_publishes_only_verified_encrypted_archive_and_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            local = root / "local"
            remote = root / "remote"
            local.mkdir()
            remote.mkdir()
            archive = local / "edutalent-backup-20260816T120000Z.tar.gz.enc"
            archive.write_bytes(b"encrypted-test-payload")
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            metadata = Path(f"{archive}.metadata.json")
            metadata.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "archive": archive.name,
                        "sha256": digest,
                    }
                ),
                encoding="utf-8",
            )
            env = os.environ.copy()
            env.update(
                {
                    "EDUTALENT_BACKUP_DIR": str(local),
                    "EDUTALENT_OFFHOST_BACKUP_DIR": str(remote),
                }
            )
            completed = subprocess.run(
                ["bash", str(SCRIPT)],
                check=False,
                capture_output=True,
                text=True,
                env=env,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            copied = remote / archive.name
            copied_metadata = Path(f"{copied}.metadata.json")
            self.assertEqual(copied.read_bytes(), archive.read_bytes())
            self.assertEqual(
                hashlib.sha256(copied.read_bytes()).hexdigest(), digest
            )
            self.assertEqual(
                json.loads(copied_metadata.read_text(encoding="utf-8"))["sha256"],
                digest,
            )
            self.assertEqual(copied.stat().st_mode & 0o777, 0o600)
            self.assertFalse(any("passphrase" in path.name for path in remote.iterdir()))
            self.assertFalse(any("partial" in path.name for path in remote.iterdir()))

    def test_copy_rejects_tampered_source_before_publication(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            local = root / "local"
            remote = root / "remote"
            local.mkdir()
            remote.mkdir()
            archive = local / "edutalent-backup-20260816T130000Z.tar.gz.enc"
            archive.write_bytes(b"tampered")
            metadata = Path(f"{archive}.metadata.json")
            metadata.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "archive": archive.name,
                        "sha256": "0" * 64,
                    }
                ),
                encoding="utf-8",
            )
            env = os.environ.copy()
            env.update(
                {
                    "EDUTALENT_BACKUP_DIR": str(local),
                    "EDUTALENT_OFFHOST_BACKUP_DIR": str(remote),
                }
            )
            completed = subprocess.run(
                ["bash", str(SCRIPT)],
                check=False,
                capture_output=True,
                text=True,
                env=env,
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("Source backup SHA-256 does not match metadata", completed.stderr)
            self.assertEqual(list(remote.iterdir()), [])

    def test_copy_rejects_same_local_and_offhost_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            env = os.environ.copy()
            env.update(
                {
                    "EDUTALENT_BACKUP_DIR": str(root),
                    "EDUTALENT_OFFHOST_BACKUP_DIR": str(root),
                }
            )
            completed = subprocess.run(
                ["bash", str(SCRIPT)],
                check=False,
                capture_output=True,
                text=True,
                env=env,
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("must differ from the local backup directory", completed.stderr)


if __name__ == "__main__":
    unittest.main()
