from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from unittest import mock

MODULE_PATH = Path(__file__).with_name("recovery_drill.py")
spec = importlib.util.spec_from_file_location("recovery_drill_wal_test", MODULE_PATH)
assert spec and spec.loader
recovery = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = recovery
spec.loader.exec_module(recovery)


class RecoveryWalArchiveTests(unittest.TestCase):
    def test_switch_waits_for_the_exact_returned_segment(self) -> None:
        expected = "00000001000000000000002A"
        with mock.patch.object(recovery, "psql", return_value=expected) as psql_call:
            with mock.patch.object(recovery, "wait_for_archive_file") as wait_call:
                observed = recovery.switch_wal_and_wait(
                    "source",
                    "secret",
                    "archive-volume",
                    "supabase/postgres:17.6.1.136",
                )

        self.assertEqual(observed, expected)
        psql_call.assert_called_once_with(
            "source",
            "secret",
            "SELECT pg_walfile_name(pg_switch_wal());",
            user="supabase_admin",
        )
        wait_call.assert_called_once_with(
            "archive-volume",
            "supabase/postgres:17.6.1.136",
            expected,
        )

    def test_archive_wait_rejects_unsafe_or_non_segment_names(self) -> None:
        for value in ("../segment", "00000001000000000000002a", "0" * 23, "0" * 25):
            with self.subTest(value=value):
                with self.assertRaisesRegex(RuntimeError, "invalid WAL archive filename"):
                    recovery.wait_for_archive_file("volume", "image", value)

    def test_drill_no_longer_accepts_global_archive_counts(self) -> None:
        source = MODULE_PATH.read_text(encoding="utf-8")
        drill = source.split("def postgres_drill(", 1)[1].split("\ndef http_json(", 1)[0]
        self.assertNotIn("minimum_files", source)
        self.assertNotIn("wait_for_archive(", source)
        self.assertEqual(drill.count("switch_wal_and_wait("), 2)
        self.assertIn("before_target_wal", drill)
        self.assertIn("after_target_wal", drill)
        self.assertIn("forced WAL switches returned the same archive segment", drill)
        self.assertIn('"archived_wal_segments"', drill)
        self.assertIn('test -f "/archive/$1"', source)


if __name__ == "__main__":
    unittest.main()
