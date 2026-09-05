from __future__ import annotations

import fcntl
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock


REPO = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("integration_target", REPO / "scripts/integration-target.py")
assert SPEC and SPEC.loader
target = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(target)


class IntegrationTargetTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.repo = Path(self.temporary.name).resolve() / "checkout"
        self.repo.mkdir()
        (self.repo / "supabase").mkdir()
        for name in ("roles.sql", "seed.sql", "config.toml", "config.integration.toml",
                     "migrations", "schemas", "tests"):
            source = REPO / "supabase" / name
            destination = self.repo / "supabase" / name
            if source.is_dir():
                shutil.copytree(source, destination)
            else:
                shutil.copyfile(source, destination)
        shutil.copytree(REPO / "scripts", self.repo / "scripts", ignore=shutil.ignore_patterns("__pycache__"))
        self.env = {name: value for name, value in os.environ.items()
                    if not name.startswith(("AI_CENTER_", "DATABASE_URL"))}
        self.log = self.repo / "called.jsonl"
        self.env["INTEGRATION_TEST_LOG"] = str(self.log)
        self.bin = self.repo / "node_modules/.bin"
        self.bin.mkdir(parents=True)
        fake_cli = self.bin / "supabase"
        fake_cli.write_text("""#!/usr/bin/env python3
import json, os, sys
with open(os.environ['INTEGRATION_TEST_LOG'], 'a') as out:
    out.write(json.dumps(sys.argv[1:]) + '\\n')
if 'start' in sys.argv:
    raise SystemExit(91)
if 'status' in sys.argv:
    print(json.dumps({'DB_URL': 'postgresql://postgres:postgres@127.0.0.1:55322/postgres',
                      'API_URL': 'http://127.0.0.1:55321'}))
""")
        fake_cli.chmod(0o755)

    def command(self, script, *args, **env):
        return subprocess.run(["bash", str(self.repo / "scripts" / script), *args],
                              cwd=self.repo, env={**self.env, **env}, text=True,
                              capture_output=True, timeout=15)

    def prepare(self):
        return target.prepare(self.repo, self.env)

    def test_prepared_stack_copies_sql_without_development_environment(self):
        (self.repo / ".env.local").write_text("DO_NOT_COPY=true\n")
        (self.repo / "supabase/.env").write_text("DO_NOT_COPY=true\n")
        original = (self.repo / "supabase/config.toml").read_bytes()
        prepared = self.prepare()
        self.assertEqual(target.guard(self.repo, self.env), prepared)
        self.assertEqual((self.repo / "supabase/config.toml").read_bytes(), original)
        self.assertFalse((prepared / "supabase/.env").exists())
        self.assertEqual((prepared / ".env").read_text(), "")
        self.assertEqual((prepared / ".env.local").read_text(), "")
        self.assertEqual((prepared / "supabase/seed.sql").read_bytes(),
                         (self.repo / "supabase/seed.sql").read_bytes())

    def test_checkouts_have_distinct_container_identities(self):
        self.assertNotEqual(target.project_id(self.repo), target.project_id(self.repo.parent / "other"))

    def test_prepare_removes_obsolete_test_migrations_only(self):
        prepared = self.prepare()
        stale = prepared / "supabase/migrations/removed.sql"
        stale.write_text("select 1;")
        self.prepare()
        self.assertFalse(stale.exists())
        self.assertTrue((self.repo / "supabase/migrations/20260818003330_initial_domain.sql").exists())

    def test_rejects_development_workdir_before_write(self):
        with self.assertRaises(target.TargetError):
            target.prepare(self.repo, {"AI_CENTER_INTEGRATION_WORKDIR": str(self.repo)})
        self.assertFalse((self.repo / "target.json").exists())

    def test_rejects_symlink_to_development_before_write(self):
        (self.repo / ".run").mkdir()
        (self.repo / ".run/integration-stack").symlink_to(self.repo, target_is_directory=True)
        with self.assertRaises(target.TargetError):
            self.prepare()
        self.assertFalse((self.repo / "target.json").exists())

    def test_rejects_config_mutated_to_development_before_stop(self):
        prepared = self.prepare()
        shutil.copyfile(self.repo / "supabase/config.toml", prepared / "supabase/config.toml")
        result = self.command("integration-stack.sh", "stop")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.log.exists(), result.stderr)

    def test_rejects_missing_or_foreign_marker(self):
        prepared = self.prepare()
        marker = prepared / "target.json"
        marker.write_text(json.dumps({"version": 1, "repo": "/other", "project_id": "AICenter"}))
        with self.assertRaises(target.TargetError):
            target.guard(self.repo, self.env)
        marker.unlink()
        with self.assertRaises(OSError):
            target.guard(self.repo, self.env)

    def test_rejects_development_port_even_when_port_override_agrees(self):
        with self.assertRaises(target.TargetError):
            target.validate_env({
                "AI_CENTER_ADMIN_DATABASE_URL": "postgresql://postgres:postgres@127.0.0.1:54322/postgres",
                "AI_CENTER_LOCAL_POSTGRES_PORT": "54322",
            })

    def test_rejects_remote_querystring_wrong_role_or_database(self):
        for value in (
            "postgresql://postgres:postgres@example.invalid:55322/postgres",
            "postgresql://postgres:postgres@127.0.0.1:55322/postgres?host=example.invalid",
            "postgresql://other:postgres@127.0.0.1:55322/postgres",
            "postgresql://postgres:postgres@127.0.0.1:55322/development",
        ):
            with self.subTest(value=value), self.assertRaises(target.TargetError):
                target.validate_env({"AI_CENTER_ADMIN_DATABASE_URL": value})

    def test_guard_rejects_development_before_any_database_or_cli_command(self):
        self.prepare()
        for script, args in (("ci-desktop.sh", ["integration"]),
                             ("baseline-alpha-upgrade-smoke.sh", []),
                             ("postgres-backup-restore-smoke.sh", []),
                             ("auth-magic-link-smoke.sh", [])):
            with self.subTest(script=script):
                result = self.command(script, *args,
                    AI_CENTER_ADMIN_DATABASE_URL="postgresql://postgres:postgres@127.0.0.1:54322/postgres")
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(self.log.exists(), result.stderr)
                self.assertNotIn("postgresql://", result.stderr)

    def test_status_must_confirm_both_database_and_api(self):
        good = {"DB_URL": "postgresql://postgres:postgres@127.0.0.1:55322/postgres",
                "API_URL": "http://127.0.0.1:55321"}
        target.validate_status(good)
        for name in good:
            with self.subTest(name=name), self.assertRaises(target.TargetError):
                target.validate_status({**good, name: "development"})

    def test_stop_names_only_the_owned_project(self):
        self.prepare()
        result = self.command("integration-stack.sh", "stop")
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = [json.loads(line) for line in self.log.read_text().splitlines()]
        self.assertEqual(calls, [["--workdir", str(self.repo / ".run/integration-stack"),
                                "stop", "--project-id", target.project_id(self.repo), "--no-backup"]])

    def test_stop_refuses_to_interrupt_an_active_run(self):
        self.prepare()
        with (self.repo / ".run/integration-stack.lock").open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            result = self.command("integration-stack.sh", "stop")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.log.exists())

    def test_prepare_refuses_port_overlap_with_development(self):
        config = self.repo / "supabase/config.toml"
        config.write_text(config.read_text().replace("port = 54322", "port = 55322"))
        with self.assertRaises(target.TargetError):
            self.prepare()

    def podman_fixture(self, *, label_override=None, remaining_container=False, unknown_name=False):
        prepared = self.prepare()
        (prepared / "stop-cli.log").write_text(json.dumps({
            "error": {"code": "LegacyStopVolumePruneError"},
        }))
        identity = target.project_id(self.repo)
        names = [f"supabase_db_{identity}", f"supabase_storage_{identity}"]
        if unknown_name:
            names.append("supabase_db_AICenter")
        deleted = []
        calls = []

        def command(argv, **_kwargs):
            self.assertEqual(argv[:4], ["podman", "--remote", "--url", "unix:///tmp/ci-podman.sock"])
            args = argv[4:]
            calls.append(args)
            result = ""
            if args[0] == "info":
                result = "5.8.4"
            elif args[0] == "ps" and remaining_container:
                result = "container-still-present"
            elif args[:2] == ["volume", "ls"]:
                result = "\n".join(name for name in names if name not in deleted)
            elif args[:2] == ["volume", "inspect"]:
                result = json.dumps([{"Name": args[2], "Labels": {
                    "com.supabase.cli.project": label_override or identity,
                }}])
            elif args[:2] == ["volume", "rm"]:
                self.assertNotIn("--force", args)
                deleted.append(args[2])
            return subprocess.CompletedProcess(argv, 0, result, "")

        return {**self.env, "DOCKER_HOST": "unix:///tmp/ci-podman.sock"}, command, deleted, calls

    def test_podman_fallback_deletes_only_exact_unattached_owned_volumes(self):
        env, command, deleted, calls = self.podman_fixture()
        with mock.patch.object(target.subprocess, "run", side_effect=command):
            target.cleanup_podman(self.repo, env)
        self.assertEqual(deleted, [f"supabase_db_{target.project_id(self.repo)}",
                                   f"supabase_storage_{target.project_id(self.repo)}"])
        self.assertFalse(any("prune" in call or "--force" in call for call in calls))

    def test_podman_fallback_refuses_remaining_containers_foreign_labels_and_names(self):
        for options in ({"remaining_container": True}, {"label_override": "AICenter"}, {"unknown_name": True}):
            with self.subTest(options=options):
                env, command, deleted, _ = self.podman_fixture(**options)
                with mock.patch.object(target.subprocess, "run", side_effect=command):
                    with self.assertRaises(target.TargetError):
                        target.cleanup_podman(self.repo, env)
                self.assertEqual(deleted, [])

    def test_podman_fallback_refuses_other_errors_and_remote_endpoints(self):
        env, _, _, _ = self.podman_fixture()
        with mock.patch.object(target.subprocess, "run") as command:
            with self.assertRaises(target.TargetError):
                target.cleanup_podman(self.repo, {**env, "DOCKER_HOST": "tcp://example.invalid:2375"})
            (self.repo / ".run/integration-stack/stop-cli.log").write_text(json.dumps({
                "error": {"code": "SomeOtherStopError"},
            }))
            with self.assertRaises(target.TargetError):
                target.cleanup_podman(self.repo, env)
            command.assert_not_called()


if __name__ == "__main__":
    unittest.main()
