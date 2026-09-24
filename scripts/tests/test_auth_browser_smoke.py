"""Local transport regressions; no Auth key, container or external network."""
import importlib.util
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import threading
import unittest
from unittest.mock import patch
import urllib.request

path = Path(__file__).resolve().parents[1] / "auth-browser-smoke.py"
spec = importlib.util.spec_from_file_location("auth_browser_smoke", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class LocalTransportTests(unittest.TestCase):
    def test_does_not_forward_local_auth_requests_to_a_proxy_or_redirect(self):
        seen = []

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                seen.append((self.server.server_port, self.path))
                if self.path == "/redirect":
                    self.send_response(302)
                    self.send_header("Location", f"http://127.0.0.1:{sink.server_port}/forbidden")
                    self.end_headers()
                else:
                    self.send_response(200)
                    self.end_headers()
                    self.wfile.write(b"local fixture")

        with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as sink, ThreadingHTTPServer(("127.0.0.1", 0), Handler) as source:
            threads = [threading.Thread(target=server.serve_forever, daemon=True) for server in (sink, source)]
            for thread in threads:
                thread.start()
            try:
                proxy = f"http://127.0.0.1:{sink.server_port}"
                with patch.dict("os.environ", {"HTTP_PROXY": proxy, "http_proxy": proxy, "ALL_PROXY": proxy, "NO_PROXY": "", "no_proxy": ""}):
                    base = f"http://127.0.0.1:{source.server_port}"
                    with module.LOCAL_HTTP.open(base + "/direct", timeout=2) as response:
                        self.assertEqual(response.read(), b"local fixture")
                    request = urllib.request.Request(base + "/redirect", headers={"Authorization": "Bearer FICTITIOUS-ONLY"})
                    with self.assertRaisesRegex(RuntimeError, "redirect refused"):
                        module.LOCAL_HTTP.open(request, timeout=2)
                self.assertEqual(seen, [(source.server_port, "/direct"), (source.server_port, "/redirect")])
            finally:
                source.shutdown()
                sink.shutdown()
                for thread in threads:
                    thread.join(timeout=2)


if __name__ == "__main__":
    unittest.main()
