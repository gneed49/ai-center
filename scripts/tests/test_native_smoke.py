from __future__ import annotations

import json
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/native-smoke.py"
SPEC = importlib.util.spec_from_file_location("native_smoke", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
native = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(native)


class NativeSmokeTests(unittest.TestCase):
    def test_prepare_produces_only_an_isolated_loopback_overlay(self) -> None:
        product = REPO / "apps/desktop/src-tauri/tauri.conf.json"
        before = product.read_bytes()
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "prepare", "--output-dir", directory],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            overlay = json.loads((Path(directory) / "tauri.config.json").read_text())
            self.assertEqual(overlay["identifier"], "com.aicenter.client.smoke")
            csp = overlay["app"]["security"]["csp"]
            connect = next(
                part.strip()
                for part in csp.split(";")
                if part.strip().startswith("connect-src")
            )
            self.assertEqual(connect, "connect-src 'self' http://127.0.0.1:4617")
            self.assertNotIn("4317", csp)
            self.assertNotIn("*", csp)
            self.assertEqual(product.read_bytes(), before)
            self.assertEqual(overlay["build"]["beforeBuildCommand"], "")

    def test_run_refuses_an_unisolated_environment_before_launching_a_client(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "run",
                    "--app",
                    "/no/application",
                    "--output-dir",
                    directory,
                ],
                env={"PATH": os.defpath},
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn(
                "deterministic integration environment required", result.stderr
            )
            self.assertEqual(list(Path(directory).iterdir()), [])

    def test_child_environment_does_not_inherit_operator_credentials_or_profile(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = native.isolated_environment(
                Path(directory),
                {
                    "PATH": os.defpath,
                    "OPENAI_API_KEY": "do-not-inherit",
                    "DATABASE_URL": "do-not-inherit",
                    "DISPLAY": ":0",
                    "XDG_CONFIG_HOME": "/operator/config",
                    "DBUS_SESSION_BUS_ADDRESS": "operator-bus",
                },
            )
            for name in (
                "OPENAI_API_KEY",
                "DATABASE_URL",
                "DISPLAY",
                "DBUS_SESSION_BUS_ADDRESS",
            ):
                self.assertNotIn(name, environment)
            self.assertEqual(environment["GDK_BACKEND"], "x11")
            self.assertEqual(
                environment["XDG_CONFIG_HOME"], str(Path(directory) / "config")
            )

    def test_process_context_cleans_up_on_failure_without_native_tools(self) -> None:
        process = None
        with self.assertRaisesRegex(RuntimeError, "synthetic failure"):
            with native.ChildProcesses() as children:
                process = children.start(
                    [sys.executable, "-c", "import time; time.sleep(30)"]
                )
                raise RuntimeError("synthetic failure")
        self.assertIsNotNone(process)
        self.assertIsNotNone(process.poll())


if __name__ == "__main__":
    unittest.main()
