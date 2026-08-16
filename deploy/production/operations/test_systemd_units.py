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
        "edutalent-restore-verify.service",
        "edutalent-wal.service",
        "edutalent-wal-verify.service",
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

    def test_maintenance_units_use_external_state_and_backup_paths(self) -> None:
        for name in self.SERVICES:
            text = (SYSTEMD_DIR / name).read_text(encoding="utf-8")
            with self.subTest(name=name):
                self.assertIn(
                    "ReadWritePaths=/var/lib/edutalent/operations /mnt/edutalent-backup",
                    text,
                )
                self.assertIn("EnvironmentFile=/etc/edutalent/operations.env", text)

    def test_required_timers_are_persistent_and_bounded(self) -> None:
        expectations = {
            "edutalent-monitor.timer": "OnUnitActiveSec=1min",
            "edutalent-backup.timer": "OnCalendar=*-*-* 02:15:00",
            "edutalent-restore-verify.timer": "OnCalendar=Sun *-*-* 04:30:00",
            "edutalent-wal-verify.timer": "OnUnitActiveSec=10min",
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

    def test_restore_verification_uses_latest_verified_backup_helper(self) -> None:
        unit = (SYSTEMD_DIR / "edutalent-restore-verify.service").read_text(
            encoding="utf-8"
        )
        helper_path = SYSTEMD_DIR / "run-latest-restore-drill"
        helper = helper_path.read_text(encoding="utf-8")
        self.assertIn("run-latest-restore-drill", unit)
        self.assertIn("set -euo pipefail", helper)
        self.assertIn("edutalent-backup-*.tar.gz.enc.metadata.json", helper)
        self.assertIn('exec bash "${OPERATIONS_COMMAND}" restore-drill "${archive}"', helper)
        completed = subprocess.run(
            ["bash", "-n", str(helper_path)],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)


if __name__ == "__main__":
    unittest.main()
