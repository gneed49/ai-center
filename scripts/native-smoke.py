#!/usr/bin/env python3
"""Prepare or explicitly run the Linux native smoke against the disposable API."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import select
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[1]
API_URL = "http://127.0.0.1:4617"
DRIVER_URL = "http://127.0.0.1:9515"


class SmokeError(Exception):
    """A sanitized smoke failure, without provider payloads or credentials."""


class ChildProcesses:
    """Own only process groups started by this smoke, including their children."""

    def __init__(self) -> None:
        self.processes: list[subprocess.Popen] = []

    def __enter__(self) -> ChildProcesses:
        return self

    def start(self, command: list[str], **options: Any) -> subprocess.Popen:
        process = subprocess.Popen(command, start_new_session=True, **options)
        self.processes.append(process)
        return process

    def __exit__(self, *_: Any) -> None:
        for process in reversed(self.processes):
            # The group can outlive its leader after a driver crash.
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=5)


def isolated_environment(profile: Path, inherited: dict[str, str]) -> dict[str, str]:
    environment = {
        key: inherited[key]
        for key in ("PATH", "LANG", "LC_ALL", "LD_LIBRARY_PATH")
        if key in inherited
    }
    for name, folder in (
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_RUNTIME_DIR", "runtime"),
    ):
        path = profile / folder
        path.mkdir(mode=0o700)
        environment[name] = str(path)
    environment.update(GDK_BACKEND="x11", NO_AT_BRIDGE="1")
    return environment


def prepare(output_dir: Path) -> None:
    product = json.loads((REPO / "apps/desktop/src-tauri/tauri.conf.json").read_text())
    directives = product["app"]["security"]["csp"].split(";")
    csp = "; ".join(
        (
            f"connect-src 'self' {API_URL}"
            if value.strip().startswith("connect-src ")
            else value.strip()
        )
        for value in directives
        if value.strip()
    )
    output_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    overlay = {
        "identifier": "com.aicenter.client.smoke",
        "build": {
            "beforeBuildCommand": "",
            "frontendDist": str(REPO / "apps/web/dist"),
        },
        "app": {"security": {"csp": csp}},
        "bundle": {"active": False},
    }
    path = output_dir / "tauri.config.json"
    path.write_text(json.dumps(overlay, indent=2) + "\n")
    path.chmod(0o600)


def validate_target(app: Path) -> dict[str, str]:
    if (
        os.environ.get("AI_CENTER_AGENT_MODE") != "deterministic"
        or os.environ.get("AI_CENTER_REAL_E2E_API_URL") != API_URL
    ):
        raise SmokeError("deterministic integration environment required")
    expected = REPO / ".run/integration-stack"
    if os.environ.get("AI_CENTER_INTEGRATION_WORKDIR") != str(expected):
        raise SmokeError("disposable integration workdir required")
    guard = subprocess.run(
        [sys.executable, str(REPO / "scripts/integration-target.py"), "guard"],
        capture_output=True,
    )
    if guard.returncode:
        raise SmokeError("disposable integration guard failed")
    if not app.is_file() or not os.access(app, os.X_OK):
        raise SmokeError("compiled native application is missing")
    marker_path = REPO / ".run/native-smoke/build.json"
    try:
        marker = json.loads(marker_path.read_text())
        matches = marker == build_identity(app)
    except (OSError, ValueError):
        matches = False
    if not matches:
        raise SmokeError("native binary has no matching isolated build record")
    with app.open("rb") as handle:
        if handle.read(4) != b"\x7fELF":
            raise SmokeError("native smoke requires a Linux ELF application")
    tools = {}
    for command in ("Xvfb", "tauri-driver", "WebKitWebDriver"):
        binary = shutil.which(command)
        if binary is None:
            raise SmokeError(f"required native tool missing: {command}")
        tools[command] = binary
    return tools


def build_identity(app: Path) -> dict[str, str]:
    overlay = REPO / ".run/native-smoke/tauri.config.json"
    with app.open("rb") as binary:
        application_hash = hashlib.file_digest(binary, "sha256").hexdigest()
    return {
        "application": str(app.resolve()),
        "api_url": API_URL,
        "application_sha256": application_hash,
        "overlay_sha256": hashlib.sha256(overlay.read_bytes()).hexdigest(),
    }


class WebDriver:
    def __init__(self) -> None:
        self.session: str | None = None
        # Loopback traffic must never inherit an operator's outbound proxy.
        self.http = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def request(self, method: str, path: str, payload: Any = None) -> Any:
        body = json.dumps(payload).encode() if payload is not None else None
        request = urllib.request.Request(
            DRIVER_URL + path,
            data=body,
            method=method,
            headers={"Content-Type": "application/json"},
        )
        try:
            with self.http.open(request, timeout=30) as response:
                value = json.load(response)["value"]
        except (OSError, ValueError, KeyError) as error:
            raise SmokeError("native WebDriver request failed") from error
        if isinstance(value, dict) and value.get("error"):
            raise SmokeError("native WebDriver command was rejected")
        return value

    def command(self, method: str, path: str, payload: Any = None) -> Any:
        if self.session is None:
            raise SmokeError("native WebDriver session is missing")
        return self.request(method, f"/session/{self.session}{path}", payload)

    def evaluate(self, script: str, *args: Any) -> Any:
        return self.command(
            "POST", "/execute/sync", {"script": script, "args": list(args)}
        )

    def wait_text(self, text: str) -> None:
        wait_until(
            lambda: self.evaluate(
                "return document.body.innerText.includes(arguments[0])", text
            ),
            "expected business screen did not appear",
        )

    def element(self, using: str, value: str) -> str:
        return self.command("POST", "/element", {"using": using, "value": value})[
            "element-6066-11e4-a52e-4f735466cecf"
        ]

    def click(self, xpath: str) -> None:
        identifier = self.element("xpath", xpath)
        self.command("POST", f"/element/{identifier}/click", {})

    def button(self, label: str) -> None:
        self.click(f"//button[normalize-space()={json.dumps(label)}]")

    def fill(self, selector: str, text: str) -> None:
        identifier = self.element("css selector", selector)
        self.command("POST", f"/element/{identifier}/value", {"text": text})


def wait_until(predicate: Any, failure: str, seconds: int = 25) -> None:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        try:
            if predicate():
                return
        except SmokeError:
            pass
        time.sleep(0.2)
    raise SmokeError(failure)


def business_flow(driver: WebDriver, checks: list[str]) -> None:
    wait_until(
        lambda: driver.evaluate("return document.body.innerText.length > 0"),
        "native shell did not render",
    )
    if driver.evaluate("return location.origin") != "tauri://localhost":
        raise SmokeError("smoke did not load the native Tauri origin")
    driver.evaluate(
        "window.__nativeSmokeErrors = []; window.addEventListener('error', () => window.__nativeSmokeErrors.push('error')); window.addEventListener('unhandledrejection', () => window.__nativeSmokeErrors.push('rejection')); return true"
    )
    checks.append("native_shell")
    project_name = "Native Context Proof " + uuid.uuid4().hex[:8]
    driver.click('//a[normalize-space()="Nouveau projet"]')
    driver.wait_text("Nom du projet")
    driver.fill("#project-name", project_name)
    driver.fill(
        "#project-objective", "Conserver des décisions validées, durables et traçables."
    )
    driver.button("Créer les scopes")
    driver.wait_text(project_name)
    checks.append("project_created")
    driver.click(
        '//article[.//h3[normalize-space()="Produit"] or .//h2[normalize-space()="Produit"]]//button[normalize-space()="Nouvelle session"]'
    )
    driver.wait_text("Message à l’agent product")
    driver.fill(
        "#session-message",
        "Les décisions validées restent consultables sans expiration.",
    )
    driver.button("Envoyer")
    wait_until(
        lambda: driver.evaluate(
            "return document.querySelectorAll('button[aria-label^=\"Sélectionner la proposition\"]').length"
        )
        == 3,
        "three proposals did not appear",
    )
    for _ in range(3):
        driver.click('//button[starts-with(@aria-label,"Sélectionner la proposition")]')
    driver.button("Confirmer (3)")
    driver.wait_text("3 connaissance(s) confirmée(s)")
    checks.append("knowledge_confirmed")
    driver.click('//a[@aria-label="Retour au projet"]')
    driver.wait_text("graphe v1")
    driver.button("Évaluer maintenant")
    driver.wait_text("Produit prêt pour le handoff")
    driver.click('//a[normalize-space()="Préparer le handoff"]')
    driver.wait_text("Compiler et ouvrir la session Tech")
    driver.button("Compiler et ouvrir la session Tech")
    driver.wait_text("Handoff terminé et courant")
    driver.wait_text("Contexte inclus")
    checks.append("handoff_current")
    if driver.evaluate("return window.__nativeSmokeErrors.length"):
        raise SmokeError("native application reported a page error")
    before = driver.evaluate("return location.href")
    driver.command("POST", "/refresh", {})
    driver.wait_text("Handoff terminé et courant")
    driver.wait_text("Contexte inclus")
    if driver.evaluate("return location.href") != before:
        raise SmokeError("handoff location changed after reload")
    checks.append("handoff_persisted_after_reload")


def run(app: Path, output_dir: Path) -> None:
    tools = validate_target(app)
    for port in (9515, 9516):
        with socket.socket() as handle:
            try:
                handle.bind(("127.0.0.1", port))
            except OSError as error:
                raise SmokeError(
                    "native driver port is occupied; existing processes were left alone"
                ) from error
    output_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    report: dict[str, Any] = {
        "schema_version": "1.0",
        "passed": False,
        "checks": [],
        "scope": "Linux native client against isolated deterministic API",
    }
    driver = WebDriver()
    try:
        with tempfile.TemporaryDirectory(
            prefix="profile-", dir=output_dir
        ) as temporary, ChildProcesses() as children, (output_dir / "driver.log").open(
            "w"
        ) as log:
            environment = isolated_environment(Path(temporary), dict(os.environ))
            read_fd, write_fd = os.pipe()
            try:
                children.start(
                    [
                        tools["Xvfb"],
                        "-displayfd",
                        str(write_fd),
                        "-screen",
                        "0",
                        "1440x900x24",
                        "-nolisten",
                        "tcp",
                    ],
                    env=environment,
                    pass_fds=(write_fd,),
                    stdout=log,
                    stderr=log,
                )
                os.close(write_fd)
                write_fd = -1
                if not select.select([read_fd], [], [], 15)[0]:
                    raise SmokeError("isolated Xvfb display did not start")
                display = os.read(read_fd, 64).decode().strip()
                if not display.isdigit():
                    raise SmokeError("Xvfb returned an invalid display")
                environment["DISPLAY"] = ":" + display
            finally:
                os.close(read_fd)
                if write_fd != -1:
                    os.close(write_fd)
            children.start(
                [
                    tools["tauri-driver"],
                    "--port",
                    "9515",
                    "--native-port",
                    "9516",
                    "--native-host",
                    "127.0.0.1",
                    "--native-driver",
                    tools["WebKitWebDriver"],
                ],
                env=environment,
                stdout=log,
                stderr=log,
            )
            wait_until(
                lambda: driver.request("GET", "/status").get("ready") is True,
                "native WebDriver did not start",
            )
            try:
                created = driver.request(
                    "POST",
                    "/session",
                    {
                        "capabilities": {
                            "alwaysMatch": {
                                "browserName": "wry",
                                "tauri:options": {"application": str(app.resolve())},
                            }
                        }
                    },
                )
                driver.session = created["sessionId"]
                business_flow(driver, report["checks"])
            finally:
                if driver.session:
                    try:
                        screenshot = driver.command("GET", "/screenshot")
                        screenshot_bytes = base64.b64decode(screenshot, validate=True)
                        if not screenshot_bytes.startswith(b"\x89PNG\r\n\x1a\n"):
                            raise SmokeError("native screenshot is not a PNG")
                        (output_dir / "screen.png").write_bytes(screenshot_bytes)
                    except (SmokeError, ValueError):
                        report["screenshot_missing"] = True
                    try:
                        driver.request("DELETE", f"/session/{driver.session}")
                    except SmokeError:
                        pass
        if report.get("screenshot_missing"):
            raise SmokeError("native screenshot was not captured")
        report["passed"] = True
    finally:
        report["generated_at"] = datetime.now(timezone.utc).isoformat()
        path = output_dir / "result.json"
        path.write_text(json.dumps(report, indent=2) + "\n")
        path.chmod(0o600)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    preparation = commands.add_parser(
        "prepare", help="Write the test overlay without launching anything."
    )
    preparation.add_argument(
        "--output-dir", type=Path, default=REPO / ".run/native-smoke"
    )
    execution = commands.add_parser(
        "run",
        help="Explicitly launch the native business smoke on the guarded disposable stack.",
    )
    execution.add_argument("--app", type=Path, required=True)
    execution.add_argument(
        "--output-dir", type=Path, default=REPO / ".run/native-smoke/artifacts"
    )
    recording = commands.add_parser(
        "record-build", help="Record the native binary produced by the isolated build."
    )
    recording.add_argument("--app", type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)

    def interrupted(_signal: int, _frame: Any) -> None:
        raise SmokeError("native smoke interrupted")

    if args.command == "run":
        signal.signal(signal.SIGTERM, interrupted)
        signal.signal(signal.SIGINT, interrupted)
    try:
        if args.command == "prepare":
            prepare(args.output_dir)
        elif args.command == "record-build":
            path = REPO / ".run/native-smoke/build.json"
            path.write_text(json.dumps(build_identity(args.app), indent=2) + "\n")
            path.chmod(0o600)
        else:
            run(args.app, args.output_dir)
    except SmokeError as error:
        print(f"Native smoke blocked: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
