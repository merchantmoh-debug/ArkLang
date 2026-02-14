import os
import sys
import time
import tempfile
import subprocess
import ast
from typing import List, Set, Tuple

from src.config import settings
from .base import CodeSandbox, ExecutionResult, truncate_output


class SecurityVisitor(ast.NodeVisitor):
    """AST visitor to enforce security restrictions on user code."""

    def __init__(self):
        self.errors: List[str] = []
        # Blacklist of dangerous modules
        self.banned_imports: Set[str] = settings.BANNED_IMPORTS
        # Blacklist of dangerous builtins/functions
        self.banned_functions: Set[str] = settings.BANNED_FUNCTIONS
        # Blacklist of dangerous attributes often used for exploits
        self.banned_attributes: Set[str] = settings.BANNED_ATTRIBUTES

    def visit_Import(self, node: ast.Import) -> None:
        for alias in node.names:
            name = alias.name.split('.')[0]
            if name in self.banned_imports:
                self.errors.append(f"Import of '{alias.name}' is forbidden.")
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        if node.module:
            name = node.module.split('.')[0]
            if name in self.banned_imports:
                self.errors.append(f"Import from '{node.module}' is forbidden.")
        self.generic_visit(node)

    def visit_Call(self, node: ast.Call) -> None:
        if isinstance(node.func, ast.Name):
            if node.func.id in self.banned_functions:
                self.errors.append(f"Call to '{node.func.id}()' is forbidden.")
        self.generic_visit(node)

    def visit_Attribute(self, node: ast.Attribute) -> None:
        if node.attr in self.banned_attributes:
            self.errors.append(f"Access to attribute '{node.attr}' is forbidden.")
        self.generic_visit(node)

    def visit_Name(self, node: ast.Name) -> None:
        if node.id in self.banned_functions:
            self.errors.append(f"Reference to banned name '{node.id}' is forbidden.")
        elif node.id in self.banned_imports:
            self.errors.append(f"Reference to banned module '{node.id}' is forbidden.")
        self.generic_visit(node)


class LocalSandbox(CodeSandbox):
    """Local subprocess-based sandbox.

    Runs code using the current Python interpreter inside an isolated temp directory.
    Applies timeout and output truncation.

    SECURITY NOTICE:
    This sandbox executes code on the local machine. It uses AST analysis to block
    common dangerous operations and clears environment variables, but it is NOT
    a perfect security boundary (e.g. it does not use containers or VMs).
    """

    def execute(self, code: str, language: str = "python", timeout: int = 30) -> ExecutionResult:
        if language.lower() != "python":
            return ExecutionResult(
                stdout="",
                stderr=f"Unsupported language: {language}",
                exit_code=1,
                duration=0.0,
                meta={"runtime": "local", "truncated": False, "timed_out": False},
            )

        # 1. Security Analysis (AST)
        # Check if we should bypass security (useful for dev/debugging or trusted environments)
        allow_dangerous = os.getenv("ALLOW_DANGEROUS_LOCAL_EXECUTION", "false").lower() == "true"

        if not allow_dangerous:
            try:
                tree = ast.parse(code)
                visitor = SecurityVisitor()
                visitor.visit(tree)
                if visitor.errors:
                    return ExecutionResult(
                        stdout="",
                        stderr="Security Violation:\n" + "\n".join(visitor.errors),
                        exit_code=1,
                        duration=0.0,
                        meta={"runtime": "local", "security_violation": True},
                    )
            except SyntaxError as e:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Syntax Error: {e}",
                    exit_code=1,
                    duration=0.0,
                    meta={"runtime": "local", "syntax_error": True},
                )
            except ValueError as e:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Value Error: {e}",
                    exit_code=1,
                    duration=0.0,
                    meta={"runtime": "local", "value_error": True},
                )
            except RecursionError as e:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Recursion Error: {e}",
                    exit_code=1,
                    duration=0.0,
                    meta={"runtime": "local", "recursion_error": True},
                )
            except Exception as e:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Security analysis failed: {e}",
                    exit_code=1,
                    duration=0.0,
                    meta={"runtime": "local", "analysis_error": True},
                )

        max_output_kb = int(os.getenv("SANDBOX_MAX_OUTPUT_KB", "10"))
        max_bytes = max_output_kb * 1024

        start = time.time()
        timed_out = False
        stdout = ""
        stderr = ""
        exit_code = 0

        with tempfile.TemporaryDirectory(prefix="ag_sandbox_") as tmpdir:
            script_path = os.path.join(tmpdir, "main.py")
            with open(script_path, "w", encoding="utf-8") as f:
                f.write(code)

            try:
                # 2. Environment Isolation
                # Pass an empty environment to prevent leaking host secrets/vars.
                # On Windows, SystemRoot is often required for Python to start.
                env = {}
                if sys.platform == 'win32' and 'SystemRoot' in os.environ:
                    env['SystemRoot'] = os.environ['SystemRoot']

                # Create a launcher script to enforce resource limits
                launcher_code = """
import sys
import runpy

# Resource limits (Unix only)
try:
    import resource
    # CPU time limit in seconds
    timeout = int(sys.argv[2])
    # Add small grace period for startup
    cpu_limit = timeout + 2
    resource.setrlimit(resource.RLIMIT_CPU, (cpu_limit, cpu_limit))

    # Memory limit (address space) - 512MB
    mem_limit_mb = 512
    mem_limit_bytes = mem_limit_mb * 1024 * 1024
    resource.setrlimit(resource.RLIMIT_AS, (mem_limit_bytes, mem_limit_bytes))

    # File Descriptor limit - Restrict to prevent FD exhaustion
    # But allow enough for imports and stdio
    resource.setrlimit(resource.RLIMIT_NOFILE, (256, 256))
except ImportError:
    pass
except Exception:
    # Ignore errors setting limits (e.g. if already restricted)
    pass

# Execute user script
script_path = sys.argv[1]
try:
    runpy.run_path(script_path, run_name="__main__")
except SystemExit as e:
    sys.exit(e.code)
except Exception as e:
    # Print exception to stderr so it's captured
    import traceback
    traceback.print_exc()
    sys.exit(1)
"""
                launcher_path = os.path.join(tmpdir, "launcher.py")
                with open(launcher_path, "w", encoding="utf-8") as f:
                    f.write(launcher_code)

                # Use -I for isolated mode (no user site-packages, no env vars)
                # Use -S to disable site module import, preventing access to system site-packages
                cmd = [
                    sys.executable,
                    "-I",
                    "-S",
                    launcher_path,
                    script_path,
                    str(timeout)
                ]

                proc = subprocess.run(
                    cmd,
                    cwd=tmpdir,
                    capture_output=True,
                    text=True,
                    timeout=timeout,
                    env=env  # RESTRICTED ENVIRONMENT
                )
                stdout = proc.stdout or ""
                stderr = proc.stderr or ""
                exit_code = proc.returncode
            except subprocess.TimeoutExpired:
                timed_out = True
                exit_code = -1
                stderr = f"Execution timed out after {timeout}s"
            except Exception as exc:
                exit_code = 1
                stderr = f"Unexpected execution error: {exc}"

        duration = time.time() - start

        stdout, trunc_out = truncate_output(stdout, max_bytes)
        stderr, trunc_err = truncate_output(stderr, max_bytes)

        return ExecutionResult(
            stdout=stdout,
            stderr=stderr,
            exit_code=exit_code,
            duration=duration,
            meta={
                "runtime": "local",
                "truncated": bool(trunc_out or trunc_err),
                "timed_out": timed_out,
                "resource_limits": {
                    "timeout_sec": timeout,
                    "max_output_kb": max_output_kb,
                },
            },
        )
