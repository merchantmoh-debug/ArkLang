import asyncio
import os
import sys
import time
import tempfile
import shutil
import ast
from typing import List, Set, Tuple, Optional
from pathlib import Path

from src.config import settings
from .base import (
    BaseSandbox,
    ExecutionResult,
    truncate_output,
    SandboxError,
    SandboxTimeoutError,
    SandboxSecurityError,
)

# Language configurations
CMD_MAP = {
    "python": [sys.executable],  # Will be followed by script path
    "ark": [sys.executable, "meta/ark.py", "run"],
    "javascript": ["node"],
    "rust": ["rustc"],  # Compile first
}


class SecurityVisitor(ast.NodeVisitor):
    """AST visitor to enforce security restrictions on user code."""

    def __init__(self, capabilities: Set[str]):
        self.errors: List[str] = []
        self.capabilities = capabilities

        # Start with default banned lists
        self.banned_imports = set(settings.BANNED_IMPORTS)
        self.banned_functions = set(settings.BANNED_FUNCTIONS)

        # Capability adjustments
        if "net" in capabilities:
            self.banned_imports.discard("socket")
            self.banned_imports.discard("urllib")
            self.banned_imports.discard("http")
            self.banned_imports.discard("requests")

        if "fs_read" in capabilities or "fs_write" in capabilities:
            self.banned_functions.discard("open")

        # Hard blocks (Override capabilities)
        self.hard_banned_imports = {"os", "subprocess", "shutil", "pty"}
        self.hard_banned_functions = {"eval", "exec", "compile", "__import__"}
        self.hard_banned_attributes = {"system", "Popen", "call", "check_call", "check_output", "run"}

    def visit_Import(self, node: ast.Import) -> None:
        for alias in node.names:
            name = alias.name.split('.')[0]
            if name in self.hard_banned_imports:
                 self.errors.append(f"Import of '{name}' is forbidden.")
            elif name in self.banned_imports:
                self.errors.append(f"Import of '{alias.name}' is forbidden without proper capabilities.")
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        if node.module:
            name = node.module.split('.')[0]
            if name in self.hard_banned_imports:
                self.errors.append(f"Import from '{name}' is forbidden.")
            elif name in self.banned_imports:
                self.errors.append(f"Import from '{node.module}' is forbidden without proper capabilities.")
        self.generic_visit(node)

    def visit_Call(self, node: ast.Call) -> None:
        if isinstance(node.func, ast.Name):
            if node.func.id in self.hard_banned_functions:
                self.errors.append(f"Call to '{node.func.id}()' is forbidden.")
            elif node.func.id in self.banned_functions:
                self.errors.append(f"Call to '{node.func.id}()' is forbidden without proper capabilities.")
        self.generic_visit(node)

    def visit_Attribute(self, node: ast.Attribute) -> None:
        # Check for things like os.system, subprocess.Popen
        if node.attr in self.hard_banned_attributes:
             self.errors.append(f"Access to attribute '{node.attr}' is restricted (potential security risk).")

        if node.attr in settings.BANNED_ATTRIBUTES:
            self.errors.append(f"Access to attribute '{node.attr}' is forbidden.")
        self.generic_visit(node)

    def visit_Name(self, node: ast.Name) -> None:
        """Catch bare references to banned names like __builtins__, open, eval."""
        all_banned_names = self.banned_functions | self.hard_banned_functions | {
            "__builtins__"
        }
        if node.id in all_banned_names:
            self.errors.append(f"Reference to banned name '{node.id}' is forbidden.")
        self.generic_visit(node)


class LocalSandbox(BaseSandbox):
    """Local subprocess-based sandbox."""

    def __init__(self, capabilities: Set[str] = None):
        super().__init__(capabilities)
        # No persistent temp_dir to avoid race conditions

    def cleanup(self):
        """Clean up the temporary directory."""
        pass

    def _check_python_security(self, code: str):
        """Analyze Python code for security violations."""
        # Bypass flag: skip all security checks
        if os.environ.get("ALLOW_DANGEROUS_LOCAL_EXECUTION", "false").lower() == "true":
            return
        try:
            tree = ast.parse(code)
            visitor = SecurityVisitor(self.capabilities)
            visitor.visit(tree)
            if visitor.errors:
                raise SandboxSecurityError(
                    "Security Violation: " + "; ".join(visitor.errors)
                )
        except SyntaxError as e:
            raise SandboxSecurityError(f"Syntax Error: {e}")
        except SandboxSecurityError:
            raise
        except Exception as e:
             raise SandboxSecurityError(f"Security analysis failed: {e}")

    def execute(
        self,
        code: str,
        language: str = "python",
        timeout: int = 30,
    ) -> ExecutionResult:
        """Synchronous execute using subprocess.run (matches test expectations)."""
        import subprocess

        language = language.lower()
        if language not in CMD_MAP:
            return ExecutionResult(
                stdout="",
                stderr=f"Unsupported language: {language}",
                exit_code=1,
                duration_ms=0.0,
                truncated=False
            )

        start_time = time.time()

        # 1. Security Check (Python only)
        if language == "python":
            try:
                self._check_python_security(code)
            except SandboxSecurityError as e:
                return ExecutionResult(
                    stdout="",
                    stderr=str(e),
                    exit_code=1,
                    duration_ms=(time.time() - start_time) * 1000,
                    truncated=False
                )

        # Use temporary directory context manager for isolation per execution
        with tempfile.TemporaryDirectory(prefix="ark_sandbox_") as temp_dir:
            # 2. File Setup
            filename = "main.py"
            if language == "javascript": filename = "main.js"
            elif language == "rust": filename = "main.rs"
            elif language == "ark": filename = "main.ark"

            filepath = os.path.join(temp_dir, filename)
            try:
                with open(filepath, "w", encoding="utf-8") as f:
                    f.write(code)
            except Exception as e:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Failed to write code to file: {e}",
                    exit_code=1,
                    duration_ms=(time.time() - start_time) * 1000,
                )

            # 3. Command Construction
            cmd = []
            cwd = temp_dir

            if language == "python":
                # -S = no site module (removes ALL site-packages from sys.path)
                # -I = isolated mode (ignores PYTHONPATH and PYTHONSTARTUP)
                cmd = [sys.executable, "-S", "-I", filepath]
            elif language == "ark":
                # Transpile then execute
                json_path = filepath + ".json"
                transpile_cmd = [
                    sys.executable,
                    os.path.abspath("meta/ark_to_json.py"),
                    filepath, "-o", json_path
                ]
                try:
                    result = subprocess.run(
                        transpile_cmd, capture_output=True, text=True,
                        timeout=timeout, cwd=os.getcwd()
                    )
                    if result.returncode != 0:
                        return ExecutionResult(
                            stdout=result.stdout,
                            stderr=f"Transpilation failed:\n{result.stderr}",
                            exit_code=result.returncode,
                            duration_ms=(time.time() - start_time) * 1000,
                        )
                except Exception as e:
                    return ExecutionResult(
                        stdout="",
                        stderr=f"Transpilation error: {e}",
                        exit_code=1,
                        duration_ms=(time.time() - start_time) * 1000,
                    )

                loader_path = os.path.abspath("target/release/ark_loader")
                if not os.path.exists(loader_path):
                    loader_path = os.path.abspath("core/target/release/ark_loader")
                if not os.path.exists(loader_path):
                    return ExecutionResult(
                        stdout="",
                        stderr="Ark Runtime (ark_loader) not found. Please compile core.",
                        exit_code=1, duration_ms=0.0
                    )
                cmd = [loader_path, json_path]
                cwd = os.getcwd()
            elif language == "javascript":
                cmd = ["node", filepath]
            elif language == "rust":
                exe_path = os.path.join(temp_dir, "main")
                compile_cmd = ["rustc", filepath, "-o", exe_path]
                try:
                    result = subprocess.run(
                        compile_cmd, capture_output=True, text=True,
                        timeout=timeout, cwd=temp_dir
                    )
                    if result.returncode != 0:
                        return ExecutionResult(
                            stdout=result.stdout,
                            stderr=f"Compilation failed:\n{result.stderr}",
                            exit_code=result.returncode,
                            duration_ms=(time.time() - start_time) * 1000,
                        )
                except FileNotFoundError:
                    return ExecutionResult(
                        stdout="", stderr="rustc not found",
                        exit_code=1, duration_ms=(time.time() - start_time) * 1000,
                    )
                cmd = [exe_path]

            # 4. Execution
            try:
                # Get max output from env
                max_output_kb = int(os.environ.get("SANDBOX_MAX_OUTPUT_KB", "100"))
                max_output_bytes = max_output_kb * 1024

                # Environment isolation: clean env for subprocess
                clean_env = {}
                # Only pass through PATH and essential system vars
                # Note: PYTHONPATH and PYTHONHOME are excluded to prevent
                # site-packages leakage (also enforced by -I flag)
                for k in ("PATH", "SYSTEMROOT", "TEMP", "TMP"):
                    if k in os.environ:
                        clean_env[k] = os.environ[k]

                result = subprocess.run(
                    cmd,
                    capture_output=True,
                    text=True,
                    timeout=timeout,
                    cwd=cwd,
                    env=clean_env,
                )

                duration_ms = (time.time() - start_time) * 1000

                stdout_str = result.stdout or ""
                stderr_str = result.stderr or ""

                # Truncation
                is_truncated = False
                if len(stdout_str.encode("utf-8", errors="ignore")) > max_output_bytes:
                    # Truncate to max_output_bytes - 32 and append marker
                    trunc_point = max(0, max_output_bytes - 32)
                    stdout_str = stdout_str[:trunc_point] + "\n... (output truncated)"
                    is_truncated = True

                return ExecutionResult(
                    stdout=stdout_str,
                    stderr=stderr_str,
                    exit_code=result.returncode,
                    duration_ms=duration_ms,
                    truncated=is_truncated,
                    timed_out=False,
                )

            except subprocess.TimeoutExpired:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Execution timed out after {timeout}s",
                    exit_code=-1,
                    duration_ms=(time.time() - start_time) * 1000,
                    truncated=False,
                    timed_out=True,
                )

            except Exception as e:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Unexpected execution error: {e}",
                    exit_code=1,
                    duration_ms=(time.time() - start_time) * 1000,
                )

if __name__ == "__main__":
    print("Verifying src/sandbox/local.py...")
    sandbox = LocalSandbox(capabilities={"net"})

    # 1. Test Python Success
    res = sandbox.execute('print("Hello World")', "python")
    assert res.stdout.strip() == "Hello World"
    assert res.exit_code == 0
    print("Python execution: OK")

    # 2. Test Security Block
    res = sandbox.execute('import os; os.system("ls")', "python")
    assert res.exit_code != 0
    assert "Security violations" in res.stderr
    print("Security block: OK")

    # 3. Test Ark (if available)
    if os.path.exists("meta/ark.py"):
        res = sandbox.execute('print("Hello Ark")', "ark")
        if res.exit_code == 0:
             print(f"Ark execution: OK ({res.stdout.strip()})")
        else:
             print(f"Ark execution failed (expected if env incomplete): {res.stderr}")
    else:
        print("Ark execution: Skipped (meta/ark.py not found)")

    print("Local verification complete.")

