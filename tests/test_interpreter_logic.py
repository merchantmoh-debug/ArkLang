import sys
import os
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

print("--- Verifying Variable Lookup Fallback ---")
scope = Scope()
var_node = MockTree("var", [MockToken("print")])

try:
    result = eval_node(var_node, scope)
    if result.type == "Intrinsic" and result.val == "print":
        print("SUCCESS: Fallback working correctly.")
    else:
        print(f"FAILURE: Did not return Intrinsic. Got {result}")
        sys.exit(1)
except Exception as e:
    print(f"FAILURE: Exception: {e}")
    sys.exit(1)


print("\n--- Verifying sys_time_sleep Fix ---")
with patch("time.sleep") as mock_sleep:
    args = [ArkValue(1, "Integer")]
    sys_time_sleep(args)
    if mock_sleep.call_count == 1:
        print("SUCCESS: time.sleep called exactly once.")
    else:
        print(f"FAILURE: time.sleep called {mock_sleep.call_count} times.")
        sys.exit(1)

print("\n--- Verifying sys_exec Security Warning ---")
with patch("os.popen") as mock_popen, patch("sys.stderr", new_callable=io.StringIO) as mock_stderr:
    mock_popen.return_value.read.return_value = "output"
    args = [ArkValue("echo hello", "String")]

    sys_exec(args)

    stderr_output = mock_stderr.getvalue()
    if "WARNING: Executing system command" in stderr_output:
        print("SUCCESS: Security warning printed.")
    else:
        print(f"FAILURE: Security warning NOT printed. Output: {stderr_output}")
        sys.exit(1)
