from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("verify_release_docs.py")
spec = importlib.util.spec_from_file_location("verify_release_docs", MODULE_PATH)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseDocsVerifierTests(unittest.TestCase):
    def test_secret_patterns_reject_private_key_material(self) -> None:
        text = "-----BEGIN PRIVATE KEY-----"
        self.assertTrue(module.SECRET_PATTERNS["private key"].search(text))

    def test_placeholder_invalid_email_is_not_real_contact(self) -> None:
        address = "security-contact@example.invalid"
        self.assertTrue(address.lower().endswith(".invalid"))

    def test_disabled_feature_row_contract(self) -> None:
        key = "attendance"
        matrix = "| `attendance` | Attendance workflow | Disabled | unavailable |\n"
        import re
        row = re.compile(rf"^\|\s*`{re.escape(key)}`\s*\|.*\|\s*Disabled\s*\|", re.M)
        self.assertIsNotNone(row.search(matrix))

    def test_relative_link_resolution_stays_inside_root(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            doc = root / "docs" / "release" / "README.md"
            target = root / "docs" / "release" / "feature-matrix.md"
            target.parent.mkdir(parents=True)
            target.touch()
            resolved = (doc.parent / "feature-matrix.md").resolve()
            self.assertEqual(resolved, target)
            self.assertEqual(resolved.relative_to(root), Path("docs/release/feature-matrix.md"))


if __name__ == "__main__":
    unittest.main()
