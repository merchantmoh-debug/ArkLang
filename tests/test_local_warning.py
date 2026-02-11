import os
import unittest
import io
import contextlib
from unittest.mock import patch
from src.sandbox.factory import get_sandbox
from src.sandbox.local import LocalSandbox

class TestLocalWarning(unittest.TestCase):
    def setUp(self):
        self.env_patcher = patch.dict(os.environ, {"SANDBOX_TYPE": "local"})
        self.env_patcher.start()

    def tearDown(self):
        self.env_patcher.stop()

    def test_local_warning_printed(self):
        # Capture stderr
        f = io.StringIO()
        with contextlib.redirect_stderr(f):
            sandbox = get_sandbox()

        # Assert warning is printed
        output = f.getvalue()
        self.assertIn("WARNING: LocalSandbox is insecure", output)
        self.assertIsInstance(sandbox, LocalSandbox)

if __name__ == "__main__":
    unittest.main()
