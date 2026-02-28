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

# ─── Path Setup ───────────────────────────────────────────────────────────────
# When invoked as `python meta/ark.py`, the repo root isn't in sys.path.
# This ensures `from meta.xyz import ...` works everywhere.
_repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _repo_root not in sys.path:
    sys.path.insert(0, _repo_root)

# ─── Version Gate ─────────────────────────────────────────────────────────────
# Guard against old Python. The core works on 3.8+, but 3.10+ is recommended
# for dataclass(slots=True) performance.
if sys.version_info < (3, 8):
    print(f"Error: Ark requires Python 3.8 or newer (you have {sys.version}).")
    print("Install Python 3.10+ from https://python.org or use pyenv.")
    sys.exit(1)

# ─── Dependency Gate ──────────────────────────────────────────────────────────
# Catch missing deps BEFORE cascading import errors confuse users.
_missing_deps = []
for _dep in ["lark", "pydantic"]:
    try:
        __import__(_dep)
    except ImportError:
        _missing_deps.append(_dep)

if _missing_deps:
    print(f"Error: Missing required dependencies: {', '.join(_missing_deps)}")
    print()
    print("Fix: Run this command from the ark-compiler directory:")
    print()
    print("    pip install -r requirements.txt")
    print()
    sys.exit(1)

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
        SafeRedirectHandler, check_capability, has_capability, CAPABILITIES,
        check_tool_allowed, is_read_only, get_capability_summary, LOCKDOWN_PATHS
    )
    from meta.ark_intrinsics import (
        INTRINSICS, LINEAR_SPECS, INTRINSICS_WITH_SCOPE,
        _make_late_intrinsics, EVENT_QUEUE,
        sys_exec, sys_time_sleep, sanitize_prompt
    )
    from meta.ark_interpreter import (
        eval_node, call_user_func, instantiate_class, eval_block,
        is_truthy, eval_binop, ARK_PARSER, NODE_HANDLERS
    )
except ModuleNotFoundError as _e:
    # Only fall back to relative imports if the error is about the 'meta' prefix.
    # Re-raise if a real dependency is missing.
    if "meta" not in str(_e):
        raise
    from ark_types import (
        RopeString, ArkValue, UNIT_VALUE, ReturnException,
        ArkFunction, ArkClass, ArkInstance, Scope
    )
    from ark_security import (
        SandboxViolation, LinearityViolation,
        check_path_security, check_exec_security, validate_url_security,
        SafeRedirectHandler, check_capability, has_capability, CAPABILITIES,
        check_tool_allowed, is_read_only, get_capability_summary, LOCKDOWN_PATHS
    )
    from ark_intrinsics import (
        INTRINSICS, LINEAR_SPECS, INTRINSICS_WITH_SCOPE,
        _make_late_intrinsics, EVENT_QUEUE,
        sys_exec, sys_time_sleep, sanitize_prompt
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


# ─── AI Mode Detection ───────────────────────────────────────────────────────
ARK_AI_MODE = None


def detect_ai_mode() -> str:
    """Detect which AI backend is available: OLLAMA > GEMINI > MOCK."""
    global ARK_AI_MODE
    import urllib.request
    import urllib.error

    # 1. Try Ollama
    try:
        with urllib.request.urlopen("http://localhost:11434/api/tags", timeout=2) as resp:
            if resp.getcode() == 200:
                ARK_AI_MODE = "OLLAMA"
                return ARK_AI_MODE
    except (urllib.error.URLError, OSError):
        pass

    # 2. Try Gemini (check for API key)
    if os.environ.get("GOOGLE_API_KEY"):
        ARK_AI_MODE = "GEMINI"
        return ARK_AI_MODE

    # 3. Fallback to mock
    ARK_AI_MODE = "MOCK"
    return ARK_AI_MODE


def ask_ollama(args):
    """Send prompt to Ollama and return ArkValue response."""
    import urllib.request
    import json as _json
    prompt = args[0].val if args else ""
    try:
        data = _json.dumps({"model": "llama3", "prompt": prompt, "stream": False}).encode()
        req = urllib.request.Request("http://localhost:11434/api/generate", data=data,
                                     headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            result = _json.loads(resp.read().decode())
            return ArkValue(result.get("response", ""), "String")
    except Exception as e:
        return ArkValue(f"[Ollama Error: {e}]", "String")


def ask_gemini(args):
    """Send prompt to Gemini API and return ArkValue response."""
    import urllib.request
    import json as _json
    prompt = args[0].val if args else ""
    api_key = os.environ.get("GOOGLE_API_KEY", "")
    try:
        url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key={api_key}"
        data = _json.dumps({"contents": [{"parts": [{"text": prompt}]}]}).encode()
        req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            result = _json.loads(resp.read().decode())
            text = result.get("candidates", [{}])[0].get("content", {}).get("parts", [{}])[0].get("text", "")
            return ArkValue(text, "String")
    except Exception as e:
        return ArkValue(f"[Gemini Error: {e}]", "String")


def ask_mock(args):
    """Return a deterministic mock response for testing."""
    prompt = args[0].val if args else ""
    return ArkValue(f"[MOCK AI] Echo: {prompt}", "String")


def ask_ai(args):
    """Dispatch to the appropriate AI backend based on detected mode."""
    global ARK_AI_MODE
    mode = ARK_AI_MODE
    if mode is None:
        mode = detect_ai_mode()
        ARK_AI_MODE = mode

    if mode == "OLLAMA":
        return ask_ollama(args)
    elif mode == "GEMINI":
        return ask_gemini(args)
    else:
        return ask_mock(args)


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
    if len(sys.argv) < 2:
        print("Usage: ark <command> [args]")
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "run":
        if len(sys.argv) < 3:
            print("Usage: ark run <file>")
            sys.exit(1)
        run_file(sys.argv[2])
    elif cmd == "repl":
        try:
            from meta.repl import run_repl
        except ModuleNotFoundError:
            from repl import run_repl
        run_repl()
    elif cmd == "version":
        print("ARK OMEGA-POINT v112.0")
    elif cmd == "compile":
        # Delegate to meta/compile.py
        sys.argv.pop(1)  # Remove 'compile'
        sys.argv[0] = 'meta/compile.py'

        if "--target" not in sys.argv:
            sys.argv.extend(["--target", "bytecode"])

        try:
            from meta import compile as ark_compile
        except ModuleNotFoundError:
            import compile as ark_compile
        ark_compile.main()
    else:
        print(f"Unknown command: {cmd}")
        print("Available commands: run, repl, compile, version")
        sys.exit(1)
