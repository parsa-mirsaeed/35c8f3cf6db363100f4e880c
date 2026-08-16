from __future__ import annotations

import subprocess
import unittest
from pathlib import Path

OPERATIONS_DIR = Path(__file__).resolve().parent
PRODUCTION_DIR = OPERATIONS_DIR.parent
SYSTEMD_DIR = PRODUCTION_DIR / "systemd"


class SystemdMaintenanceUnitTests(unittest.TestCase):
    SERVICES = (
        "edutalent-monitor.service",
        "edutalent-backup.service",
        "edutalent-offhost-copy.service",
        "edutalent-restore-verify.service",
        "edutalent-wal.service",
        "edutalent-wal-verify.service",
        "edutalent-offhost-wal.service",
    )

    def test_services_run_as_dedicated_unprivileged_operator(self) -> None:
        for name in self.SERVICES:
            text = (SYSTEMD_DIR / name).read_text(encoding="utf-8")
            with self.subTest(name=name):
                self.assertIn("User=edutalent-operator", text)
                self.assertIn("Group=edutalent-operator", text)
                self.assertIn("NoNewPrivileges=true", text)
                self.assertIn("ProtectSystem=strict", text)
                self.assertIn("ProtectHome=true", text)
                self.assertIn("PrivateTmp=true", text)
                self.assertIn("PrivateDevices=true", text)
                self.assertIn("CapabilityBoundingSet=\n", text)
                self.assertIn("AmbientCapabilities=\n", text)
                self.assertIn("UMask=0077", text)
                self.assertNotIn("sudo", text)
                self.assertNotIn("docker.service", text)

    def test_maintenance_units_use_external_state_and_backup_paths(self) -> None:
        for name in self.SERVICES:
            text = (SYSTEMD_DIR / name).read_text(encoding="utf-8")
            with self.subTest(name=name):
                self.assertIn("EnvironmentFile=/etc/edutalent/operations.env", text)
        for name in (
            "edutalent-monitor.service",
            "edutalent-backup.service",
            "edutalent-offhost-copy.service",
            "edutalent-restore-verify.service",
            "edutalent-wal.service",
            "edutalent-wal-verify.service",
        ):
            text = (SYSTEMD_DIR / name).read_text(encoding="utf-8")
            with self.subTest(name=name):
                self.assertIn("/var/lib/edutalent/operations", text)
        offhost_backup = (SYSTEMD_DIR / "edutalent-offhost-copy.service").read_text(
            encoding="utf-8"
        )
        offhost_wal = (SYSTEMD_DIR / "edutalent-offhost-wal.service").read_text(
            encoding="utf-8"
        )
        self.assertIn("/mnt/edutalent-offhost", offhost_backup)
        self.assertIn("/mnt/edutalent-offhost", offhost_wal)

    def test_required_timers_are_persistent_and_bounded(self) -> None:
        expectations = {
            "edutalent-monitor.timer": "OnUnitActiveSec=1min",
            "edutalent-backup.timer": "OnCalendar=*-*-* 02:15:00",
            "edutalent-offhost-copy.timer": "OnCalendar=*-*-* 03:15:00",
            "edutalent-restore-verify.timer": "OnCalendar=Sun *-*-* 04:30:00",
            "edutalent-wal-verify.timer": "OnUnitActiveSec=10min",
            "edutalent-offhost-wal.timer": "OnUnitActiveSec=10min",
        }
        for name, cadence in expectations.items():
            text = (SYSTEMD_DIR / name).read_text(encoding="utf-8")
            with self.subTest(name=name):
                self.assertIn("Persistent=true", text)
                self.assertIn(cadence, text)
                self.assertIn("WantedBy=timers.target", text)

    def test_wal_receiver_starts_at_boot_and_has_explicit_stop(self) -> None:
        text = (SYSTEMD_DIR / "edutalent-wal.service").read_text(encoding="utf-8")
        self.assertIn("RemainAfterExit=true", text)
        self.assertIn("pitr-start", text)
        self.assertIn("pitr-stop", text)
        self.assertIn("WantedBy=multi-user.target", text)

    def test_shell_helpers_are_syntax_valid_and_are_invoked_through_bash(self) -> None:
        units = {
            "edutalent-restore-verify.service": "run-latest-restore-drill",
            "edutalent-offhost-copy.service": "run-latest-offhost-copy",
            "edutalent-offhost-wal.service": "run-offhost-wal-sync",
        }
        for unit_name, helper_name in units.items():
            unit = (SYSTEMD_DIR / unit_name).read_text(encoding="utf-8")
            helper_path = SYSTEMD_DIR / helper_name
            helper = helper_path.read_text(encoding="utf-8")
            with self.subTest(unit=unit_name):
                self.assertIn(f"ExecStart=/bin/bash /opt/edutalent/deploy/production/systemd/{helper_name}", unit)
                self.assertIn("set -euo pipefail", helper)
                completed = subprocess.run(
                    ["bash", "-n", str(helper_path)],
                    check=False,
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_restore_verification_selects_latest_verified_backup(self) -> None:
        helper = (SYSTEMD_DIR / "run-latest-restore-drill").read_text(
            encoding="utf-8"
        )
        self.assertIn("edutalent-backup-*.tar.gz.enc.metadata.json", helper)
        self.assertIn('exec bash "${OPERATIONS_COMMAND}" restore-drill "${archive}"', helper)

    def test_offhost_copy_does_not_reference_passphrase(self) -> None:
        helper = (SYSTEMD_DIR / "run-latest-offhost-copy").read_text(encoding="utf-8")
        self.assertIn("sha256sum", helper)
        self.assertIn(".partial.$$.tmp", helper)
        self.assertNotIn("PASSPHRASE", helper)

    def test_offhost_wal_sync_only_copies_completed_segments(self) -> None:
        helper = (SYSTEMD_DIR / "run-offhost-wal-sync").read_text(encoding="utf-8")
        self.assertIn("[0-9A-F]{24}", helper)
        self.assertIn("sha256sum", helper)
        self.assertIn("latest WAL segment", helper)
        self.assertNotIn("pgpass", helper.lower())


if __name__ == "__main__":
    unittest.main()
