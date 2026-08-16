from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

OPERATIONS_DIR = Path(__file__).resolve().parent
PRODUCTION_DIR = OPERATIONS_DIR.parent
MODULE_PATH = PRODUCTION_DIR / "container_hardening_inventory.py"

spec = importlib.util.spec_from_file_location("edutalent_container_inventory", MODULE_PATH)
assert spec and spec.loader
inventory = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = inventory
spec.loader.exec_module(inventory)


class ContainerHardeningInventoryTests(unittest.TestCase):
    def compose(self, image: str = "example/app:1") -> dict:
        return {
            "services": {
                "app": {
                    "image": image,
                    "user": "10001:10001",
                    "read_only": True,
                    "cap_drop": ["ALL"],
                    "pids_limit": 256,
                    "security_opt": ["no-new-privileges:true"],
                    "restart": "unless-stopped",
                    "healthcheck": {"test": ["CMD", "true"]},
                    "deploy": {
                        "resources": {
                            "limits": {"cpus": "2", "memory": "2G"}
                        }
                    },
                    "networks": {"internal": {}},
                    "volumes": [
                        {"type": "volume", "source": "state", "target": "/state"},
                        {
                            "type": "bind",
                            "source": "/etc/example",
                            "target": "/config",
                            "read_only": True,
                        },
                    ],
                }
            }
        }

    def test_source_inventory_records_hardening_without_requiring_digest(self) -> None:
        result = inventory.build_inventory(self.compose(), require_digests=False)
        self.assertEqual(result["status"], "pass")
        row = result["services"][0]
        self.assertTrue(row["explicit_non_root_user"])
        self.assertTrue(row["read_only_root"])
        self.assertTrue(row["drops_all_capabilities"])
        self.assertTrue(row["no_new_privileges"])
        self.assertEqual(row["writable_paths"], ["/state"])
        self.assertEqual(row["read_only_paths"], ["/config"])
        self.assertEqual(result["summary"]["non_digest_images"], ["app"])

    def test_release_inventory_requires_sha256_digest(self) -> None:
        result = inventory.build_inventory(self.compose(), require_digests=True)
        self.assertEqual(result["status"], "fail")
        self.assertIn("non-digest-images", result["summary"]["failed_checks"])

        digest = "example/app@sha256:" + "a" * 64
        result = inventory.build_inventory(
            self.compose(image=digest), require_digests=True
        )
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["summary"]["non_digest_images"], [])

    def test_privileged_host_network_and_docker_socket_fail(self) -> None:
        document = self.compose(image="example/app@sha256:" + "b" * 64)
        app = document["services"]["app"]
        app["privileged"] = True
        app["network_mode"] = "host"
        app["volumes"].append(
            {
                "type": "bind",
                "source": "/var/run/docker.sock",
                "target": "/var/run/docker.sock",
            }
        )
        result = inventory.build_inventory(document, require_digests=True)
        self.assertEqual(result["status"], "fail")
        self.assertEqual(
            set(result["summary"]["failed_checks"]),
            {"privileged-services", "host-network-services", "docker-socket-mounts"},
        )


if __name__ == "__main__":
    unittest.main()
