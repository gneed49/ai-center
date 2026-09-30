#!/usr/bin/env python3
"""Exercise the built web image locally; this does not qualify Auth or a live API."""
import argparse
import json
import re
import subprocess
import time
import urllib.error
import urllib.request
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", default="localhost/ai-center-web:company-v1")
    args = parser.parse_args()
    name = "ai-center-web-smoke-" + uuid.uuid4().hex[:12]
    started = False
    try:
        subprocess.run([
            "podman", "run", "--detach", "--name", name, "--pull=never",
            "--cap-drop=ALL", "--security-opt=no-new-privileges:true",
            "--publish", "127.0.0.1::8080",
            "--env", "AI_CENTER_API_UPSTREAM=http://127.0.0.1:9", args.image,
        ], check=True, capture_output=True, text=True, timeout=30)
        started = True
        binding = subprocess.check_output(["podman", "port", name, "8080/tcp"], text=True, timeout=10).strip()
        if not re.fullmatch(r"127\.0\.0\.1:\d+", binding):
            raise RuntimeError("Unexpected non-loopback test binding")
        origin = "http://" + binding
        for _ in range(50):
            try:
                with urllib.request.urlopen(origin + "/healthz", timeout=1) as response:
                    if response.read() == b"ok\n":
                        break
            except (OSError, urllib.error.URLError):
                time.sleep(0.1)
        else:
            raise RuntimeError("Web container did not become ready")

        expected = {
            "X-Content-Type-Options": "nosniff",
            "X-Frame-Options": "DENY",
            "Referrer-Policy": "strict-origin-when-cross-origin",
            "Permissions-Policy": "camera=(), geolocation=(), microphone=()",
        }

        def fetch(path, status=200):
            try:
                response = urllib.request.urlopen(origin + path, timeout=5)
            except urllib.error.HTTPError as error:
                response = error
            with response:
                if response.status != status:
                    raise RuntimeError(f"Unexpected status for {path}: {response.status}")
                for header, value in expected.items():
                    if response.headers.get(header) != value:
                        raise RuntimeError(f"Missing security header {header} on {path}")
                return response.read(), response.headers

        html, headers = fetch("/")
        if headers.get("Cache-Control") != "no-cache":
            raise RuntimeError("HTML entry point must revalidate")
        asset = re.search(rb'src="(/assets/[^\"]+\.js)"', html)
        if asset is None:
            raise RuntimeError("Built application asset not found")
        _, asset_headers = fetch(asset.group(1).decode())
        if asset_headers.get("Cache-Control") != "public, max-age=31536000, immutable":
            raise RuntimeError("Hashed assets must have immutable caching")
        route_html, _ = fetch("/projects/fictitious-project/artifacts")
        if route_html != html:
            raise RuntimeError("Application deep links must return the SPA entry point")
        fetch("/healthz")
        fetch("/assets/fictitious-missing.js", 404)
        # This test deliberately has no API. Failure must stay a failure.
        fetch("/api/health", 502)
        subprocess.run(["podman", "healthcheck", "run", name], check=True, capture_output=True, timeout=10)
        image_id = subprocess.check_output(["podman", "image", "inspect", args.image, "--format", "{{.Id}}"], text=True, timeout=10).strip()
        print(json.dumps({"result": "passed", "image_id": image_id, "checked": ["HTML", "hashed asset", "SPA deep link", "healthcheck", "404", "unavailable upstream"], "scope": "local web image only; no live API/Auth proof"}))
    finally:
        if started:
            subprocess.run(["podman", "rm", "--force", name], check=True, capture_output=True, timeout=20)


if __name__ == "__main__":
    main()
