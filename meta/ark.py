"""
Ark Compiler — Entry Point

Phase 72: Structural Hardening — Refactored from 2,166-line monolith.
All logic lives in:
  - ark_types.py      → RopeString, ArkValue, Scope, etc.
  - ark_security.py   → Sandbox, capability tokens, path/URL security
  - ark_intrinsics.py → All intrinsic functions + INTRINSICS registry
  - ark_interpreter.py → eval_node, handle_*, AST evaluation engine
"""
import sys
import os

# --- Re-export everything for backward compatibility ---
# External consumers (gauntlet.py, compile.py, tests) can still do:
#   from ark import ArkValue, Scope, INTRINSICS, eval_node, etc.

try:
    from meta.ark_types import (
        RopeString, ArkValue, UNIT_VALUE, ReturnException,
        ArkFunction, ArkClass, ArkInstance, Scope
    )
    from meta.ark_security import (
        SandboxViolation, LinearityViolation,
        check_path_security, check_exec_security, validate_url_security,
        SafeRedirectHandler, check_capability, has_capability, CAPABILITIES
    )
    from meta.ark_intrinsics import (
        INTRINSICS, LINEAR_SPECS, INTRINSICS_WITH_SCOPE,
        _make_late_intrinsics, EVENT_QUEUE
    )
    from meta.ark_interpreter import (
        eval_node, call_user_func, instantiate_class, eval_block,
        is_truthy, eval_binop, ARK_PARSER, NODE_HANDLERS
    )
except ModuleNotFoundError:
    from ark_types import (
        RopeString, ArkValue, UNIT_VALUE, ReturnException,
        ArkFunction, ArkClass, ArkInstance, Scope
    )
    from ark_security import (
        SandboxViolation, LinearityViolation,
        check_path_security, check_exec_security, validate_url_security,
        SafeRedirectHandler, check_capability, has_capability, CAPABILITIES
    )
    from ark_intrinsics import (
        INTRINSICS, LINEAR_SPECS, INTRINSICS_WITH_SCOPE,
        _make_late_intrinsics, EVENT_QUEUE
    )
    from ark_interpreter import (
        eval_node, call_user_func, instantiate_class, eval_block,
        is_truthy, eval_binop, ARK_PARSER, NODE_HANDLERS
    )


# ─── Wire Late Intrinsics ────────────────────────────────────────────────────
# These intrinsics depend on call_user_func from the interpreter.
# They must be injected after both modules are loaded.
_late = _make_late_intrinsics(call_user_func)
INTRINSICS.update(_late)


# ─── Colors ───────────────────────────────────────────────────────────────────
class Colors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'


# ─── Runner ───────────────────────────────────────────────────────────────────

def run_file(path):
    print(f"{Colors.OKCYAN}[ARK OMEGA-POINT v112.0] Running {path}{Colors.ENDC}", file=sys.stderr)
    with open(path, "r") as f:
        code = f.read()
    
    tree = ARK_PARSER.parse(code)
    scope = Scope()
    scope.set("sys", ArkValue("sys", "Namespace"))
    scope.set("math", ArkValue("math", "Namespace"))
    scope.set("true", ArkValue(1, "Integer"))
    scope.set("false", ArkValue(0, "Integer"))
    
    # Inject sys_args
    args_vals = []
    if len(sys.argv) >= 3:
        for a in sys.argv[2:]:
            args_vals.append(ArkValue(a, "String"))
    scope.set("sys_args", ArkValue(args_vals, "List"))

    try:
        eval_node(tree, scope)
    except ReturnException as e:
        print(f"{Colors.FAIL}Error: Return statement outside function{Colors.ENDC}", file=sys.stderr)
    except Exception as e:
        if isinstance(e, SandboxViolation):
            print(f"{Colors.FAIL}SandboxViolation: {e}{Colors.ENDC}", file=sys.stderr)
        else:
            print(f"{Colors.FAIL}Runtime Error: {e}{Colors.ENDC}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        pass
    else:
        run_file(sys.argv[2])
