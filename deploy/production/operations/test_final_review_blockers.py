from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

OPERATIONS_DIR = Path(__file__).resolve().parent
PRODUCTION_DIR = OPERATIONS_DIR.parent
SCRIPT_PATH = PRODUCTION_DIR / "edutalent-operations"
RECOVERY_PATH = OPERATIONS_DIR / "recovery_drill.py"

spec = importlib.util.spec_from_file_location(
    "edutalent_recovery_final_review", RECOVERY_PATH
)
assert spec and spec.loader
recovery = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = recovery
spec.loader.exec_module(recovery)


class FinalBackupBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.script = SCRIPT_PATH.read_text(encoding="utf-8")

    def function(self, name: str, next_name: str) -> str:
        return self.script.split(f"{name}() {{", 1)[1].split(
            f"\n{next_name}() {{", 1
        )[0]

    def test_backup_cleanup_is_registered_for_shell_exit_before_quiescence(self) -> None:
        backup = self.function("backup_create", "backup_verify")
        exit_trap = "trap 'cleanup_backup \"$?\"; exit \"$?\"' EXIT"
        self.assertIn(exit_trap, backup)
        self.assertNotIn("trap cleanup_backup RETURN", backup)
        self.assertIn("trap - EXIT", backup)
        self.assertIn("cleanup_backup 0", backup)
        self.assertLess(backup.index(exit_trap), backup.index("compose stop --timeout 30"))
        self.assertLess(
            backup.rindex("cleanup_backup 0"),
            backup.index('echo "Created and verified encrypted backup'),
        )

    def test_verify_and_restore_use_the_canonical_verified_archive(self) -> None:
        sidecar = self.function("verify_backup_sidecar", "backup_preflight")
        verify = self.function("backup_verify", "restore_drill")
        restore = self.function("restore_drill", "pitr_start")
        self.assertIn('archive="$(canonical_path "${requested}")"', sidecar)
        self.assertIn('"$(dirname "${archive}")" != "${BACKUP_ROOT}"', sidecar)
        self.assertIn("direct file under the configured backup root", sidecar)
        for body in (verify, restore):
            self.assertIn('archive="$(verify_backup_sidecar "${archive}")"', body)
            self.assertLess(
                body.index('archive="$(verify_backup_sidecar "${archive}")"'),
                body.index('decrypt_backup "${archive}"'),
            )

    def test_backup_verify_rejects_an_archive_outside_the_configured_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            backup_root = root / "backups"
            staging_root = root / "staging"
            outside_root = root / "outside"
            for path in (backup_root, staging_root, outside_root):
                path.mkdir()
            passphrase = root / "passphrase"
            passphrase.write_text("test-only-passphrase\n", encoding="utf-8")
            passphrase.chmod(0o600)
            archive = outside_root / "edutalent-backup-test.tar.gz.enc"
            archive.write_bytes(b"different-generation")
            metadata = Path(f"{archive}.metadata.json")
            metadata.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "created_at": "2026-08-01T00:00:00Z",
                        "archive": archive.name,
                        "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                    }
                ),
                encoding="utf-8",
            )
            env = os.environ.copy()
            env.update(
                {
                    "EDUTALENT_BACKUP_DIR": str(backup_root),
                    "EDUTALENT_BACKUP_STAGING_DIR": str(staging_root),
                    "EDUTALENT_BACKUP_PASSPHRASE_FILE": str(passphrase),
                    "EDUTALENT_BACKUP_MIN_FREE_BYTES": "0",
                    "EDUTALENT_BACKUP_STAGING_MIN_FREE_BYTES": "0",
                }
            )
            completed = subprocess.run(
                ["bash", str(SCRIPT_PATH), "backup-verify", str(archive)],
                check=False,
                capture_output=True,
                text=True,
                env=env,
            )
            self.assertNotEqual(completed.returncode, 0, completed.stdout)
            self.assertIn(
                "direct file under the configured backup root", completed.stderr
            )


class PinnedSupabaseRecoveryTests(unittest.TestCase):
    def test_compose_image_parser_resolves_the_database_service(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            compose = Path(directory) / "docker-compose.yml"
            compose.write_text(
                "services:\n"
                "  api:\n"
                "    image: example/api:1\n"
                "  db:\n"
                "    container_name: supabase-db\n"
                "    image: supabase/postgres:17.6.1.136\n"
                "    restart: unless-stopped\n"
                "  pooler:\n"
                "    image: example/pooler:1\n",
                encoding="utf-8",
            )
            self.assertEqual(
                recovery.parse_compose_service_image(compose, "db"),
                "supabase/postgres:17.6.1.136",
            )

    def test_physical_recovery_uses_the_materialized_production_runtime(self) -> None:
        text = RECOVERY_PATH.read_text(encoding="utf-8")
        self.assertNotIn("postgres:17-alpine", text)
        for required in (
            "production-bootstrap",
            "SUPABASE_UPSTREAM",
            'PRODUCTION_DIR / "runtime" / "supabase"',
            "supabase/postgres:",
            "volumes/db/roles.sql",
            "volumes/db/jwt.sql",
            "volumes/db/_supabase.sql",
            "pg_hba.conf",
            "/etc/postgresql-custom",
            "config_file=/etc/postgresql/postgresql.conf",
            "hba_file=/etc/postgresql/pg_hba.conf",
        ):
            self.assertIn(required, text)


if __name__ == "__main__":
    unittest.main()
