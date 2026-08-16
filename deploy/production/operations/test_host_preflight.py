from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

OPERATIONS_DIR = Path(__file__).resolve().parent
PRODUCTION_DIR = OPERATIONS_DIR.parent
MODULE_PATH = PRODUCTION_DIR / "host_preflight.py"
BASELINE_PATH = PRODUCTION_DIR / "host-baseline.json"

spec = importlib.util.spec_from_file_location("edutalent_host_preflight", MODULE_PATH)
assert spec and spec.loader
host_preflight = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = host_preflight
spec.loader.exec_module(host_preflight)


class HostPreflightTests(unittest.TestCase):
    def setUp(self) -> None:
        self.baseline = json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
        self.facts = {
            "system": "linux",
            "os_id": "ubuntu",
            "os_version_id": "24.04",
            "architecture": "x86_64",
            "kernel": "6.8.0-55-generic",
            "cpu_count": 8,
            "memory_bytes": 16 * 1024**3,
            "root_free_bytes": 80 * 1024**3,
            "root_free_inodes": 5_000_000,
            "root_fstype": "ext4",
            "docker_engine_version": "28.3.2",
            "docker_compose_version": "2.39.1",
            "docker_rootless": True,
            "operator_uid": 1001,
            "ntp_synchronized": True,
            "firewall_detected": True,
            "production_device": 100,
            "backup_dir": "/mnt/edutalent-backup",
            "backup_device": 200,
            "backup_free_bytes": 100 * 1024**3,
            "backup_writable": True,
            "backup_passphrase_file": "/etc/edutalent/backup.passphrase",
            "backup_passphrase_mode": "0600",
        }

    def evaluate(self, *, require_operations: bool = False):
        return host_preflight.evaluate(
            self.baseline,
            dict(self.facts),
            require_operations=require_operations,
        )

    def test_compliant_automatic_baseline_passes(self) -> None:
        result = self.evaluate(require_operations=True)
        self.assertEqual(result["automatic_status"], "pass")
        self.assertEqual(result["failed_check_ids"], [])
        self.assertIn("data-at-rest-encryption", result["pending_manual_check_ids"])
        self.assertIn("backup-passphrase-escrow", result["pending_manual_check_ids"])
        self.assertIn("off-host-copy", result["pending_manual_check_ids"])

    def test_unsupported_os_fails_closed(self) -> None:
        self.facts["os_version_id"] = "22.04"
        result = self.evaluate()
        self.assertEqual(result["automatic_status"], "fail")
        self.assertIn("supported-os", result["failed_check_ids"])

    def test_capacity_and_filesystem_shortfalls_fail(self) -> None:
        self.facts["memory_bytes"] = 4 * 1024**3
        self.facts["root_free_bytes"] = 1024**3
        self.facts["root_free_inodes"] = 10
        self.facts["root_fstype"] = "overlay"
        result = self.evaluate()
        for expected in (
            "minimum-memory",
            "minimum-root-free-space",
            "minimum-root-free-inodes",
            "supported-root-filesystem",
        ):
            self.assertIn(expected, result["failed_check_ids"])

    def test_root_operator_is_rejected(self) -> None:
        self.facts["operator_uid"] = 0
        result = self.evaluate()
        self.assertIn("unprivileged-operator", result["failed_check_ids"])

    def test_unsynchronized_clock_is_rejected(self) -> None:
        self.facts["ntp_synchronized"] = False
        result = self.evaluate()
        self.assertIn("time-synchronization", result["failed_check_ids"])

    def test_unknown_time_state_requires_manual_evidence_without_false_pass(self) -> None:
        self.facts["ntp_synchronized"] = None
        result = self.evaluate()
        self.assertEqual(result["automatic_status"], "pass")
        self.assertIn("time-synchronization", result["pending_manual_check_ids"])

    def test_rootful_docker_requires_tailored_review(self) -> None:
        self.facts["docker_rootless"] = False
        result = self.evaluate()
        self.assertEqual(result["automatic_status"], "pass")
        self.assertIn("docker-daemon-mode", result["pending_manual_check_ids"])

    def test_operations_mode_requires_separate_backup_filesystem(self) -> None:
        self.facts["backup_device"] = self.facts["production_device"]
        result = self.evaluate(require_operations=True)
        self.assertIn("backup-separate-filesystem", result["failed_check_ids"])

    def test_operations_mode_rejects_unsafe_passphrase_mode(self) -> None:
        self.facts["backup_passphrase_mode"] = "0644"
        result = self.evaluate(require_operations=True)
        self.assertIn("backup-passphrase-mode", result["failed_check_ids"])

    def test_baseline_explicitly_denies_ha_claim(self) -> None:
        host_preflight.validate_baseline(self.baseline)
        self.assertFalse(self.baseline["availability"]["high_availability"])
        self.assertEqual(self.baseline["availability"]["architecture"], "single-node")

    def test_definition_only_contract_is_machine_readable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "baseline.json"
            baseline = json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
            host_preflight.validate_baseline(baseline)
            output.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "baseline_id": baseline["baseline_id"],
                        "definition_status": "pass",
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(json.loads(output.read_text())["definition_status"], "pass")


if __name__ == "__main__":
    unittest.main()
