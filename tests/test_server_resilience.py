import unittest
import time
import subprocess
import urllib.request
import json
import os
import signal
import sys

class TestServerResilience(unittest.TestCase):
    def setUp(self):
        # Start server in background
        print(f"Starting server with {sys.executable}")
        self.server_process = subprocess.Popen(
            [sys.executable, "server.py"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
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
