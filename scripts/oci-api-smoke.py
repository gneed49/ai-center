#!/usr/bin/env python3
"""Run the built API against the guarded local DB/Auth; no provider calls."""
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

REPO = Path(__file__).resolve().parents[1]


def main():
    subprocess.run(["python3", str(REPO / "scripts/integration-target.py"), "guard"], cwd=REPO, check=True)
    raw = os.environ["AI_CENTER_RUNTIME_DATABASE_URL"]
    url = urllib.parse.urlsplit(raw)
    if (url.hostname, url.port, url.username, url.path, url.query, url.fragment) != ("127.0.0.1", 55322, "ai_center_runtime", "/postgres", "", ""):
        raise RuntimeError("Exact guarded database target required")
    runtime = "docker" if shutil.which("docker") else "podman"
    image = os.environ.get("AI_CENTER_API_SMOKE_IMAGE", "localhost/ai-center-api:company-v1")
    name = "ai-center-api-smoke-" + uuid.uuid4().hex[:12]
    started = False

    def command(*args):
        result = subprocess.run([runtime, *args], capture_output=True, text=True, timeout=45)
        if result.returncode:
            raise RuntimeError("API image command failed; command output withheld")
        return result.stdout.strip()

    # Reserve only a loopback port. Host networking keeps the disposable DB local
    # to the API; it must never bind 0.0.0.0 during this smoke.
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    with tempfile.TemporaryDirectory(prefix="ai-center-api-smoke-") as directory:
        path = Path(directory) / "private.env"
        path.write_text("\n".join([
            "DATABASE_URL=" + raw,
            f"AI_CENTER_BIND=127.0.0.1:{port}",
            "AI_CENTER_AUTH_MODE=supabase", "AI_CENTER_AGENT_MODE=deterministic",
            "SUPABASE_URL=http://127.0.0.1:55321", "AI_CENTER_COMPANY_CREATORS=",
            "AI_CENTER_CORS_ORIGINS=http://127.0.0.1:5183",
        ]) + "\n")
        path.chmod(0o600)
        try:
            command("run", "--detach", "--name", name, "--pull=never", "--network=host",
                    "--read-only", "--cap-drop=ALL", "--security-opt=no-new-privileges:true",
                    "--tmpfs", "/tmp:rw,size=32m,mode=1777", "--env-file", str(path), image)
            started = True
            origin = f"http://127.0.0.1:{port}"
            for _ in range(100):
                try:
                    with urllib.request.urlopen(origin + "/api/health", timeout=1) as response:
                        health = json.load(response)
                        if health.get("status") == "ok" and health.get("database") == "connected":
                            break
                except (OSError, urllib.error.URLError):
                    time.sleep(0.1)
            else:
                raise RuntimeError("API image did not reach database readiness")
            for headers in ({}, {"Authorization": "Bearer synthetic-invalid-token"}):
                try:
                    urllib.request.urlopen(urllib.request.Request(origin + "/api/workspaces", headers=headers), timeout=5)
                except urllib.error.HTTPError as error:
                    if error.code != 401:
                        raise RuntimeError("Unauthenticated API access must return 401") from None
                else:
                    raise RuntimeError("Unauthenticated API access was accepted")
            if runtime == "podman":
                command("healthcheck", "run", name)
            else:
                command("exec", name, "/usr/local/bin/ai-center-server", "--healthcheck")
            config = json.loads(command("inspect", name))[0]
            if config["Config"]["User"] != "10001:10001" or not config["HostConfig"]["ReadonlyRootfs"]:
                raise RuntimeError("Non-root read-only runtime required")
            if config["Config"].get("Healthcheck", {}).get("Test") != ["CMD", "/usr/local/bin/ai-center-server", "--healthcheck"]:
                raise RuntimeError("Image must retain its scheduled healthcheck; Podman builds require --format docker")
            command("stop", "--time", "30", name)
            state = json.loads(command("inspect", "--format", "{{json .State}}", name))
            if state["ExitCode"] != 0:
                raise RuntimeError("API must terminate gracefully on SIGTERM")
            print(json.dumps({"result": "passed", "image": image, "image_id": config["Image"],
                              "checks": ["database readiness", "Auth refusal", "built-in healthcheck", "non-root read-only filesystem", "SIGTERM exit 0"],
                              "scope": "local guarded DB/Auth; no hosted service or billed model call"}))
        finally:
            if started:
                command("rm", "--force", name)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        raise SystemExit(f"API image smoke failed ({type(error).__name__}); no credential or log content printed.") from None
