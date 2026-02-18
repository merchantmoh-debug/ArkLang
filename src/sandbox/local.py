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
                 self.errors.append(f"Import of '{name}' is strictly forbidden.")
            elif name in self.banned_imports:
                self.errors.append(f"Import of '{alias.name}' is forbidden without proper capabilities.")
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        if node.module:
            name = node.module.split('.')[0]
            if name in self.hard_banned_imports:
                self.errors.append(f"Import from '{name}' is strictly forbidden.")
            elif name in self.banned_imports:
                self.errors.append(f"Import from '{node.module}' is forbidden without proper capabilities.")
        self.generic_visit(node)

    def visit_Call(self, node: ast.Call) -> None:
        if isinstance(node.func, ast.Name):
            if node.func.id in self.hard_banned_functions:
                self.errors.append(f"Call to '{node.func.id}()' is strictly forbidden.")
            elif node.func.id in self.banned_functions:
                self.errors.append(f"Call to '{node.func.id}()' is forbidden without proper capabilities.")
        self.generic_visit(node)

    def visit_Attribute(self, node: ast.Attribute) -> None:
        # Check for things like os.system, subprocess.Popen
        # We can't easily resolve the type of the object, but we can check the attribute name
        if node.attr in self.hard_banned_attributes:
             # Heuristic check: if the attribute name is suspicious
             self.errors.append(f"Access to attribute '{node.attr}' is restricted (potential security risk).")

        if node.attr in settings.BANNED_ATTRIBUTES:
            self.errors.append(f"Access to attribute '{node.attr}' is forbidden.")
        self.generic_visit(node)


class LocalSandbox(BaseSandbox):
    """Local subprocess-based sandbox."""

    def __init__(self, capabilities: Set[str] = None):
        super().__init__(capabilities)
        # No persistent temp_dir to avoid race conditions

    async def cleanup(self):
        """Clean up the temporary directory."""
        pass

    def _check_python_security(self, code: str):
        """Analyze Python code for security violations."""
        try:
            tree = ast.parse(code)
            visitor = SecurityVisitor(self.capabilities)
            visitor.visit(tree)
            if visitor.errors:
                raise SandboxSecurityError(
                    "Security violations detected:\n" + "\n".join(visitor.errors)
                )
        except SyntaxError as e:
            # Let the interpreter handle syntax errors, or fail early
            pass
        except SandboxSecurityError:
            raise
        except Exception as e:
             raise SandboxSecurityError(f"Security analysis failed: {e}")

    async def execute(
        self,
        code: str,
        language: str = "python",
        timeout: int = 30,
    ) -> ExecutionResult:
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
            env = os.environ.copy()

            if language == "python":
                cmd = [sys.executable, filepath]
            elif language == "ark":
                # Step 1: Transpile to JSON (MAST)
                json_path = filepath + ".json"
                transpile_cmd = [
                    sys.executable,
                    os.path.abspath("meta/ark_to_json.py"),
                    filepath,
                    "-o",
                    json_path
                ]

                try:
                    proc = await asyncio.create_subprocess_exec(
                        *transpile_cmd,
                        stdout=asyncio.subprocess.PIPE,
                        stderr=asyncio.subprocess.PIPE,
                        cwd=os.getcwd() # Run from repo root
                    )
                    stdout, stderr = await proc.communicate()
                    if proc.returncode != 0:
                        return ExecutionResult(
                            stdout=stdout.decode(),
                            stderr=f"Transpilation failed:\n{stderr.decode()}",
                            exit_code=proc.returncode,
                            duration_ms=(time.time() - start_time) * 1000,
                        )
                except Exception as e:
                     return ExecutionResult(
                        stdout="",
                        stderr=f"Transpilation error: {e}",
                        exit_code=1,
                        duration_ms=(time.time() - start_time) * 1000,
                    )

                # Step 2: Execute with Rust Runtime
                loader_path = os.path.abspath("target/release/ark_loader")
                if not os.path.exists(loader_path):
                     # Fallback to debug build if release missing?
                     loader_path = os.path.abspath("core/target/release/ark_loader")

                if not os.path.exists(loader_path):
                     return ExecutionResult(
                        stdout="",
                        stderr="Ark Runtime (ark_loader) not found. Please compile core.",
                        exit_code=1,
                        duration_ms=0.0
                    )

                cmd = [loader_path, json_path]
                # Run from root so intrinsic paths are relative to root?
                # Or temp dir?
                # If script imports from lib/std, we need root context.
                # But sandbox usually isolates.
                # Let's run from root but rely on loader security.
                cwd = os.getcwd()
            elif language == "javascript":
                cmd = ["node", filepath]
            elif language == "rust":
                # Compile first
                exe_path = os.path.join(temp_dir, "main")
                compile_cmd = ["rustc", filepath, "-o", exe_path]

                try:
                    proc = await asyncio.create_subprocess_exec(
                        *compile_cmd,
                        stdout=asyncio.subprocess.PIPE,
                        stderr=asyncio.subprocess.PIPE,
                        cwd=temp_dir
                    )
                    stdout, stderr = await proc.communicate()
                    if proc.returncode != 0:
                        return ExecutionResult(
                            stdout=stdout.decode(),
                            stderr=f"Compilation failed:\n{stderr.decode()}",
                            exit_code=proc.returncode,
                            duration_ms=(time.time() - start_time) * 1000,
                        )
                except FileNotFoundError:
                     return ExecutionResult(
                        stdout="",
                        stderr="rustc not found",
                        exit_code=1,
                        duration_ms=(time.time() - start_time) * 1000,
                    )

                cmd = [exe_path]

            # 4. Execution
            try:
                # Resource limits for Unix
                def preexec():
                    try:
                        import resource
                        # CPU limit (soft, hard)
                        resource.setrlimit(resource.RLIMIT_CPU, (timeout + 2, timeout + 5))
                        # Memory limit (512MB)
                        mem = 512 * 1024 * 1024
                        resource.setrlimit(resource.RLIMIT_AS, (mem, mem))
                    except ImportError:
                        pass
                    except ValueError:
                        pass

                process = await asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                    cwd=cwd,
                    preexec_fn=preexec if sys.platform != "win32" else None,
                    env=env
                )

                try:
                    stdout_data, stderr_data = await asyncio.wait_for(
                        process.communicate(), timeout=timeout
                    )
                except asyncio.TimeoutError:
                    try:
                        process.kill()
                    except ProcessLookupError:
                        pass
                    await process.wait()
                    return ExecutionResult(
                        stdout="",
                        stderr=f"Execution timed out after {timeout}s",
                        exit_code=-1,
                        duration_ms=(time.time() - start_time) * 1000,
                        truncated=False
                    )

                duration_ms = (time.time() - start_time) * 1000

                stdout_str = stdout_data.decode("utf-8", errors="ignore")
                stderr_str = stderr_data.decode("utf-8", errors="ignore")

                trunc_stdout, is_trunc_out = truncate_output(stdout_str)
                trunc_stderr, is_trunc_err = truncate_output(stderr_str)

                return ExecutionResult(
                    stdout=trunc_stdout,
                    stderr=trunc_stderr,
                    exit_code=process.returncode,
                    duration_ms=duration_ms,
                    truncated=(is_trunc_out or is_trunc_err)
                )

            except Exception as e:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Execution failed: {e}",
                    exit_code=1,
                    duration_ms=(time.time() - start_time) * 1000,
                )

if __name__ == "__main__":
    async def main():
        print("Verifying src/sandbox/local.py...")
        sandbox = LocalSandbox(capabilities={"net"})

        # 1. Test Python Success
        res = await sandbox.execute('print("Hello World")', "python")
        assert res.stdout.strip() == "Hello World"
        assert res.exit_code == 0
        print("Python execution: OK")

        # 2. Test Security Block
        res = await sandbox.execute('import os; os.system("ls")', "python")
        assert res.exit_code != 0
        assert "Security violations" in res.stderr
        print("Security block: OK")

        # 3. Test Ark (if available)
        if os.path.exists("meta/ark.py"):
            res = await sandbox.execute('print("Hello Ark")', "ark")
            if res.exit_code == 0:
                 print(f"Ark execution: OK ({res.stdout.strip()})")
            else:
                 print(f"Ark execution failed (expected if env incomplete): {res.stderr}")
        else:
            print("Ark execution: Skipped (meta/ark.py not found)")

        await sandbox.cleanup()
        print("Local verification complete.")

    asyncio.run(main())
