from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPO = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("migration_plan", REPO / "scripts/migration-plan.py")
assert SPEC and SPEC.loader
plan = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(plan)


class MigrationPlanTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.baseline = "20260818003330_initial_domain.sql"
        self.later = "20260907155244_personal_provider_connections.sql"
        self.write(self.baseline)
        self.write(self.later)

    def write(self, name, content="select 1;\n"):
        (self.directory / name).write_text(content)

    def test_discovers_every_migration_in_version_order_including_future_files(self):
        self.write("20261001000000_following_change.sql")
        self.write("20260825121915_alpha_context_proof.sql")
        self.assertEqual(plan.discover(self.directory), [
            self.baseline, "20260825121915_alpha_context_proof.sql",
            self.later, "20261001000000_following_change.sql",
        ])

    def test_rejects_duplicate_versions(self):
        self.write("20260907155244_conflicting_change.sql")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            plan.discover(self.directory)

    def test_requires_the_expected_baseline_and_at_least_one_upgrade(self):
        (self.directory / self.baseline).unlink()
        with self.assertRaisesRegex(ValueError, "baseline"):
            plan.discover(self.directory)
        self.write(self.baseline)
        (self.directory / self.later).unlink()
        with self.assertRaisesRegex(ValueError, "upgrade"):
            plan.discover(self.directory)

    def test_rejects_a_file_before_the_known_baseline(self):
        self.write("20260801000000_older.sql")
        with self.assertRaisesRegex(ValueError, "baseline"):
            plan.discover(self.directory)

    def test_rejects_unrecognized_names_instead_of_silently_skipping_them(self):
        self.write("missing_timestamp.sql")
        with self.assertRaisesRegex(ValueError, "name"):
            plan.discover(self.directory)

    def test_rejects_symlinks_empty_files_and_directories(self):
        invalid = self.directory / "20260908000000_invalid.sql"
        invalid.symlink_to(self.directory / self.baseline)
        with self.assertRaisesRegex(ValueError, "regular"):
            plan.discover(self.directory)
        invalid.unlink()
        invalid.touch()
        with self.assertRaisesRegex(ValueError, "empty"):
            plan.discover(self.directory)
        invalid.unlink()
        invalid.mkdir()
        with self.assertRaisesRegex(ValueError, "regular"):
            plan.discover(self.directory)

    def test_shell_stops_on_migration_failure_and_cleans_only_its_created_database(self):
        repo = self.directory / "checkout"
        repo.mkdir()
        shutil.copytree(REPO / "supabase", repo / "supabase",
                        ignore=shutil.ignore_patterns(".temp", ".branches", ".env*"))
        shutil.copytree(REPO / "scripts", repo / "scripts",
                        ignore=shutil.ignore_patterns("__pycache__"))
        bin_dir = repo / "node_modules/.bin"
        bin_dir.mkdir(parents=True)
        log = repo / "calls.jsonl"
        env = {key: value for key, value in os.environ.items()
               if not key.startswith(("AI_CENTER_", "DATABASE_URL", "SUPABASE_"))}
        env.update(PATH=str(bin_dir) + os.pathsep + env["PATH"],
                   UPGRADE_TEST_LOG=str(log),
                   AI_CENTER_ADMIN_DATABASE_URL="postgresql://postgres:fixture@127.0.0.1:55322/postgres")
        for executable in ["supabase", "psql", "createdb", "dropdb"]:
            stub = bin_dir / executable
            stub.write_text("""#!/usr/bin/env python3
import json,os,pathlib,sys
name=pathlib.Path(sys.argv[0]).name
args=sys.argv[1:]
if name=='supabase':
 print(json.dumps({'DB_URL':'postgresql://postgres:postgres@127.0.0.1:55322/postgres',
                   'API_URL':'http://127.0.0.1:55321'}))
else:
 files=[arg.split('=',1)[1] for arg in args if arg.startswith('--file=')]
 with open(os.environ['UPGRADE_TEST_LOG'],'a') as log:
  log.write(json.dumps({'name':name,'files':files,'database':args[-1] if name in ('createdb','dropdb') else None})+'\\n')
 if files and files[0].endswith('20260905004751_review_observation_cycles_and_leases.sql'):
  raise SystemExit(93)
 if '--tuples-only' in args: print('0')
""")
            stub.chmod(0o755)
        prepared = subprocess.run(["python3", "scripts/integration-target.py", "prepare"],
                                  cwd=repo, env=env, capture_output=True, text=True, timeout=15)
        self.assertEqual(prepared.returncode, 0, prepared.stderr)
        result = subprocess.run(["bash", "scripts/baseline-alpha-upgrade-smoke.sh"],
                                cwd=repo, env=env, capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 93, result.stderr)
        self.assertNotIn("vérifié", result.stdout)
        calls = [json.loads(line) for line in log.read_text().splitlines()]
        migrations = [Path(path).name for call in calls for path in call["files"]
                      if path.startswith("supabase/migrations/")]
        self.assertEqual(migrations, [
            self.baseline, "20260825121915_alpha_context_proof.sql",
            "20260904235403_external_tracking_runtime_grants.sql",
            "20260905004751_review_observation_cycles_and_leases.sql",
        ])
        created = [call["database"] for call in calls if call["name"] == "createdb"]
        dropped = [call["database"] for call in calls if call["name"] == "dropdb"]
        self.assertEqual(created, dropped)
        self.assertEqual(len(created), 1)
        self.assertRegex(created[0], r"^ai_center_upgrade_[0-9]+_[0-9]+$")


if __name__ == "__main__":
    unittest.main()
