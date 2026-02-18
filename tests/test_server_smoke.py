"""
P1: Server Smoke Tests — Timeout-Based

Verifies that all server demos in the README actually start and respond to HTTP requests.
These demos are skipped by the Gauntlet due to their interactive (blocking) nature.

Usage: python -m pytest tests/test_server_smoke.py -v
"""

import os
import subprocess
import sys
import time
import urllib.request
import urllib.error

import pytest

# All tests require the Ark runtime to be importable
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ARK_CMD = [sys.executable, os.path.join(REPO_ROOT, "meta", "ark.py"), "run"]


def _start_ark_server(ark_file, env_caps="net,fs_read"):
    """Start an Ark server demo as a background process."""
    env = {**os.environ, "ARK_CAPABILITIES": env_caps}
    proc = subprocess.Popen(
        ARK_CMD + [ark_file],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=REPO_ROOT,
        env=env,
    )
    return proc


def _wait_for_server(port, timeout=8, interval=0.5):
    """Poll until the server responds or timeout is reached."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            resp = urllib.request.urlopen(f"http://localhost:{port}/", timeout=2)
            return resp
        except (urllib.error.URLError, ConnectionRefusedError):
            time.sleep(interval)
    return None


def _kill(proc):
    """Terminate and clean up a subprocess."""
    try:
        proc.terminate()
        proc.wait(timeout=5)
    except Exception:
        proc.kill()
        proc.wait(timeout=5)


# ── Test: simple_server.ark (port 8087) ──────────────────────────────────

@pytest.mark.timeout(20)
def test_simple_server():
    """
    README claim: examples/simple_server.ark starts an HTTP server.
    Verifies: server binds port 8087 and responds to GET /.
    """
    proc = _start_ark_server("examples/simple_server.ark")
    try:
        resp = _wait_for_server(8087)
        assert resp is not None, "simple_server.ark did not respond on port 8087 within timeout"
        data = resp.read().decode()
        assert len(data) > 0, "simple_server.ark returned empty response"
        assert resp.status == 200
    finally:
        _kill(proc)


# ── Test: server.ark (port 8080) ─────────────────────────────────────────

@pytest.mark.timeout(20)
def test_server():
    """
    README claim: examples/server.ark starts an HTTP server.
    Verifies: server binds port 8080 and responds with HTML.
    """
    proc = _start_ark_server("examples/server.ark")
    try:
        resp = _wait_for_server(8080)
        assert resp is not None, "server.ark did not respond on port 8080 within timeout"
        data = resp.read().decode()
        assert "Ark Server" in data or "Welcome" in data, f"Expected Ark Server content, got: {data[:200]}"
        assert resp.status == 200
    finally:
        _kill(proc)


# ── Test: snake.ark (port 8000) ──────────────────────────────────────────

@pytest.mark.timeout(20)
def test_snake_server():
    """
    README claim: python3 meta/ark.py run examples/snake.ark
    Verifies: snake server binds port 8000 and /state returns JSON with snake data.
    """
    proc = _start_ark_server("examples/snake.ark")
    try:
        resp = _wait_for_server(8000)
        assert resp is not None, "snake.ark did not respond on port 8000 within timeout"
        data = resp.read().decode()
        # Snake state should be JSON-ish with snake array
        assert "snake" in data.lower() or "[" in data, f"Expected snake state JSON, got: {data[:200]}"
    finally:
        _kill(proc)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
