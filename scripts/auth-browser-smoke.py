#!/usr/bin/env python3
"""Two real local Auth identities and browser UI against the guarded DB."""
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import urllib.request
import uuid

REPO = Path(__file__).resolve().parents[1]


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, _request, _fp, _code, _msg, _headers, _url):
        raise RuntimeError("Local Auth redirect refused")


LOCAL_HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())


def main():
    subprocess.run(["python3", str(REPO / "scripts/integration-target.py"), "guard"], check=True, cwd=REPO)
    workdir = os.environ["AI_CENTER_INTEGRATION_WORKDIR"]
    status = subprocess.run([str(REPO / "node_modules/.bin/supabase"), "--workdir", workdir, "status", "-o", "json"], check=True, capture_output=True, cwd=workdir)
    keys = json.loads(status.stdout)
    origin = keys["API_URL"]
    if origin != "http://127.0.0.1:55321":
        raise RuntimeError("Guarded local Auth required")
    service_key, anon_key = keys["SERVICE_ROLE_KEY"], keys["ANON_KEY"]
    users, processes = [], []

    def auth(path, payload=None, admin=True, method="POST"):
        key = service_key if admin else anon_key
        request = urllib.request.Request(origin + "/auth/v1" + path,
            data=json.dumps(payload).encode() if payload is not None else None,
            method=method, headers={"apikey": key, "Authorization": "Bearer " + key, "Content-Type": "application/json"})
        with LOCAL_HTTP.open(request, timeout=10) as response:
            return json.load(response)

    with tempfile.TemporaryDirectory(prefix="ai-center-auth-browser-") as directory:
        private = Path(directory)
        log_handles = []
        try:
            sessions = []
            for _ in range(2):
                email = f"auth-browser-{uuid.uuid4().hex}@example.invalid"
                user = auth("/admin/users", {"email": email, "email_confirm": True})
                users.append(str(uuid.UUID(user["id"])))
                link = auth("/admin/generate_link", {"type": "magiclink", "email": email})
                otp = link.get("email_otp") or link.get("properties", {}).get("email_otp")
                if not otp:
                    raise RuntimeError("Local OTP unavailable")
                sessions.append(auth("/verify", {"type": "magiclink", "email": email, "token": otp}, admin=False))
            fixture = private / "sessions.json"
            fixture.write_text(json.dumps({"sessions": sessions}))
            fixture.chmod(0o600)
            subprocess.run(["cargo", "build", "-p", "ai-center-server", "--bin", "ai-center-server"], check=True, cwd=REPO)
            api_env = dict(os.environ, DATABASE_URL=os.environ["AI_CENTER_RUNTIME_DATABASE_URL"],
                AI_CENTER_BIND="127.0.0.1:4618", AI_CENTER_AUTH_MODE="supabase",
                AI_CENTER_AGENT_MODE="deterministic", AI_CENTER_COMPANY_CREATORS=users[0],
                AI_CENTER_CORS_ORIGINS="http://127.0.0.1:5183", SUPABASE_URL=origin)
            api_env.pop("AI_CENTER_WORKSPACE_ID", None)
            api_env.pop("AI_CENTER_ACTOR_ID", None)
            web_env = dict(os.environ, VITE_API_URL="http://127.0.0.1:4618", VITE_SUPABASE_URL=origin,
                VITE_SUPABASE_ANON_KEY=anon_key, VITE_WORKSPACE_ID="", TAURI_DEV_HOST="")
            commands = [
                ([str(Path(os.environ.get("CARGO_TARGET_DIR", REPO / "target")) / "debug/ai-center-server")], api_env, workdir),
                (["node", str(REPO / "node_modules/vite/bin/vite.js"), "--config", "apps/web/vite.integration.config.ts", "apps/web", "--host", "127.0.0.1", "--port", "5183", "--strictPort"], web_env, REPO),
            ]
            for index, (command, env, cwd) in enumerate(commands):
                log = (private / f"process-{index}.log").open("wb")
                log_handles.append(log)
                processes.append(subprocess.Popen(command, env=env, cwd=cwd, stdout=log, stderr=log, start_new_session=True))
            for endpoint in ("http://127.0.0.1:4618/api/health", "http://127.0.0.1:5183/"):
                for _ in range(100):
                    if any(process.poll() is not None for process in processes):
                        raise RuntimeError("Local browser services exited")
                    try:
                        with LOCAL_HTTP.open(endpoint, timeout=1):
                            break
                    except OSError:
                        time.sleep(0.1)
                else:
                    raise RuntimeError("Local browser services unavailable")
            playwright_log = (private / "playwright.log").open("wb")
            log_handles.append(playwright_log)
            playwright = subprocess.Popen(["npx", "playwright", "test", "--config=apps/web/playwright.auth.config.ts"],
                cwd=REPO, env=dict(os.environ, AI_CENTER_AUTH_BROWSER_FIXTURE=str(fixture), PLAYWRIGHT_NO_COPY_PROMPT="1"),
                stdout=playwright_log, stderr=playwright_log, start_new_session=True)
            processes.append(playwright)
            if playwright.wait(timeout=120) != 0:
                # Print only locations, never assertion values, headers or URLs.
                report_path = private / "report.json"
                if report_path.exists():
                    report = json.loads(report_path.read_text())
                    for suite in report.get("suites", []):
                        for spec in suite.get("specs", []):
                            for test in spec.get("tests", []):
                                for result in test.get("results", []):
                                    for error in result.get("errors", []):
                                        location = error.get("location", {})
                                        line = location.get("line")
                                        if isinstance(line, int):
                                            print(f"Local Auth browser assertion failed at team.spec.ts:{line}.")
                raise RuntimeError("Local Auth browser failed")
            print("Auth navigateur local : société, invitation, deux comptes, rôles et retrait vérifiés. Aucun SMTP externe ni appel IA.")
        finally:
            for process in reversed(processes):
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                if process.poll() is None:
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        pass
                # A wrapper can exit before a browser child. The session group
                # remains ours even when its original leader has already exited.
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait(timeout=5)
            for log in log_handles:
                log.close()
            cleanup_failed = False
            for user in users:
                try:
                    auth("/admin/users/" + user, method="DELETE")
                except Exception:
                    cleanup_failed = True
            if cleanup_failed:
                raise RuntimeError("Ephemeral Auth cleanup failed; guarded stack teardown required")


if __name__ == "__main__":
    def terminate(_signum, _frame):
        raise SystemExit(143)

    signal.signal(signal.SIGTERM, terminate)
    try:
        main()
    except Exception as error:
        raise SystemExit(f"Auth browser smoke failed ({type(error).__name__}); private credentials and service logs withheld.") from None
