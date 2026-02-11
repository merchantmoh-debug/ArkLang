import os
import sys
import unittest
from unittest.mock import patch

# Add src to path
sys.path.append(os.path.abspath("src"))

from sandbox.factory import get_sandbox
from sandbox.local import LocalSandbox
from sandbox.docker_exec import DockerSandbox

class TestSandboxConfigs(unittest.TestCase):

    def setUp(self):
        self.env_patcher = patch.dict(os.environ, {}, clear=True)
        self.env_patcher.start()

    def tearDown(self):
        self.env_patcher.stop()

    def test_default_is_docker(self):
        # No SANDBOX_TYPE set
        sandbox = get_sandbox()
        self.assertIsInstance(sandbox, DockerSandbox)

    def test_explicit_local(self):
        os.environ["SANDBOX_TYPE"] = "local"
        sandbox = get_sandbox()
        self.assertIsInstance(sandbox, LocalSandbox)

    def test_explicit_docker(self):
        os.environ["SANDBOX_TYPE"] = "docker"
        sandbox = get_sandbox()
        self.assertIsInstance(sandbox, DockerSandbox)

    def test_invalid_type(self):
        os.environ["SANDBOX_TYPE"] = "invalid"
        with self.assertRaises(ValueError) as cm:
            get_sandbox()
        self.assertIn("Unknown sandbox type: invalid", str(cm.exception))

    def test_e2b_raises_runtime_error_if_missing(self):
        os.environ["SANDBOX_TYPE"] = "e2b"
        # Since e2b_exec.py is missing, it should raise RuntimeError
        with self.assertRaises(RuntimeError) as cm:
            get_sandbox()
        self.assertIn("E2B sandbox requested", str(cm.exception))
        self.assertIn("package is not installed", str(cm.exception))

if __name__ == "__main__":
    unittest.main()
