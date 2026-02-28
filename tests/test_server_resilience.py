import unittest
import time
import subprocess
import urllib.request
import json
import os
import signal
import sys
import socket

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SERVER_SCRIPT = os.path.join(REPO_ROOT, "scripts", "server.py")

def _port_available(port):
    """Check if a TCP port is available for binding."""
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("", port))
        return True
    except OSError:
        return False

@unittest.skipUnless(os.path.exists(SERVER_SCRIPT), "scripts/server.py not found")
@unittest.skipUnless(_port_available(8000), "Port 8000 is already in use")
class TestServerResilience(unittest.TestCase):
    def setUp(self):
        # Start server in background
        print(f"Starting server with {sys.executable}")

        # Set up environment: server needs PYTHONPATH to find src.sandbox.*
        env = os.environ.copy()
        env["PYTHONPATH"] = REPO_ROOT
        env.setdefault("ARK_CAPABILITIES", "exec,net,fs_read")

        self.server_process = subprocess.Popen(
            [sys.executable, SERVER_SCRIPT],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=REPO_ROOT,
            env=env,
        )

        # Poll for readiness
        start_time = time.time()
        while time.time() - start_time < 10:
            try:
                with urllib.request.urlopen("http://localhost:8000/api/stats", timeout=1) as response:
                    if response.status == 200:
                        return # Ready!
            except Exception:
                time.sleep(0.5)

        # If we get here, it failed
        self.server_process.terminate()
        try:
            out, err = self.server_process.communicate(timeout=2)
            print(f"Server STDOUT: {out.decode()}")
            print(f"Server STDERR: {err.decode()}")
        except:
            pass
        self.fail("Server did not start in 10 seconds")

    def tearDown(self):
        if self.server_process.poll() is None:
            self.server_process.terminate()
            try:
                self.server_process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.server_process.kill()

    def test_api_stats(self):
        try:
            with urllib.request.urlopen("http://localhost:8000/api/stats") as response:
                self.assertEqual(response.status, 200)
                data = json.loads(response.read().decode())
                self.assertIn("cpu", data)
                self.assertIn("memory", data)
                self.assertIn("neural", data)
                print(f"\n[Verified] Stats: {data}")
        except urllib.error.URLError as e:
            self.fail(f"Could not connect to server: {e}")

    def test_home_page(self):
        try:
            with urllib.request.urlopen("http://localhost:8000/") as response:
                self.assertEqual(response.status, 200)
                text = response.read().decode()
                self.assertIn("Ark Web Playground", text)
        except urllib.error.URLError as e:
            self.fail(f"Could not connect to server: {e}")

if __name__ == "__main__":
    unittest.main()
