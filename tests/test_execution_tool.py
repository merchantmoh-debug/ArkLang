import unittest
from unittest.mock import patch, Mock
import os
from src.tools.execution_tool import run_python_code
from src.sandbox.base import ExecutionResult

class TestExecutionTool(unittest.TestCase):
    def setUp(self):
        # Patch get_sandbox where it is imported in execution_tool
        self.patcher = patch('src.tools.execution_tool.get_sandbox')
        self.mock_get_sandbox = self.patcher.start()
        self.mock_sandbox = Mock()
        self.mock_get_sandbox.return_value = self.mock_sandbox

    def tearDown(self):
        self.patcher.stop()

    def test_run_python_code_success(self):
        self.mock_sandbox.execute.return_value = ExecutionResult(
            stdout="hello\n",
            stderr="",
            exit_code=0,
            duration=0.1,
            meta={}
        )
        result = run_python_code("print('hello')")
        self.assertEqual(result, "hello")
        self.mock_sandbox.execute.assert_called_with(code="print('hello')", language="python", timeout=30)

    def test_run_python_code_failure(self):
        self.mock_sandbox.execute.return_value = ExecutionResult(
            stdout="",
            stderr="error occurred\n",
            exit_code=1,
            duration=0.1,
            meta={}
        )
        result = run_python_code("bad code")
        self.assertEqual(result, "Error (exit_code=1): error occurred")

    def test_run_python_code_no_output(self):
        self.mock_sandbox.execute.return_value = ExecutionResult(
            stdout="",
            stderr="",
            exit_code=0,
            duration=0.1,
            meta={}
        )
        result = run_python_code("pass")
        self.assertEqual(result, "(no output)")

    def test_run_python_code_timeout_default(self):
        # Default env is 30, no arg
        self.mock_sandbox.execute.return_value = ExecutionResult(stdout="ok", stderr="", exit_code=0, duration=0, meta={})
        # Explicitly remove env var if set
        with patch.dict(os.environ, {}, clear=True):
             # Wait, ensure SANDBOX_TIMEOUT_SEC is not set
             if "SANDBOX_TIMEOUT_SEC" in os.environ:
                 del os.environ["SANDBOX_TIMEOUT_SEC"]
             run_python_code("code")
             self.mock_sandbox.execute.assert_called_with(code="code", language="python", timeout=30)

    def test_run_python_code_timeout_arg(self):
        self.mock_sandbox.execute.return_value = ExecutionResult(stdout="ok", stderr="", exit_code=0, duration=0, meta={})
        run_python_code("code", timeout=10)
        self.mock_sandbox.execute.assert_called_with(code="code", language="python", timeout=10)

    def test_run_python_code_timeout_env(self):
        self.mock_sandbox.execute.return_value = ExecutionResult(stdout="ok", stderr="", exit_code=0, duration=0, meta={})
        with patch.dict(os.environ, {"SANDBOX_TIMEOUT_SEC": "60"}):
            run_python_code("code")
            self.mock_sandbox.execute.assert_called_with(code="code", language="python", timeout=60)

    def test_run_python_code_timeout_invalid_env(self):
        self.mock_sandbox.execute.return_value = ExecutionResult(stdout="ok", stderr="", exit_code=0, duration=0, meta={})
        with patch.dict(os.environ, {"SANDBOX_TIMEOUT_SEC": "invalid"}):
            run_python_code("code")
            # Should fall back to 30 due to exception
            self.mock_sandbox.execute.assert_called_with(code="code", language="python", timeout=30)
