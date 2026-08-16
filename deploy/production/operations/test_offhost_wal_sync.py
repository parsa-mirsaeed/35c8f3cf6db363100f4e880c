from __future__ import annotations

import hashlib
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

OPERATIONS_DIR = Path(__file__).resolve().parent
PRODUCTION_DIR = OPERATIONS_DIR.parent
SCRIPT = PRODUCTION_DIR / "systemd" / "run-offhost-wal-sync"


class OffhostWalSyncTests(unittest.TestCase):
    def run_sync(self, local: Path, remote: Path) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.update(
            {
                "EDUTALENT_WAL_DIR": str(local),
                "EDUTALENT_OFFHOST_WAL_DIR": str(remote),
            }
        )
        return subprocess.run(
            ["bash", str(SCRIPT)],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )

    def test_completed_wal_segments_copy_and_verify(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            local = root / "wal"
            remote = root / "offhost"
            local.mkdir()
            remote.mkdir()
            segment_a = local / "000000010000000000000001"
            segment_b = local / "000000010000000000000002"
            partial = local / "000000010000000000000003.partial"
            segment_a.write_bytes(b"segment-a")
            segment_b.write_bytes(b"segment-b")
            partial.write_bytes(b"must-not-copy")

            completed = self.run_sync(local, remote)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            for source in (segment_a, segment_b):
                copied = remote / source.name
                self.assertEqual(copied.read_bytes(), source.read_bytes())
                self.assertEqual(copied.stat().st_mode & 0o777, 0o600)
                self.assertEqual(
                    hashlib.sha256(copied.read_bytes()).hexdigest(),
                    hashlib.sha256(source.read_bytes()).hexdigest(),
                )
            self.assertFalse((remote / partial.name).exists())

    def test_existing_tampered_offhost_segment_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            local = root / "wal"
            remote = root / "offhost"
            local.mkdir()
            remote.mkdir()
            name = "00000001000000000000000A"
            (local / name).write_bytes(b"source")
            (remote / name).write_bytes(b"tampered")

            completed = self.run_sync(local, remote)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("hash mismatch", completed.stderr.lower())

    def test_empty_wal_directory_is_not_reported_as_success(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            local = root / "wal"
            remote = root / "offhost"
            local.mkdir()
            remote.mkdir()
            completed = self.run_sync(local, remote)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("No completed WAL segments", completed.stderr)


if __name__ == "__main__":
    unittest.main()
