"""
Interpreter Logic Tests

Tests for variable lookup fallback, sys_time_sleep, and sys_exec.
Converted from script-style to proper unittest to prevent pytest crashes.
"""
import sys
import os
import unittest
from unittest.mock import MagicMock, patch
import io

# Mock lark before import
lark_mock = MagicMock()
sys.modules["lark"] = lark_mock

# Ensure we can import meta.ark
sys.path.append(os.getcwd())

try:
    from meta.ark import eval_node, Scope, ArkValue, INTRINSICS, sys_time_sleep, sys_exec
except ImportError:
    sys.path.append(os.path.dirname(os.getcwd()))
    from meta.ark import eval_node, Scope, ArkValue, INTRINSICS, sys_time_sleep, sys_exec


class MockToken:
    def __init__(self, value):
        self.value = value
        self.type = "IDENTIFIER"


class MockTree:
    def __init__(self, data, children):
        self.data = data
        self.children = children


class TestInterpreterLogic(unittest.TestCase):
    """Unit tests for interpreter logic functions."""

    def test_variable_lookup_fallback(self):
        """Verify that looking up 'print' falls back to intrinsic."""
        scope = Scope()
        var_node = MockTree("var", [MockToken("print")])
        result = eval_node(var_node, scope)
        self.assertEqual(result.type, "Intrinsic")
        self.assertEqual(result.val, "print")

    @patch("time.sleep")
    def test_sys_time_sleep(self, mock_sleep):
        """Verify sys_time_sleep calls time.sleep exactly once."""
        args = [ArkValue(1, "Integer")]
        sys_time_sleep(args)
        self.assertEqual(mock_sleep.call_count, 1)

    @patch("subprocess.run")
    def test_sys_exec_returns_output(self, mock_run):
        """Verify sys_exec executes a command and returns output."""
        mock_run.return_value = MagicMock(
            stdout="hello\n",
            stderr="",
            returncode=0,
        )
        args = [ArkValue("echo hello", "String")]
        result = sys_exec(args)
        self.assertEqual(result.type, "String")
        self.assertIn("hello", result.val)


if __name__ == "__main__":
    unittest.main()
