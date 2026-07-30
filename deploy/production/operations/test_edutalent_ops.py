from __future__ import annotations

import importlib.util
import json
import re
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("edutalent_ops.py")
WORKFLOW_PATH = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "production-operations.yml"
spec = importlib.util.spec_from_file_location("edutalent_ops", MODULE_PATH)
assert spec and spec.loader
import sys
ops = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = ops
spec.loader.exec_module(ops)


class BackupManifestTests(unittest.TestCase):
    def test_manifest_detects_tampering_and_extra_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            payload = root / "database.dump"
            payload.write_bytes(b"verified")
            ops.create_manifest(root, {"test": True})
            ops.verify_manifest(root)
            payload.write_bytes(b"tampered")
            with self.assertRaisesRegex(RuntimeError, "checksum mismatch|size mismatch"):
                ops.verify_manifest(root)

    def test_manifest_rejects_unlisted_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "a").write_text("a", encoding="utf-8")
            ops.create_manifest(root)
            (root / "b").write_text("b", encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "inventory mismatch"):
                ops.verify_manifest(root)


class ComposeSecurityTests(unittest.TestCase):
    def secure_config(self) -> dict:
        baseline = {"security_opt": ["no-new-privileges:true"], "networks": ["data"]}
        return {
            "services": {
                "gateway": {**baseline, "ports": [{"published": "80"}, {"published": "443"}], "networks": ["ingress", "edge"]},
                "ai-gateway": {**baseline, "networks": ["ai_internal", "ai_egress"]},
                "app": baseline,
            },
            "networks": {
                "edge": {"internal": True},
                "supabase_api": {"internal": True},
                "data": {"internal": True},
                "admin": {"internal": True},
                "ai_internal": {"internal": True},
                "ingress": {"internal": False},
                "ai_egress": {"internal": False},
            },
        }

    def test_secure_topology_passes(self) -> None:
        self.assertEqual(ops.validate_compose_security(self.secure_config()), [])

    def test_public_database_and_docker_socket_fail(self) -> None:
        config = self.secure_config()
        config["services"]["app"]["ports"] = ["8080:8080"]
        config["services"]["app"]["volumes"] = ["/var/run/docker.sock:/var/run/docker.sock"]
        config["services"]["app"]["networks"] = ["data", "ai_egress"]
        violations = "\n".join(ops.validate_compose_security(config))
        self.assertIn("only gateway may publish", violations)
        self.assertIn("Docker socket", violations)
        self.assertIn("only ai-gateway", violations)


class WorkflowPrivilegeBoundaryTests(unittest.TestCase):
    def test_wal_switch_uses_admin_without_elevating_postgres(self) -> None:
        workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        self.assertRegex(
            workflow,
            re.compile(
                r"psql\s+-h 127\.0\.0\.1\s+-U postgres\b.*?operations_backup_probe",
                re.DOTALL,
            ),
        )
        self.assertRegex(
            workflow,
            re.compile(
                r"psql\s+-h 127\.0\.0\.1\s+-U supabase_admin\b.*?pg_switch_wal",
                re.DOTALL,
            ),
        )
        self.assertIn("postgres|f", workflow)
        self.assertIn("supabase_admin|t", workflow)
        self.assertNotRegex(
            workflow,
            re.compile(
                r"(?im)^\s*(?:ALTER\s+ROLE\s+postgres\b[^\n]*\bSUPERUSER\b|GRANT\s+EXECUTE\s+ON\s+FUNCTION\s+[^\n]*pg_switch_wal[^\n]*\bTO\s+postgres\b)"
            ),
        )


class AlertTests(unittest.TestCase):
    def test_stale_backup_disk_and_service_raise_critical_alerts(self) -> None:
        snapshot = {
            "collected_epoch": 1_000_000,
            "services": [{"Service": "app", "State": "exited", "Health": ""}],
            "disk_free_bytes": 1,
            "tls_seconds_remaining": 100,
            "database_connections": 90,
            "database_connection_limit": 100,
            "latest_backup_created_at": "1970-01-01T00:00:00Z",
            "latest_wal_received_at": None,
            "core_health": False,
            "qdrant_health": False,
            "ai_gateway_health": False,
        }
        policy = {
            "required_services": ["app"],
            "minimum_disk_free_bytes": 100,
            "minimum_tls_seconds_remaining": 200,
            "maximum_database_connection_ratio": 0.8,
            "maximum_backup_age_seconds": 60,
            "maximum_wal_age_seconds": 60,
        }
        codes = {alert["code"] for alert in ops.evaluate_alerts(snapshot, policy)}
        self.assertTrue({"service_unhealthy", "disk_free_low", "tls_expiring", "database_connections_high", "backup_stale", "wal_archive_stale", "core_health_failed"}.issubset(codes))


if __name__ == "__main__":
    unittest.main()
