#!/usr/bin/env python3
"""Exercise SQLx TLS against only this checkout's guarded disposable PostgreSQL."""
import importlib.util
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    result = subprocess.run(args, capture_output=True, text=True, timeout=90, **kwargs)
    if result.returncode:
        # Neither command arguments nor process output can expose credentials.
        operation = next((item for item in ("psql", "mkdir", "chown", "chmod", "cp", "inspect") if item in args), args[1])
        raise RuntimeError(f"TLS fixture command failed: {Path(args[0]).name}, operation {operation}, status {result.returncode}")
    return result.stdout


def main():
    spec = importlib.util.spec_from_file_location("integration_target", REPO / "scripts/integration-target.py")
    target = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(target)
    target.guard(REPO, dict(os.environ))
    container = "supabase_db_" + target.project_id(REPO)
    runtime = "docker" if shutil.which("docker") else "podman"
    # Inspect the exact checkout-derived identity before changing its TLS settings.
    detail = json.loads(run([runtime, "inspect", container]))[0]
    if detail.get("Name", "").lstrip("/") != container:
        raise RuntimeError("Disposable container identity mismatch")
    remote = "/tmp/ai-center-tls-" + secrets.token_hex(8)

    def sql(statement):
        return run([runtime, "exec", container, "psql", "-X", "-v", "ON_ERROR_STOP=1", "-U", "supabase_admin", "-d", "postgres", "-At", "-c", statement]).strip()

    original = json.loads(sql("select json_object_agg(name,setting) from pg_settings where name in ('ssl','ssl_cert_file','ssl_key_file')"))
    with tempfile.TemporaryDirectory(prefix="aicenter-tls-") as directory:
        folder = Path(directory)
        run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1", "-subj", "/CN=AI Center fictional CI CA", "-addext", "basicConstraints=critical,CA:TRUE", "-keyout", str(folder / "ca.key"), "-out", str(folder / "ca.crt")])
        run(["openssl", "req", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=localhost", "-keyout", str(folder / "server.key"), "-out", str(folder / "server.csr")])
        (folder / "extensions").write_text("subjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n")
        run(["openssl", "x509", "-req", "-in", str(folder / "server.csr"), "-CA", str(folder / "ca.crt"), "-CAkey", str(folder / "ca.key"), "-CAcreateserial", "-days", "1", "-extfile", str(folder / "extensions"), "-out", str(folder / "server.crt")])
        try:
            run([runtime, "exec", "--user", "0", container, "mkdir", "-m", "700", remote])
            for name in ("server.crt", "server.key"):
                run([runtime, "cp", str(folder / name), f"{container}:{remote}/{name}"])
            run([runtime, "exec", "--user", "0", container, "chown", "-R", "postgres:postgres", remote])
            run([runtime, "exec", "--user", "0", container, "chmod", "600", remote + "/server.key"])
            for name, value in (("ssl_cert_file", remote + "/server.crt"), ("ssl_key_file", remote + "/server.key"), ("ssl", "on")):
                sql(f"alter system set {name}='{value}'")
            sql("select pg_reload_conf()")
            for _ in range(20):
                if sql("show ssl_cert_file") == remote + "/server.crt":
                    break
                time.sleep(0.1)
            env = dict(os.environ, AI_CENTER_TLS_TEST_ROOT=str(folder / "ca.crt"))
            result = subprocess.run(["cargo", "test", "-p", "ai-center-server", "--lib", "database_transport::tests::real_postgres_tls", "--", "--ignored", "--test-threads=1"], cwd=REPO, env=env, timeout=180)
            if result.returncode:
                raise RuntimeError("PostgreSQL TLS assertions failed")
        finally:
            for name in ("ssl", "ssl_cert_file", "ssl_key_file"):
                value = original[name].replace("'", "''")
                sql(f"alter system set {name}='{value}'")
            sql("select pg_reload_conf()")
            run([runtime, "exec", "--user", "0", container, "rm", "-rf", "--", remote])
    print("PostgreSQL TLS: trusted CA accepted; untrusted CA and wrong hostname refused.")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        detail = str(error) if isinstance(error, RuntimeError) else type(error).__name__
        raise SystemExit(f"Guarded TLS smoke failed: {detail}; no connection details printed.") from None
