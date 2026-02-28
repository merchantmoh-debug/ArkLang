import unittest
import subprocess
import time
import os
import sys
import json

try:
    import urllib.request
    import urllib.error
except ImportError:
    pass

SERVER_SCRIPT = "scripts/server.py"
PORT = 8000
BASE_URL = f"http://localhost:{PORT}"

HAS_SERVER_SCRIPT = os.path.exists(SERVER_SCRIPT)
IS_WINDOWS = sys.platform == "win32"


@unittest.skipUnless(HAS_SERVER_SCRIPT, "Server script not found — skip neural pulse tests")
@unittest.skipIf(IS_WINDOWS, "Neural pulse server tests use os.setsid (Unix-only)")
class TestNeuralPulse(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Start server in background
        print(f"[TEST] Starting {SERVER_SCRIPT} on port {PORT}...")

        env = os.environ.copy()
        env["PYTHONPATH"] = os.getcwd() # Ensure root is in path

        kwargs = {}
        if not IS_WINDOWS:
            kwargs["preexec_fn"] = os.setsid

        cls.server_process = subprocess.Popen(
            [sys.executable, SERVER_SCRIPT],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
            **kwargs
        )

        # Wait for server to come up
        for _ in range(20):
            try:
                with urllib.request.urlopen(BASE_URL) as response:
                    if response.status == 200:
                        print("[TEST] Server is up!")
                        return
            except urllib.error.URLError:
                time.sleep(0.5)
            except ConnectionResetError:
                time.sleep(0.5)

        # Kill if timeout
        if cls.server_process:
            try:
                if not IS_WINDOWS:
                    import signal
                    os.killpg(os.getpgid(cls.server_process.pid), signal.SIGTERM)
                else:
                    cls.server_process.terminate()
                cls.server_process.wait()
            except Exception:
                pass
            stdout, stderr = cls.server_process.communicate()
            print(f"Server STDOUT: {stdout.decode()}")
            print(f"Server STDERR: {stderr.decode()}")

        raise RuntimeError("Server failed to start")

    @classmethod
    def tearDownClass(cls):
        print("[TEST] Stopping server...")
        if cls.server_process:
            try:
                if not IS_WINDOWS:
                    import signal
                    os.killpg(os.getpgid(cls.server_process.pid), signal.SIGTERM)
                else:
                    cls.server_process.terminate()
                cls.server_process.wait(timeout=2)
                stdout, stderr = cls.server_process.communicate()
                print(f"--- Server STDOUT ---\n{stdout.decode()}\n---------------------")
                print(f"--- Server STDERR ---\n{stderr.decode()}\n---------------------")
            except:
                pass

    def get_json(self, path):
        req = urllib.request.Request(f"{BASE_URL}{path}")
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode())

    def post_json(self, path, data):
        req = urllib.request.Request(f"{BASE_URL}{path}", method="POST")
        req.add_header('Content-Type', 'application/json')
        jsondata = json.dumps(data).encode('utf-8')
        req.add_header('Content-Length', len(jsondata))

        try:
            with urllib.request.urlopen(req, jsondata) as response:
                return json.loads(response.read().decode())
        except urllib.error.HTTPError as e:
            # Handle 500 etc gracefully for tests
            print(f"[TEST] POST Error: {e.code} {e.read().decode()}")
            raise

    def test_01_stats_endpoint(self):
        """Verify /api/stats returns expected structure"""
        data = self.get_json("/api/stats")

        self.assertIn("cpu", data)
        self.assertIn("memory", data)
        self.assertIn("neural", data)
        self.assertIn("sys_info", data)

        sys_info = data["sys_info"]
        self.assertEqual(sys_info["platform"], "Ark Sovereign Runtime")
        self.assertEqual(sys_info["version"], "v112.0 (Prime)")

    def test_02_neural_pulse_activation(self):
        """Verify Neural Activity increases with load"""
        # Baseline
        data = self.get_json("/api/stats")
        baseline_neural = data["neural"]
        print(f"[TEST] Baseline Neural: {baseline_neural}")

        # Generate Load (Fire 20 requests)
        payload = {"code": "print('pulse')"}

        print("[TEST] Firing 20 requests to stimulate Neural Pulse...")
        for _ in range(20):
            try:
                self.post_json("/api/run", payload)
            except:
                pass # Ignore errors, just want to trigger counter

        # Check Pulse
        data = self.get_json("/api/stats")
        active_neural = data["neural"]
        print(f"[TEST] Active Neural: {active_neural}")

        # Expect increase (10 base + 2*count)
        self.assertGreater(active_neural, baseline_neural, "Neural activity did not increase with load")
        self.assertGreaterEqual(active_neural, 40, "Neural activity too low for load generated")

    def test_03_path_traversal_protection(self):
        """Verify server hardening"""
        target = f"{BASE_URL}/../scripts/server.py"
        try:
            with urllib.request.urlopen(target) as response:
                self.fail("Should have returned 403 or 404")
        except urllib.error.HTTPError as e:
            print(f"[TEST] Path Traversal Response: {e.code}")
            # Python http.server returns 404/403 for errors
            self.assertIn(e.code, [403, 404])

if __name__ == "__main__":
    unittest.main()
