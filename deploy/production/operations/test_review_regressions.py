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
REPOSITORY_ROOT = PRODUCTION_DIR.parent.parent
SCRIPT_PATH = PRODUCTION_DIR / "edutalent-operations"
POLICY_PATH = Path(__file__).with_name("alert-policy.json")
WORKFLOW_PATH = REPOSITORY_ROOT / ".github/workflows/production-operations.yml"
WEB_MAIN_PATH = REPOSITORY_ROOT / "packages/web/src/main.rs"
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
            metadata.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "created_at": "2026-01-01T00:00:00Z",
                        "archive": archive.name,
                        "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                    }
                ),
                encoding="utf-8",
            )
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
            metadata.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "created_at": "2026-01-01T00:00:00Z",
                        "archive": "../outside",
                        "sha256": "0" * 64,
                    }
                ),
                encoding="utf-8",
            )
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
        self.assertTrue(
            {
                "backup_disk_free_low",
                "backup_unverifiable",
                "tls_status_unknown",
                "wal_receiver_stopped",
            }.issubset(codes)
        )

    def test_database_metrics_failure_and_pressure_are_critical(self) -> None:
        policy = {
            "required_services": [],
            "minimum_disk_free_bytes": 10,
            "minimum_tls_seconds_remaining": 10,
            "maximum_database_connection_ratio": 0.85,
            "maximum_backup_age_seconds": 0,
            "maximum_wal_age_seconds": 0,
        }
        base_snapshot = {
            "collected_epoch": 1_000_000,
            "services": [],
            "disk_free_bytes": 1000,
            "backup_configured": False,
            "tls_seconds_remaining": 1000,
            "latest_backup_created_at": "1970-01-01T00:00:00Z",
            "latest_wal_received_at": "1970-01-01T00:00:00Z",
            "wal_receiver_running": True,
            "core_health": True,
            "qdrant_health": True,
            "ai_gateway_health": True,
        }

        unknown = dict(base_snapshot)
        unknown.update(
            {
                "database_connections": -1,
                "database_connection_limit": -1,
            }
        )
        unknown_alerts = ops.evaluate_alerts(unknown, policy)
        self.assertIn(
            ("database_metrics_unknown", "critical"),
            {(alert["code"], alert["severity"]) for alert in unknown_alerts},
        )

        pressure = dict(base_snapshot)
        pressure.update(
            {
                "database_connections": 85,
                "database_connection_limit": 100,
            }
        )
        pressure_alerts = ops.evaluate_alerts(pressure, policy)
        self.assertIn(
            ("database_connections_high", "critical"),
            {(alert["code"], alert["severity"]) for alert in pressure_alerts},
        )

    def degraded_snapshot(self, policy: dict[str, object]) -> dict[str, object]:
        return {
            "collected_epoch": 0,
            "services": [
                {"Service": service, "State": "running", "Health": "healthy"}
                for service in policy["required_services"]
            ],
            "disk_free_bytes": int(policy["minimum_disk_free_bytes"]) + 1,
            "backup_configured": False,
            "tls_seconds_remaining": int(policy["minimum_tls_seconds_remaining"]) + 1,
            "database_connections": 1,
            "database_connection_limit": 100,
            "latest_backup_created_at": "1970-01-01T00:00:00Z",
            "latest_wal_received_at": "1970-01-01T00:00:00Z",
            "wal_receiver_running": True,
            "core_health": True,
            "qdrant_health": True,
            "ai_gateway_health": True,
        }

    def test_ai_gateway_outage_is_warning_only(self) -> None:
        policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
        self.assertNotIn("ai-gateway", policy["required_services"])
        snapshot = self.degraded_snapshot(policy)
        snapshot["ai_gateway_health"] = False
        alerts = ops.evaluate_alerts(snapshot, policy)
        self.assertIn(
            ("ai_gateway_unavailable", "warning"),
            {(alert["code"], alert["severity"]) for alert in alerts},
        )
        self.assertFalse(any(alert["severity"] == "critical" for alert in alerts))

    def test_qdrant_outage_is_warning_only(self) -> None:
        policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
        self.assertNotIn("qdrant", policy["required_services"])
        snapshot = self.degraded_snapshot(policy)
        snapshot["qdrant_health"] = False
        alerts = ops.evaluate_alerts(snapshot, policy)
        self.assertIn(
            ("qdrant_unavailable", "warning"),
            {(alert["code"], alert["severity"]) for alert in alerts},
        )
        self.assertFalse(any(alert["severity"] == "critical" for alert in alerts))


class OperationsScriptBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.script = SCRIPT_PATH.read_text(encoding="utf-8")
        self.workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        self.web_main = WEB_MAIN_PATH.read_text(encoding="utf-8")

    def function(self, name: str, next_name: str) -> str:
        return self.script.split(f"{name}() {{", 1)[1].split(
            f"\n{next_name}() {{", 1
        )[0]

    def workflow_step(self, name: str, next_name: str) -> str:
        return self.workflow.split(f"      - name: {name}", 1)[1].split(
            f"\n      - name: {next_name}", 1
        )[0]

    def test_backup_quiesces_and_resumes_exact_writer_containers(self) -> None:
        backup = self.function("backup_create", "backup_verify")
        self.assertIn("compose ps --services --filter status=running", backup)
        self.assertIn("compose stop --timeout 30", backup)
        self.assertIn("quiesced_container_ids=()", backup)
        self.assertIn('id="$(compose ps --quiet "${service}"', backup)
        self.assertIn('quiesced_container_ids+=("${id}")', backup)
        self.assertIn('docker start "${quiesced_container_ids[@]}"', backup)
        self.assertNotIn('compose start "${quiesced_services[@]}"', backup)
        self.assertIn(".State.Health.Status", backup)
        self.assertIn("Writer-facing service did not recover", backup)
        self.assertLess(
            backup.index('quiesced_container_ids+=("${id}")'),
            backup.index("compose stop --timeout 30"),
        )
        self.assertIn("--no-deps --pull never --entrypoint sh app", backup)
        self.assertIn('"consistency": "write-quiesced"', backup)
        quiesce_case = backup.split('case "${service}" in', 1)[1].split(
            "esac", 1
        )[0]
        self.assertNotIn("db|", quiesce_case)
        self.assertNotIn("qdrant", quiesce_case)

    def test_backup_stages_only_on_a_capacity_checked_protected_path(self) -> None:
        preflight = self.function("backup_preflight", "backup_create")
        backup = self.function("backup_create", "backup_verify")
        self.assertIn("EDUTALENT_BACKUP_STAGING_DIR", self.script)
        self.assertIn("EDUTALENT_TEST_STAGING_AVAILABLE_BYTES", self.script)
        self.assertIn("Insufficient backup staging capacity", preflight)
        self.assertIn(
            'mktemp -d "${BACKUP_STAGING_ROOT}/edutalent-backup.XXXXXX"', backup
        )
        self.assertNotIn('temp="$(mktemp -d)"', backup)

    def test_backup_verifies_partial_before_publishing_final_names(self) -> None:
        backup = self.function("backup_create", "backup_verify")
        verify_partial = 'decrypt_backup "${archive}.partial" "${verify_temp}"'
        publish_archive = 'mv "${archive}.partial" "${archive}"'
        self.assertIn("backup_committed=false", backup)
        self.assertIn(verify_partial, backup)
        self.assertIn(publish_archive, backup)
        self.assertLess(backup.index(verify_partial), backup.index(publish_archive))
        self.assertIn('rm -f -- "${archive}" "${metadata_file}"', backup)
        self.assertLess(
            backup.index(publish_archive),
            backup.index("backup_committed=true"),
        )

    def test_qdrant_snapshot_is_downloaded_authenticated_and_deleted(self) -> None:
        backup = self.function("backup_create", "backup_verify")
        self.assertIn("delete_qdrant_snapshot()", backup)
        self.assertIn("--request DELETE", backup)
        self.assertIn("qdrant_snapshot_name", backup)
        self.assertGreaterEqual(
            backup.count('delete_qdrant_snapshot "${qdrant_snapshot_name}"'), 2
        )
        self.assertIn('qdrant_snapshot_name=""', backup)
        self.assertIn(
            'http://qdrant:6333/collections/${QDRANT_COLLECTION}/snapshots/${EDUTALENT_QDRANT_SNAPSHOT_NAME}',
            backup,
        )
        self.assertIn('--header "api-key: ${QDRANT_API_KEY}"', backup)
        self.assertIn('--volume "${payload}/qdrant:/backup"', backup)
        self.assertIn('[ ! -s "${destination}.partial" ]', backup)
        self.assertNotIn("compose cp \"qdrant:/qdrant/storage/collections/", backup)

    def test_live_acceptance_exercises_existing_qdrant_collection(self) -> None:
        step = self.workflow_step(
            "Start WAL reception and create encrypted full backup",
            "Run sustained load while exercising database and application recovery",
        )
        create_collection = 'http://qdrant:6333/collections/${QDRANT_COLLECTION}'
        self.assertIn(create_collection, step)
        self.assertIn('${QDRANT_VECTOR_SIZE}', step)
        self.assertIn('--request PUT', step)
        self.assertLess(step.index(create_collection), step.index("backup-create"))

    def test_recovery_load_uses_database_backed_readiness(self) -> None:
        step = self.workflow_step(
            "Run sustained load while exercising database and application recovery",
            "Run final production boundaries and alert evaluation",
        )
        self.assertIn("https://app.ops.internal/readyz", step)
        self.assertNotIn("https://app.ops.internal/healthz", step)
        self.assertIn('.route("/readyz", axum::routing::get(database_readiness))', self.web_main)
        self.assertIn('sqlx::query_scalar::<_, i32>("SELECT 1")', self.web_main)
        self.assertIn("StatusCode::SERVICE_UNAVAILABLE", self.web_main)

    def test_verified_full_backup_retires_only_included_wal_with_a_tail(self) -> None:
        backup = self.function("backup_create", "backup_verify")
        self.assertIn("included_wal_files=()", backup)
        self.assertIn("EDUTALENT_WAL_RETAIN_SEGMENTS", self.script)
        self.assertIn("retire_included_wal_segments()", backup)
        self.assertIn('rm -f -- "${included_wal_files[$index]}"', backup)
        self.assertLess(
            backup.index("backup_committed=true"),
            backup.rindex("retire_included_wal_segments"),
        )

    def test_backup_verify_and_restore_require_the_adjacent_sidecar_first(self) -> None:
        sidecar = self.function("verify_backup_sidecar", "backup_preflight")
        verify = self.function("backup_verify", "restore_drill")
        restore = self.function("restore_drill", "pitr_start")
        self.assertIn('metadata="${archive}.metadata.json"', sidecar)
        self.assertIn("backup-metadata-verify", sidecar)
        self.assertIn('--backup-dir "${BACKUP_ROOT}"', sidecar)
        for body in (verify, restore):
            self.assertIn('verify_backup_sidecar "${archive}"', body)
            self.assertLess(
                body.index('verify_backup_sidecar "${archive}"'),
                body.index('decrypt_backup "${archive}"'),
            )

    def test_restore_cleanup_removes_every_plaintext_copy(self) -> None:
        restore = self.function("restore_drill", "pitr_start")
        exit_trap = "trap 'cleanup_drill \"$?\"; exit \"$?\"' EXIT"
        self.assertEqual(restore.count(exit_trap), 1)
        self.assertIn("trap - EXIT", restore)
        self.assertIn("cleanup_drill 0", restore)
        self.assertIn('rm -f "${container_dump}"', restore)
        self.assertIn('rm -rf -- "${temp}"', restore)
        self.assertNotIn("trap cleanup_drill RETURN", restore)
        self.assertNotIn("trap 'rm -rf", restore)

    def test_restore_replay_uses_only_the_protected_admin_identity(self) -> None:
        restore = self.function("restore_drill", "pitr_start")
        self.assertIn("restore_role=supabase_admin", restore)
        self.assertIn("SELECT current_user, rolsuper::text", restore)
        self.assertIn("supabase_admin|true|false", restore)
        self.assertIn('-U "${restore_role}" -d "${drill_db}"', restore)
        self.assertIn("--no-owner --no-acl --exit-on-error", restore)
        self.assertIn('-U postgres "${drill_db}"', restore)
        self.assertIn('-U postgres --if-exists "${drill_db}"', restore)
        self.assertNotIn('pg_restore -h 127.0.0.1 -U postgres', restore)

    def test_acceptance_creates_pitr_and_backup_before_alerts(self) -> None:
        acceptance = self.function("acceptance", "prune_backups")
        self.assertLess(
            acceptance.index("pitr_verify"), acceptance.index("backup_create")
        )
        self.assertLess(
            acceptance.index("backup_create"), acceptance.index("collect_snapshot")
        )
        self.assertLess(
            acceptance.index("collect_snapshot"), acceptance.index("evaluate_alerts")
        )

    def test_workflow_rejects_every_critical_alert_status(self) -> None:
        self.assertNotIn("|| test $? -eq 2", self.workflow)
        self.assertIn(
            'bash deploy/production/edutalent-operations alerts "${snapshot}" 2>&1 | tee alerts-live.log',
            self.workflow,
        )

    def test_final_production_boundaries_fail_fast(self) -> None:
        step = self.workflow_step(
            "Run final production boundaries and alert evaluation",
            "Stop WAL receiver",
        )
        self.assertIn("set -euo pipefail", step)
        for command in (
            "production-database-check",
            "production-gateway-check",
            "production-qdrant-check",
            "production-ai-check",
        ):
            self.assertIn(command, step)
        self.assertNotIn("|| true", step)

    def test_snapshot_reads_the_gateway_prepared_certificate(self) -> None:
        snapshot = self.function("collect_snapshot", "evaluate_alerts")
        self.assertIn(
            'compose cp gateway:/etc/caddy/tls/fullchain.pem "${gateway_cert}"',
            snapshot,
        )
        self.assertIn('gateway_cert="${temp}/gateway-fullchain.pem"', snapshot)
        self.assertNotIn('read_env "${APP_ENV}" TLS_CERT_FILE', snapshot)

    def test_snapshot_verifies_backup_disk_archive_and_receiver(self) -> None:
        snapshot = self.function("collect_snapshot", "evaluate_alerts")
        self.assertIn("backup-metadata-verify", snapshot)
        self.assertIn("backup_disk_free_bytes", snapshot)
        self.assertIn("wal_receiver_running", snapshot)
        self.assertIn("tls_remaining=-1", snapshot)

    def test_temporary_patch_workflows_are_absent(self) -> None:
        for name in (
            "apply-backup-resume-fix.yml",
            "apply-backup-mode-fix.yml",
        ):
            self.assertFalse((REPOSITORY_ROOT / ".github/workflows" / name).exists())


if __name__ == "__main__":
    unittest.main()
