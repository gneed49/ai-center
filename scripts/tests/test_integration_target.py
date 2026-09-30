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
                    if not name.startswith(("AI_CENTER_", "DATABASE_URL", "SUPABASE_"))}
        self.env["DOCKER_HOST"] = "unix:///tmp/integration-fictitious.sock"
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

    def test_prepare_writes_only_the_official_pin_and_versions_its_marker(self):
        development_temp = self.repo / "supabase/.temp"
        development_temp.mkdir()
        (development_temp / "postgres-version").write_text("unexpected")
        (development_temp / "project-ref").write_text("do-not-copy")
        prepared = self.prepare()
        self.assertEqual((prepared / "supabase/.temp/postgres-version").read_text(), "17.11.0.002\n")
        self.assertEqual(json.loads((prepared / "target.json").read_text()), target.marker(self.repo))
        self.assertEqual([p.name for p in (prepared / "supabase/.temp").iterdir()], ["postgres-version"])
        self.assertEqual((development_temp / "postgres-version").read_text(), "unexpected")

    def test_intact_legacy_workdir_can_stop_then_prepare_without_database_access(self):
        prepared = self.prepare()
        (prepared / "target.json").write_text(json.dumps(target.marker(self.repo, 1)))
        (prepared / "supabase/.temp/postgres-version").unlink()
        with self.assertRaises(target.TargetError):
            target.guard(self.repo, self.env)
        stopped = self.command("integration-stack.sh", "stop")
        self.assertEqual(stopped.returncode, 0, stopped.stderr)
        with mock.patch.object(target.subprocess, "run") as command:
            self.prepare()
            command.assert_not_called()
        target.guard(self.repo, self.env)

    def test_new_marker_missing_or_changed_pin_is_not_silently_repaired(self):
        prepared = self.prepare()
        pin = prepared / "supabase/.temp/postgres-version"
        for value in ("17.6.1.158\n", "17.11.0.002@sha256:forged\n", "17.11.0.002\n\n", None):
            with self.subTest(value=value):
                if value is None:
                    pin.unlink()
                else:
                    pin.write_text(value)
                for check in (target.guard, target.prepare):
                    with self.assertRaises(target.TargetError):
                        check(self.repo, self.env)
        self.assertFalse(self.log.exists())

    def test_pin_and_temp_directory_symlinks_are_refused_before_write(self):
        prepared = self.prepare()
        pin = prepared / "supabase/.temp/postgres-version"
        outside = self.repo / "keep.txt"
        outside.write_text("keep")
        pin.unlink()
        pin.symlink_to(outside)
        with self.assertRaises(target.TargetError):
            self.prepare()
        self.assertEqual(outside.read_text(), "keep")
        pin.unlink()
        pin.parent.rmdir()
        pin.parent.symlink_to(self.repo, target_is_directory=True)
        with self.assertRaises(target.TargetError):
            self.prepare()
        self.assertFalse((self.repo / "postgres-version").exists())

    def test_image_overrides_and_linked_projects_are_refused_before_cli(self):
        prepared = self.prepare()
        for name, value in (("SUPABASE_DB_MAJOR_VERSION", "15"),
                            ("SUPABASE_EXPERIMENTAL_ORIOLEDB_VERSION", "17.11.0.002"),
                            ("SUPABASE_CLI_BINARY_OVERRIDE", "/unexpected"),
                            ("SUPABASE_INTERNAL_IMAGE_REGISTRY", "example.invalid"),
                            ("SUPABASE_EXPERIMENTAL_STACK", "true"),
                            ("SUPABASE_USE_SLIM_IMAGES", "true")):
            with self.subTest(name=name):
                result = self.command("integration-stack.sh", "prepare", **{name: value})
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(self.log.exists())
        (prepared / "supabase/.temp/project-ref").write_text("foreign")
        with self.assertRaises(target.TargetError):
            target.guard(self.repo, self.env)

    def postgres_fixture(self, architecture="amd64"):
        self.prepare()
        detail = {"name": "/supabase_db_" + target.project_id(self.repo),
                  "image_id": "sha256:" + "a" * 64,
                  "image": "public.ecr.aws/supabase/postgres:17.11.0.002",
                  "labels": {"com.supabase.cli.project": target.project_id(self.repo)}, "running": True}
        metadata = {"id": detail["image_id"], "architecture": architecture, "os": "linux",
                    "digests": ["public.ecr.aws/supabase/postgres@" + target.POSTGRES_PLATFORM_DIGESTS[architecture]]}
        return detail, metadata

    def verify_fixture(self, detail, metadata, version="170011"):
        results = [subprocess.CompletedProcess([], 0, json.dumps(detail), ""),
                   subprocess.CompletedProcess([], 0, json.dumps(metadata), ""),
                   subprocess.CompletedProcess([], 0, version, "")]
        return mock.patch.object(target.subprocess, "run", side_effect=results)

    def test_running_postgres_attestation_checks_digest_and_version_without_private_inspect(self):
        for architecture in ("amd64", "arm64"):
            with self.subTest(architecture=architecture):
                detail, metadata = self.postgres_fixture(architecture)
                with self.verify_fixture(detail, metadata) as command:
                    proof = target.verify_postgres(self.repo, self.env)
                self.assertEqual(proof["architecture"], architecture)
                self.assertEqual(proof["server_version_num"], "170011")
                self.assertEqual(proof["repo_digest"], metadata["digests"][0])
                commands = [call.args[0] for call in command.call_args_list]
                self.assertEqual(commands[-1][-1], "show server_version_num")
                for args in commands[:2]:
                    self.assertIn("--format", args)
                    self.assertNotIn(".Env", " ".join(args))
                proof_path = self.repo / ".run/integration-stack/postgres-image.json"
                self.assertEqual(json.loads(proof_path.read_text()), proof)
                self.assertEqual(proof_path.stat().st_mode & 0o777, 0o600)

    def test_foreign_stopped_or_old_container_never_reaches_image_or_sql_checks(self):
        for changes in ({"name": "/supabase_db_AICenter"}, {"running": False},
                        {"labels": {"com.supabase.cli.project": "other"}},
                        {"image": "supabase/postgres:17.6.1.158"}):
            with self.subTest(changes=changes):
                detail, metadata = self.postgres_fixture()
                with self.verify_fixture({**detail, **changes}, metadata) as command:
                    with self.assertRaises(target.TargetError):
                        target.verify_postgres(self.repo, self.env)
                    self.assertEqual(command.call_count, 1)

    def test_podman_attestation_uses_the_same_explicit_socket_as_supabase(self):
        detail, metadata = self.postgres_fixture()
        with mock.patch.object(target.shutil, "which", return_value=None):
            with self.verify_fixture(detail, metadata) as command:
                target.verify_postgres(self.repo, self.env)
        for call in command.call_args_list:
            self.assertEqual(call.args[0][:4],
                ["podman", "--remote", "--url", "unix:///tmp/integration-fictitious.sock"])

    def test_unverified_digest_architecture_and_image_id_never_reach_sql(self):
        for changes in ({"digests": []}, {"digests": ["supabase/postgres@sha256:" + "0" * 64]},
                        {"architecture": "riscv64"}, {"id": "sha256:" + "b" * 64}):
            with self.subTest(changes=changes):
                detail, metadata = self.postgres_fixture()
                with self.verify_fixture(detail, {**metadata, **changes}) as command:
                    with self.assertRaises(target.TargetError):
                        target.verify_postgres(self.repo, self.env)
                    self.assertEqual(command.call_count, 2)

    def test_old_effective_server_version_does_not_write_a_success_receipt(self):
        detail, metadata = self.postgres_fixture()
        with self.verify_fixture(detail, metadata):
            target.verify_postgres(self.repo, self.env)
        with self.verify_fixture(detail, metadata, "170006"):
            with self.assertRaises(target.TargetError):
                target.verify_postgres(self.repo, self.env)
        self.assertFalse((self.repo / ".run/integration-stack/postgres-image.json").exists())
        # A failed image/version check must not prevent exact-target recovery.
        stopped = self.command("integration-stack.sh", "stop")
        self.assertEqual(stopped.returncode, 0, stopped.stderr)

    def test_reset_rechecks_running_postgres_before_the_next_sql_phase(self):
        detail, metadata = self.postgres_fixture()
        docker = self.bin / "docker"
        docker.write_text("""#!/usr/bin/env python3
import json, os, pathlib, sys
args=sys.argv[1:]
with open(os.environ['INTEGRATION_RUNTIME_LOG'], 'a') as out:
 out.write(json.dumps(args)+'\\n')
if args[0]=='container': print(os.environ['INTEGRATION_CONTAINER'])
elif args[0]=='image': print(os.environ['INTEGRATION_IMAGE'])
elif args[0]=='exec': print(os.environ['INTEGRATION_SERVER_VERSION'])
else: raise SystemExit(90)
""")
        docker.chmod(0o755)
        runtime_log = self.repo / "runtime-called.jsonl"
        env = {**self.env, "PATH": str(self.bin) + os.pathsep + self.env["PATH"],
               "INTEGRATION_RUNTIME_LOG": str(runtime_log),
               "INTEGRATION_CONTAINER": json.dumps(detail), "INTEGRATION_IMAGE": json.dumps(metadata)}
        script = ("source scripts/integration-common.sh\nintegration_environment\n"
                  "integration_supabase db reset --local --yes\nprintf 'NEXT_SQL_PHASE\\n'\n")
        for version, succeeds in (("170011", True), ("170006", False)):
            with self.subTest(version=version):
                runtime_log.unlink(missing_ok=True)
                result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", script],
                    cwd=self.repo, env={**env, "INTEGRATION_SERVER_VERSION": version},
                    text=True, capture_output=True, timeout=15)
                self.assertEqual(result.returncode == 0, succeeds, result.stderr)
                self.assertEqual("NEXT_SQL_PHASE" in result.stdout, succeeds)
                calls = [json.loads(line) for line in runtime_log.read_text().splitlines()]
                self.assertEqual([args[0] for args in calls], ["container", "image", "exec"])
                self.assertEqual(calls[-1][-1], "show server_version_num")

    def test_attestation_refuses_a_symlink_receipt_before_contacting_runtime(self):
        prepared = self.prepare()
        outside = self.repo / "keep-proof.txt"
        outside.write_text("keep")
        (prepared / "postgres-image.json").symlink_to(outside)
        with mock.patch.object(target.subprocess, "run") as command:
            with self.assertRaises(target.TargetError):
                target.verify_postgres(self.repo, self.env)
            command.assert_not_called()
        self.assertEqual(outside.read_text(), "keep")

    def test_failed_or_timed_out_verification_does_not_repeat_private_output(self):
        self.prepare()
        for effect in (subprocess.CompletedProcess([], 1, "PRIVATE", "PRIVATE"),
                       subprocess.TimeoutExpired("PRIVATE", 20, output="PRIVATE")):
            with self.subTest(effect=type(effect).__name__):
                kwargs = {"side_effect": effect} if isinstance(effect, Exception) else {"return_value": effect}
                with mock.patch.object(target.subprocess, "run", **kwargs):
                    with self.assertRaises(target.TargetError) as caught:
                        target.verify_postgres(self.repo, self.env)
                self.assertNotIn("PRIVATE", str(caught.exception))

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
