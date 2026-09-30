"""Operator commands must reject incomplete intent before any database call."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).parents[1]
WORKSPACE = "cccccccc-cccc-4ccc-8ccc-cccccccccccc"
PROJECT = "dddddddd-dddd-4ddd-8ddd-dddddddddddd"
ACTOR = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee"
ENV = {
    "AI_CENTER_OPERATOR_DATABASE_URL": "postgresql://postgres:FICTIF-only@127.0.0.1:55322/postgres",
    "AI_CENTER_ERASURE_ALLOWED_HOST": "127.0.0.1",
    "AI_CENTER_ERASURE_ALLOWED_PORT": "55322",
    "AI_CENTER_ERASURE_ALLOWED_DATABASE": "postgres",
}


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    with patch.object(sys, "path", [str(SCRIPTS), *sys.path]):
        spec.loader.exec_module(module)
    return module


class ProjectMaintenanceTests(unittest.TestCase):
    def invoke(self, name, args, extra=None):
        module = load(name)
        result = subprocess.CompletedProcess([], 0, '{"format":"[FICTIF]"}', '')
        with patch.dict(os.environ, ENV | (extra or {}), clear=True), patch.object(sys, "argv", [name, *args]), patch.object(module.erasure_common.subprocess, "run", return_value=result) as run, contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            try:
                status = module.main()
            except SystemExit as error:
                status = error.code
        return status, run

    def test_project_preview_has_both_exact_ids_and_no_mutation(self):
        status, run = self.invoke("project-erasure", ["preview", WORKSPACE, PROJECT])
        self.assertEqual(status, 0)
        sql = run.call_args.kwargs["input"]
        self.assertIn("read only", sql)
        self.assertIn(f"'{WORKSPACE}'::uuid,'{PROJECT}'::uuid", sql)
        self.assertNotIn("operator_purge_project", sql)

    def test_project_execution_needs_project_not_company_confirmation(self):
        args = ["execute", WORKSPACE, PROJECT, "--receipt", "a" * 32, "--confirm"]
        status, run = self.invoke("project-erasure", [*args, WORKSPACE], {"AI_CENTER_ALLOW_PROJECT_ERASURE": "yes"})
        self.assertEqual(status, 2)
        run.assert_not_called()
        status, run = self.invoke("project-erasure", [*args, PROJECT], {"AI_CENTER_ALLOW_PROJECT_ERASURE": "yes"})
        self.assertEqual(status, 0)
        self.assertIn("operator_purge_project", run.call_args.kwargs["input"])

    def test_abandoned_execution_requires_stopped_workers_and_named_operator(self):
        args = ["execute", WORKSPACE, "--receipt", "b" * 32, "--confirm", WORKSPACE]
        for incomplete in [args, [*args, "--workers-stopped"], [*args, "--operator-actor", ACTOR]]:
            status, run = self.invoke("close-abandoned-work", incomplete, {"AI_CENTER_ALLOW_ABANDONED_MAINTENANCE": "yes"})
            self.assertEqual(status, 2)
            run.assert_not_called()
        status, run = self.invoke("close-abandoned-work", [*args, "--workers-stopped", "--operator-actor", ACTOR], {"AI_CENTER_ALLOW_ABANDONED_MAINTENANCE": "yes"})
        self.assertEqual(status, 0)
        self.assertIn(f"true,'{ACTOR}'::uuid", run.call_args.kwargs["input"])

    def test_target_pin_and_receipt_injection_are_rejected_for_both_commands(self):
        for name, args in [("project-erasure", ["preview", WORKSPACE, PROJECT]), ("close-abandoned-work", ["preview", WORKSPACE])]:
            status, run = self.invoke(name, args, {"AI_CENTER_ERASURE_ALLOWED_PORT": "5432"})
            self.assertEqual(status, 2)
            run.assert_not_called()
        status, run = self.invoke("project-erasure", ["execute", WORKSPACE, PROJECT, "--confirm", PROJECT, "--receipt", "');select 1;--"], {"AI_CENTER_ALLOW_PROJECT_ERASURE": "yes"})
        self.assertEqual(status, 2)
        run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
