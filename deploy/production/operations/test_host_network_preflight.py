from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path

OPERATIONS_DIR = Path(__file__).resolve().parent
PRODUCTION_DIR = OPERATIONS_DIR.parent
MODULE_PATH = PRODUCTION_DIR / "host_network_preflight.py"
BASELINE_PATH = PRODUCTION_DIR / "host-baseline.json"

spec = importlib.util.spec_from_file_location("edutalent_host_network_preflight", MODULE_PATH)
assert spec and spec.loader
network_preflight = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = network_preflight
spec.loader.exec_module(network_preflight)


class HostNetworkPreflightTests(unittest.TestCase):
    def setUp(self) -> None:
        self.baseline = json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
        self.facts = {
            "app_env_present": True,
            "domains": {
                "app.school.example": True,
                "supabase.school.example": True,
                "admin.school.example": True,
            },
            "configured_domain_count": 3,
            "required_ports": [80, 443],
            "occupied_required_ports": [],
            "ntp_synchronized": True,
            "clock_skew_seconds": 0.125,
        }

    def evaluate(self):
        return network_preflight.evaluate(self.baseline, dict(self.facts))

    def test_resolved_dns_free_ports_and_bounded_skew_pass(self) -> None:
        result = self.evaluate()
        self.assertEqual(result["automatic_status"], "pass")
        self.assertEqual(result["failed_check_ids"], [])
        self.assertEqual(result["manual_status"], "pass")

    def test_missing_domain_fails_contract(self) -> None:
        self.facts["configured_domain_count"] = 2
        self.facts["domains"].pop("admin.school.example")
        result = self.evaluate()
        self.assertIn("dns-domain-count", result["failed_check_ids"])
        self.assertIn("dns-resolution", result["failed_check_ids"])

    def test_unresolved_domain_fails(self) -> None:
        self.facts["domains"]["admin.school.example"] = False
        result = self.evaluate()
        self.assertIn("dns-resolution", result["failed_check_ids"])

    def test_required_public_port_occupied_before_start_fails(self) -> None:
        self.facts["occupied_required_ports"] = [443]
        result = self.evaluate()
        self.assertIn("prestart-public-ports-free", result["failed_check_ids"])

    def test_port_contract_cannot_be_silently_changed(self) -> None:
        self.facts["required_ports"] = [8080, 8443]
        result = self.evaluate()
        self.assertIn("required-port-contract", result["failed_check_ids"])

    def test_known_unsynchronized_clock_fails(self) -> None:
        self.facts["ntp_synchronized"] = False
        result = self.evaluate()
        self.assertIn("time-synchronization", result["failed_check_ids"])

    def test_excessive_measured_clock_skew_fails(self) -> None:
        self.facts["clock_skew_seconds"] = 61.0
        result = self.evaluate()
        self.assertIn("clock-skew", result["failed_check_ids"])

    def test_unobservable_skew_is_manual_not_false_pass(self) -> None:
        self.facts["clock_skew_seconds"] = None
        result = self.evaluate()
        self.assertEqual(result["automatic_status"], "pass")
        self.assertIn("clock-skew", result["pending_manual_check_ids"])


if __name__ == "__main__":
    unittest.main()
