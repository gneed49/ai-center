"""No database call: verify the boundary before operator-only erasure."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("company_erasure", Path(__file__).parents[1] / "company-erasure.py")
MODULE = importlib.util.module_from_spec(SPEC)
with patch.object(sys, "path", [str(Path(__file__).parents[1]), *sys.path]):
    SPEC.loader.exec_module(MODULE)
WORKSPACE = "cccccccc-cccc-4ccc-8ccc-cccccccccccc"
ENV = {
    "AI_CENTER_OPERATOR_DATABASE_URL": "postgresql://postgres:FICTIF-only@127.0.0.1:55322/postgres",
    "AI_CENTER_ERASURE_ALLOWED_HOST": "127.0.0.1",
    "AI_CENTER_ERASURE_ALLOWED_PORT": "55322",
    "AI_CENTER_ERASURE_ALLOWED_DATABASE": "postgres",
}


class CompanyErasureTests(unittest.TestCase):
    def run_guard(self, args, environment, result=None):
        stdout, stderr = io.StringIO(), io.StringIO()
        result = result or subprocess.CompletedProcess([], 0, '{"format":"synthetic-receipt"}\n', '')
        with patch.dict(os.environ, environment, clear=True), patch.object(sys, 'argv', ['company-erasure.py', *args]), patch.object(MODULE.erasure_common.subprocess, 'run', return_value=result) as run, contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            try:
                status = MODULE.main()
            except SystemExit as error:
                status = error.code
        return status, stdout.getvalue(), stderr.getvalue(), run

    def test_target_mismatch_or_missing_execution_confirmation_never_calls_psql(self):
        for changed in [
            {"AI_CENTER_ERASURE_ALLOWED_HOST": "another.invalid"},
            {"AI_CENTER_ERASURE_ALLOWED_PORT": "5432"},
            {"AI_CENTER_ERASURE_ALLOWED_DATABASE": "another"},
            {"AI_CENTER_OPERATOR_DATABASE_URL": ENV["AI_CENTER_OPERATOR_DATABASE_URL"] + "?hostaddr=192.0.2.1"},
        ]:
            status, _, error, run = self.run_guard(['preview', WORKSPACE], ENV | changed)
            self.assertEqual(status, 2)
            run.assert_not_called()
            self.assertNotIn('FICTIF-only', error)
        status, _, _, run = self.run_guard(['execute', WORKSPACE], ENV)
        self.assertEqual(status, 2)
        run.assert_not_called()

    def test_preview_pins_libpq_target_and_never_puts_password_in_argv_or_sql(self):
        status, _, _, run = self.run_guard(['preview', WORKSPACE], ENV | {
            'PGHOSTADDR': '192.0.2.1', 'PGSERVICE': 'another', 'PGOPTIONS': '-c session_replication_role=replica',
        })
        self.assertEqual(status, 0)
        args, kwargs = run.call_args
        self.assertNotIn('FICTIF-only', repr(args))
        self.assertNotIn('FICTIF-only', kwargs['input'])
        self.assertNotIn('PGHOSTADDR', kwargs['env'])
        self.assertNotIn('PGSERVICE', kwargs['env'])
        self.assertEqual(kwargs['env']['PGHOST'], '127.0.0.1')
        self.assertEqual(kwargs['env']['PGPORT'], '55322')
        self.assertIn('read only', kwargs['input'])
        self.assertNotIn('replica', kwargs['env']['PGOPTIONS'])

    def test_explicit_execution_has_exact_uuid_and_remote_tls_verification(self):
        env = ENV | {
            'AI_CENTER_OPERATOR_DATABASE_URL': 'postgresql://postgres:FICTIF-only@database.example.invalid:6543/postgres',
            'AI_CENTER_ERASURE_ALLOWED_HOST': 'database.example.invalid',
            'AI_CENTER_ERASURE_ALLOWED_PORT': '6543',
            'AI_CENTER_ALLOW_COMPANY_ERASURE': 'yes', 'PGSSLROOTCERT': '/private/test-ca.crt',
        }
        status, _, _, run = self.run_guard(['execute', WORKSPACE, '--confirm', WORKSPACE, '--receipt', 'a' * 32], env)
        self.assertEqual(status, 0)
        kwargs = run.call_args.kwargs
        self.assertEqual(kwargs['env']['PGSSLMODE'], 'verify-full')
        self.assertEqual(kwargs['env']['PGSSLROOTCERT'], '/private/test-ca.crt')
        self.assertIn(f"'{WORKSPACE}'::uuid,'{WORKSPACE}'::uuid", kwargs['input'])
        self.assertIn('operator_purge_workspace', kwargs['input'])

    def test_failed_server_diagnostics_are_not_relayed(self):
        result = subprocess.CompletedProcess([], 1, '', 'server echoed FICTIF-only and sensitive row')
        status, output, error, _ = self.run_guard(['preview', WORKSPACE], ENV, result)
        self.assertEqual(status, 1)
        self.assertEqual(output, '')
        self.assertNotIn('sensitive row', error)
        self.assertNotIn('FICTIF-only', error)


if __name__ == '__main__':
    unittest.main()
