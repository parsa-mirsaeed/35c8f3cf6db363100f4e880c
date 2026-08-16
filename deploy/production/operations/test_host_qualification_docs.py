from __future__ import annotations

import json
import unittest
from pathlib import Path

OPERATIONS_DIR = Path(__file__).resolve().parent
PRODUCTION_DIR = OPERATIONS_DIR.parent
BASELINE = PRODUCTION_DIR / "host-baseline.json"
HOST_DOC = PRODUCTION_DIR / "HOST_BASELINE.md"
CIS_DOC = OPERATIONS_DIR / "CIS_DOCKER_1_8_0.md"
ACCEPTANCE = OPERATIONS_DIR / "TARGET_HOST_ACCEPTANCE.md"
SYSTEMD_README = PRODUCTION_DIR / "systemd" / "README.md"


class HostQualificationDocumentationTests(unittest.TestCase):
    def test_machine_baseline_is_explicitly_single_node(self) -> None:
        value = json.loads(BASELINE.read_text(encoding="utf-8"))
        self.assertEqual(value["schema_version"], 1)
        self.assertEqual(value["operating_system"]["id"], "ubuntu")
        self.assertEqual(value["operating_system"]["version_id"], "24.04")
        self.assertEqual(value["availability"]["architecture"], "single-node")
        self.assertFalse(value["availability"]["high_availability"])
        self.assertEqual(value["security_benchmarks"]["cis_docker"], "1.8.0")

    def test_host_document_does_not_claim_ha_or_contractual_rpo_rto(self) -> None:
        text = HOST_DOC.read_text(encoding="utf-8")
        self.assertIn("single-node and is not highly available", text)
        self.assertIn("Actual RPO/RTO are measured acceptance results, not SLA guarantees", text)
        self.assertIn("CI evidence alone cannot satisfy PR-13's target-host exit gate", text)

    def test_cis_map_disclaims_certification_and_requires_host_evidence(self) -> None:
        text = CIS_DOC.read_text(encoding="utf-8")
        self.assertIn("does not claim CIS certification or complete conformance", text)
        self.assertIn("pass", text)
        self.assertIn("fail", text)
        self.assertIn("not-applicable", text)
        self.assertIn("accepted-risk", text)
        self.assertIn("--require-digests", text)

    def test_target_acceptance_is_not_prefilled_as_pass(self) -> None:
        text = ACCEPTANCE.read_text(encoding="utf-8")
        self.assertIn("not pre-filled as PASS by CI", text)
        self.assertIn("Measured RPO", text)
        self.assertIn("Measured RTO", text)
        self.assertIn("School-scale load / soak", text)
        self.assertIn("single-node and not highly available", text)
        self.assertIn("ready for contracted production", text)
        self.assertNotIn("Automatic result: PASS\n", text)

    def test_systemd_installation_requires_separate_passphrase_escrow(self) -> None:
        text = SYSTEMD_README.read_text(encoding="utf-8")
        self.assertIn("escrow must remain separate from backup media", text)
        self.assertIn("host_preflight.py", text)
        self.assertIn("--require-operations", text)


if __name__ == "__main__":
    unittest.main()
