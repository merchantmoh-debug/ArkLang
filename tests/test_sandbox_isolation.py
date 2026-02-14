import unittest
import sys
import os
from unittest.mock import MagicMock, patch

# Ensure src is in python path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

# Mock dependencies if missing
try:
    import pydantic
    from src.sandbox.local import LocalSandbox
except ImportError:
    # Mock pydantic and settings
    sys.modules['pydantic'] = MagicMock()
    sys.modules['pydantic_settings'] = MagicMock()

    mock_settings = MagicMock()
    # Allow sys and os for this test
    mock_settings.BANNED_IMPORTS = {
        "subprocess", "shutil", "importlib", "socket",
        "pickle", "urllib", "http", "xml", "base64", "pty", "pdb",
        "platform", "venv", "ensurepip", "site", "imp", "posix", "nt"
    }
    mock_settings.BANNED_FUNCTIONS = {
        "open", "exec", "eval", "compile", "__import__", "input",
        "exit", "quit", "help", "dir", "vars", "globals", "locals",
        "breakpoint", "memoryview"
    }
    mock_settings.BANNED_ATTRIBUTES = {
        "__subclasses__", "__bases__", "__globals__", "__code__",
        "__closure__", "__func__", "__self__", "__module__", "__dict__"
    }

    sys.modules['src.config'] = MagicMock()
    sys.modules['src.config'].settings = mock_settings

    from src.sandbox.local import LocalSandbox

class TestSandboxIsolation(unittest.TestCase):
    def test_sys_path_isolation(self):
        """Test that the sandbox runs in isolated mode (no site-packages)."""
        sandbox = LocalSandbox()

        # Test 1: Check sys.path for site-packages
        # Test 2: Check os.environ for leakage (e.g. PATH or custom vars)

        # We inject a custom env var into the PARENT process
        os.environ["SECRET_TOKEN"] = "SUPER_SECRET"

        code = """
import sys
import os

failures = []

# Check 1: sys.path
site_packages = [p for p in sys.path if 'site-packages' in p or 'dist-packages' in p]
if site_packages:
    failures.append(f"Found site-packages in sys.path: {site_packages}")

# Check 2: Environment Leakage
if "SECRET_TOKEN" in os.environ:
    failures.append("Found SECRET_TOKEN in os.environ - Environment Leakage!")

if failures:
    print("FAILURE: " + "; ".join(failures))
else:
    print("SUCCESS: Isolation verified")
"""

        # Force allow sys and os via patching settings if needed
        # (If mocking, we handled it above. If not, we need to patch)
        # Assuming mocking handles it or we use ALLOW_DANGEROUS...

        os.environ["ALLOW_DANGEROUS_LOCAL_EXECUTION"] = "true"
        try:
            result = sandbox.execute(code)
        finally:
             del os.environ["ALLOW_DANGEROUS_LOCAL_EXECUTION"]
             del os.environ["SECRET_TOKEN"]

        print(f"Stdout: {result.stdout}")
        print(f"Stderr: {result.stderr}")

        self.assertIn("SUCCESS: Isolation verified", result.stdout)
        self.assertNotIn("FAILURE", result.stdout)

if __name__ == "__main__":
    unittest.main()
