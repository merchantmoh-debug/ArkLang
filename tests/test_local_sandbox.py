import os
import sys
import unittest
from unittest.mock import patch, MagicMock
import subprocess

# Add src to path
sys.path.append(os.path.abspath("src"))

from sandbox.local import LocalSandbox
from sandbox.base import ExecutionResult

class TestLocalSandbox(unittest.TestCase):

    def setUp(self):
        self.sandbox = LocalSandbox()
        # Ensure consistent environment for tests
        self.env_patcher = patch.dict(os.environ, {"SANDBOX_MAX_OUTPUT_KB": "10"})
        self.env_patcher.start()

    def tearDown(self):
        self.env_patcher.stop()

    @patch("subprocess.run")
    def test_execute_success(self, mock_run):
        # Mock successful execution
        mock_process = MagicMock()
        mock_process.returncode = 0
        mock_process.stdout = "hello\n"
        mock_process.stderr = ""
        mock_run.return_value = mock_process

        code = "print('hello')"
        result = self.sandbox.execute(code)

        self.assertEqual(result.stdout, "hello\n")
        self.assertEqual(result.stderr, "")
        self.assertEqual(result.exit_code, 0)
        self.assertFalse(result.meta["timed_out"])
        self.assertFalse(result.meta["truncated"])

        # Verify subprocess.run was called correctly
        mock_run.assert_called_once()
        args, kwargs = mock_run.call_args
        self.assertEqual(kwargs["timeout"], 30)
        self.assertEqual(kwargs["capture_output"], True)
        self.assertEqual(kwargs["text"], True)
        self.assertEqual(kwargs["cwd"], kwargs["cwd"]) # Just ensure it exists

    @patch("subprocess.run")
    def test_execute_error_nonzero_exit(self, mock_run):
        # Mock execution error
        mock_process = MagicMock()
        mock_process.returncode = 1
        mock_process.stdout = ""
        mock_process.stderr = "NameError: name 'x' is not defined\n"
        mock_run.return_value = mock_process

        code = "print(x)"
        result = self.sandbox.execute(code)

        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "NameError: name 'x' is not defined\n")
        self.assertEqual(result.exit_code, 1)

    @patch("subprocess.run")
    def test_execute_timeout(self, mock_run):
        # Mock timeout
        mock_run.side_effect = subprocess.TimeoutExpired(cmd=["python"], timeout=5)

        code = "while True: pass"
        result = self.sandbox.execute(code, timeout=5)

        self.assertEqual(result.exit_code, -1)
        self.assertTrue(result.meta["timed_out"])
        self.assertIn("Execution timed out", result.stderr)

    def test_unsupported_language(self):
        result = self.sandbox.execute("console.log('hello')", language="javascript")

        self.assertEqual(result.exit_code, 1)
        self.assertIn("Unsupported language: javascript", result.stderr)
        self.assertEqual(result.stdout, "")

    @patch("subprocess.run")
    def test_output_truncation(self, mock_run):
        # Set max output to 1KB
        with patch.dict(os.environ, {"SANDBOX_MAX_OUTPUT_KB": "1"}):
            # Generate 2KB of output
            long_output = "a" * 2048
            mock_process = MagicMock()
            mock_process.returncode = 0
            mock_process.stdout = long_output
            mock_process.stderr = ""
            mock_run.return_value = mock_process

            result = self.sandbox.execute("print('a' * 2048)")

            self.assertTrue(result.meta["truncated"])
            self.assertIn("... (output truncated)", result.stdout)
            # 1KB = 1024 bytes. Truncation happens at max_bytes - 32.
            self.assertLess(len(result.stdout), 2048)

    @patch("subprocess.run")
    def test_unexpected_exception(self, mock_run):
        # Mock unexpected exception during subprocess.run
        mock_run.side_effect = Exception("Disk full")

        result = self.sandbox.execute("print('hello')")

        self.assertEqual(result.exit_code, 1)
        self.assertIn("Unexpected execution error: Disk full", result.stderr)

if __name__ == "__main__":
    unittest.main()
