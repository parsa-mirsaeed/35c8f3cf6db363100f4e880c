from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("edutalent_ops.py")
PRODUCTION_DIR = Path(__file__).resolve().parent.parent
SCRIPT_PATH = PRODUCTION_DIR / "edutalent-operations"
spec = importlib.util.spec_from_file_location("edutalent_ops_review", MODULE_PATH)
assert spec and spec.loader
ops = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = ops
spec.loader.exec_module(ops)


class BackupMetadataVerificationTests(unittest.TestCase):
    def test_metadata_requires_the_named_archive_and_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "edutalent-backup-test.tar.gz.enc"
            archive.write_bytes(b"encrypted")
            metadata = root / f"{archive.name}.metadata.json"
            metadata.write_text(json.dumps({
                "schema_version": 1,
                "created_at": "2026-01-01T00:00:00Z",
                "archive": archive.name,
                "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
            }), encoding="utf-8")
            self.assertEqual(
                ops.verify_backup_metadata(metadata, root)["created_at"],
                "2026-01-01T00:00:00Z",
            )
            archive.write_bytes(b"truncated")
            with self.assertRaisesRegex(RuntimeError, "checksum mismatch"):
                ops.verify_backup_metadata(metadata, root)

    def test_metadata_rejects_archive_path_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            metadata = root / "unsafe.metadata.json"
            metadata.write_text(json.dumps({
                "schema_version": 1,
                "created_at": "2026-01-01T00:00:00Z",
                "archive": "../outside",
                "sha256": "0" * 64,
            }), encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "invalid archive name"):
                ops.verify_backup_metadata(metadata, root)


class FailClosedMonitoringTests(unittest.TestCase):
    def test_unknown_and_unverifiable_states_are_critical(self) -> None:
        snapshot = {
            "collected_epoch": 1_000_000,
            "services": [],
            "disk_free_bytes": 1000,
            "backup_configured": True,
            "backup_disk_free_bytes": 1,
            "latest_backup_verified": False,
            "latest_backup_created_at": "1970-01-01T00:00:00Z",
            "latest_wal_received_at": "1970-01-01T00:00:00Z",
            "wal_receiver_running": False,
            "tls_seconds_remaining": -1,
            "database_connections": 0,
            "database_connection_limit": 100,
            "core_health": True,
            "qdrant_health": True,
            "ai_gateway_health": True,
        }
        policy = {
            "required_services": [],
            "minimum_disk_free_bytes": 10,
            "minimum_backup_disk_free_bytes": 10,
            "minimum_tls_seconds_remaining": 10,
            "maximum_database_connection_ratio": 0.9,
            "maximum_backup_age_seconds": 1,
            "maximum_wal_age_seconds": 1,
        }
        codes = {alert["code"] for alert in ops.evaluate_alerts(snapshot, policy)}
        self.assertTrue({
            "backup_disk_free_low",
            "backup_unverifiable",
            "tls_status_unknown",
            "wal_receiver_stopped",
        }.issubset(codes))


class OperationsScriptBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.script = SCRIPT_PATH.read_text(encoding="utf-8")

    def function(self, name: str, next_name: str) -> str:
        return self.script.split(f"{name}() {{", 1)[1].split(f"\n{next_name}() {{", 1)[0]

    def test_backup_quiesces_and_resumes_writer_services(self) -> None:
        backup = self.function("backup_create", "backup_verify")
        self.assertIn("compose ps --services --filter status=running", backup)
        self.assertIn("compose stop --timeout 30", backup)
        self.assertIn('compose start "${quiesced_services[@]}"', backup)
        self.assertIn("--no-deps --pull never --entrypoint sh app", backup)
        self.assertIn('"consistency": "write-quiesced"', backup)
        quiesce_case = backup.split('case "${service}" in', 1)[1].split('esac', 1)[0]
        self.assertNotIn("db|", quiesce_case)
        self.assertNotIn("qdrant", quiesce_case)

    def test_restore_cleanup_removes_every_plaintext_copy(self) -> None:
        restore = self.function("restore_drill", "pitr_start")
        self.assertEqual(restore.count("trap cleanup_drill RETURN"), 1)
        self.assertIn('rm -f "${container_dump}"', restore)
        self.assertIn('rm -rf "${temp}"', restore)
        self.assertNotIn("trap 'rm -rf", restore)

    def test_acceptance_creates_pitr_and_backup_before_alerts(self) -> None:
        acceptance = self.function("acceptance", "prune_backups")
        self.assertLess(acceptance.index("pitr_verify"), acceptance.index("backup_create"))
        self.assertLess(acceptance.index("backup_create"), acceptance.index("collect_snapshot"))
        self.assertLess(acceptance.index("collect_snapshot"), acceptance.index("evaluate_alerts"))

    def test_snapshot_verifies_backup_disk_archive_and_receiver(self) -> None:
        snapshot = self.function("collect_snapshot", "evaluate_alerts")
        self.assertIn("backup-metadata-verify", snapshot)
        self.assertIn("backup_disk_free_bytes", snapshot)
        self.assertIn("wal_receiver_running", snapshot)
        self.assertIn("tls_remaining=-1", snapshot)


if __name__ == "__main__":
    unittest.main()
