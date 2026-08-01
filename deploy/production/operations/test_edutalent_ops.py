from __future__ import annotations

import importlib.util
import json
import re
import stat
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("edutalent_ops.py")
PRODUCTION_DIR = Path(__file__).resolve().parent.parent
OPERATIONS_SCRIPT_PATH = PRODUCTION_DIR / "edutalent-operations"
COMPOSE_PATH = PRODUCTION_DIR / "compose.production.yaml"
HBA_PATH = PRODUCTION_DIR / "pg_hba.conf"
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

    def test_manifest_rejects_mode_changes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            wal = root / "000000010000000000000001"
            wal.write_bytes(b"wal")
            wal.chmod(0o600)
            ops.create_manifest(root)
            ops.verify_manifest(root)
            wal.chmod(0o644)
            with self.assertRaisesRegex(RuntimeError, "mode mismatch"):
                ops.verify_manifest(root)

    def test_manifest_normalizes_payload_permissions(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "payload"
            nested = root / "nested"
            nested.mkdir(parents=True)
            root.chmod(0o755)
            nested.chmod(0o755)
            public_file = nested / "public.txt"
            public_file.write_text("public", encoding="utf-8")
            public_file.chmod(0o644)
            executable_file = root / "tool"
            executable_file.write_bytes(b"tool")
            executable_file.chmod(0o755)

            manifest = ops.create_manifest(root)
            ops.verify_manifest(root)

            self.assertEqual(stat.S_IMODE(root.stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE(nested.stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE(public_file.stat().st_mode), 0o600)
            self.assertEqual(stat.S_IMODE(executable_file.stat().st_mode), 0o600)
            self.assertEqual({row["mode"] for row in manifest["files"]}, {0o600})

        script = OPERATIONS_SCRIPT_PATH.read_text(encoding="utf-8")
        decrypt = script.split("decrypt_backup() {", 1)[1].split(
            "\nverify_decrypted_payload() {", 1
        )[0]
        self.assertNotIn("--same-permissions", decrypt)
        self.assertIn("manifest-create", script)


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
        self.assertIn(
            "has_function_privilege('postgres', 'pg_catalog.pg_switch_wal()', 'EXECUTE')",
            workflow,
        )
        self.assertIn(
            "has_function_privilege('supabase_admin', 'pg_catalog.pg_switch_wal()', 'EXECUTE')",
            workflow,
        )
        self.assertIn("test \"${privilege_state}\" = 'false|false|true'", workflow)
        self.assertNotRegex(
            workflow,
            re.compile(
                r"(?im)^\s*(?:ALTER\s+ROLE\s+postgres\b[^\n]*\bSUPERUSER\b|GRANT\s+EXECUTE\s+ON\s+FUNCTION\s+[^\n]*pg_switch_wal[^\n]*\bTO\s+postgres\b)"
            ),
        )

    def test_wal_switch_waits_for_active_physical_receiver(self) -> None:
        workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        self.assertIn(
            "SELECT active::text FROM pg_replication_slots WHERE slot_name = 'edutalent_backup'",
            workflow,
        )
        self.assertIn('if [[ "${receiver_active}" != true ]]; then', workflow)
        self.assertIn("docker logs --tail 100 edutalent-pitr-archive", workflow)
        self.assertIn("Active WAL receiver did not persist an archive file", workflow)
        self.assertIn("test \"${receiver_network}\" = edutalent-data", workflow)
        self.assertIn("Replication-only role unexpectedly opened a normal SQL session", workflow)

    def test_wal_receiver_uses_dedicated_role_on_private_network(self) -> None:
        script = OPERATIONS_SCRIPT_PATH.read_text(encoding="utf-8")
        pitr_start = script.split("pitr_start() {", 1)[1].split("\npitr_verify() {", 1)[0]
        self.assertIn("--network edutalent-data", pitr_start)
        self.assertIn("supabase-db:5432:*:edutalent_backup", pitr_start)
        self.assertIn(
            "postgresql://edutalent_backup@supabase-db:5432/", pitr_start
        )
        self.assertIn("CREATE ROLE edutalent_backup", pitr_start)
        self.assertIn("ALTER ROLE edutalent_backup", pitr_start)
        self.assertIn("true|true|false|false|false|false|false", pitr_start)
        self.assertNotIn("--network container:", pitr_start)
        self.assertNotIn("postgresql://postgres@127.0.0.1:5432/", pitr_start)

    def test_replication_hba_rejects_normal_backup_role_sessions(self) -> None:
        records = [
            line.split()
            for line in HBA_PATH.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
        replication_records = [
            record for record in records if len(record) >= 2 and record[1] == "replication"
        ]
        self.assertEqual(
            replication_records,
            [
                ["host", "replication", "postgres", "127.0.0.1/32", "scram-sha-256"],
                ["host", "replication", "edutalent_backup", "10.0.0.0/8", "scram-sha-256"],
                ["host", "replication", "edutalent_backup", "172.16.0.0/12", "scram-sha-256"],
                ["host", "replication", "edutalent_backup", "192.168.0.0/16", "scram-sha-256"],
            ],
        )
        self.assertIn(
            ["host", "all", "edutalent_backup", "0.0.0.0/0", "reject"], records
        )
        self.assertIn(
            ["host", "all", "edutalent_backup", "::0/0", "reject"], records
        )
        compose = COMPOSE_PATH.read_text(encoding="utf-8")
        self.assertIn(
            "${EDUTALENT_PRODUCTION_DIR:?EDUTALENT_PRODUCTION_DIR is required}/pg_hba.conf:/etc/postgresql/pg_hba.conf:ro",
            compose,
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
