import unittest
import sys
import os
from meta.compile import compile_to_python
from meta.ark import Scope, ArkValue, eval_binop, UNIT_VALUE, INTRINSICS

class TestCompilerStmt(unittest.TestCase):
    def test_func_def_and_call(self):
        code = """
        func add(a, b) {
            return a + b
        }
        res := add(10, 20)
        """
        code_obj = compile_to_python(code)
        scope = Scope()
        from meta.compile import resolve_var, call_func, is_truthy, ArkFunction
        exec_globals = {
            "scope": scope, "ArkValue": ArkValue, "eval_binop": eval_binop, "UNIT_VALUE": UNIT_VALUE,
            "INTRINSICS": INTRINSICS, "resolve_var": resolve_var, "call_func": call_func,
            "is_truthy": is_truthy, "ArkFunction": ArkFunction
        }
        exec(code_obj, exec_globals)
        print(f"DEBUG: Scope vars func: {scope.vars}")
        res = scope.get("res")
        if res:
            print(f"DEBUG: res type: {res.type}, val: {res.val}")

        self.assertIsNotNone(res)
        self.assertEqual(res.val, 30)

    def test_if_stmt(self):
        code = """
        x := 10
        if x > 5 {
            y := 1
        } else {
            y := 0
        }
        """
        code_obj = compile_to_python(code)
        scope = Scope()
        from meta.compile import resolve_var, call_func, is_truthy, ArkFunction
        exec_globals = {
            "scope": scope, "ArkValue": ArkValue, "eval_binop": eval_binop, "UNIT_VALUE": UNIT_VALUE,
            "INTRINSICS": INTRINSICS, "resolve_var": resolve_var, "call_func": call_func,
            "is_truthy": is_truthy, "ArkFunction": ArkFunction
        }
        exec(code_obj, exec_globals)
        print(f"DEBUG: Scope vars if: {scope.vars}")
        y = scope.get("y")
        self.assertIsNotNone(y)
        self.assertEqual(y.val, 1)

if __name__ == "__main__":
    unittest.main()
