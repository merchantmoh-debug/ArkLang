import os
import sys
import tempfile
import subprocess
import time
import json
import asyncio
from typing import Dict, Any, Optional

from src.sandbox.factory import get_sandbox


def run_python_code(code: str, timeout: Optional[int] = None) -> str:
    """
    Execute Python code using the configured sandbox.
    (Sync version for compatibility, consider using async wrapper if needed)
    """
    sandbox = get_sandbox()

    try:
        effective_timeout = (
            int(timeout) if timeout is not None else int(os.getenv("SANDBOX_TIMEOUT_SEC", "30"))
        )
    except Exception:
        effective_timeout = 30

    result = sandbox.execute(code=code, language="python", timeout=effective_timeout)

    if result.exit_code != 0:
        err = (result.stderr or "").strip()
        if not err:
            err = "Unknown error"
        return f"Error (exit_code={result.exit_code}): {err}"

    out = (result.stdout or "").strip()
    return out if out else "(no output)"


async def execute_ark(code: str, timeout: float = 30.0) -> Dict[str, Any]:
    """
    Execute Ark code using the meta/ark.py interpreter (Async).

    Args:
        code: The Ark source code to execute.
        timeout: Execution timeout in seconds.

    Returns:
        Dict with stdout, stderr, exit_code, duration_ms.
    """

    def _run_ark_sync():
        start_time = time.time()

        # Create temporary file for Ark code
        with tempfile.NamedTemporaryFile(mode='w', suffix='.ark', delete=False) as f:
            f.write(code)
            temp_path = f.name

        try:
            # Locate meta/ark.py
            base_dir = os.getcwd()
            ark_interpreter = os.path.join(base_dir, "meta", "ark.py")

            if not os.path.exists(ark_interpreter):
                return {
                    "stdout": "",
                    "stderr": f"Interpreter not found at {ark_interpreter}",
                    "exit_code": 1,
                    "duration_ms": 0
                }

            # Run via subprocess
            cmd = [sys.executable, ark_interpreter, "run", temp_path]

            try:
                proc = subprocess.run(
                    cmd,
                    capture_output=True,
                    text=True,
                    timeout=timeout,
                    cwd=base_dir
                )
                stdout = proc.stdout
                stderr = proc.stderr
                exit_code = proc.returncode
            except subprocess.TimeoutExpired:
                stdout = ""
                stderr = f"Execution timed out after {timeout}s"
                exit_code = 124

            duration_ms = (time.time() - start_time) * 1000

            return {
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": exit_code,
                "duration_ms": round(duration_ms, 2)
            }

        finally:
            if os.path.exists(temp_path):
                os.unlink(temp_path)

    return await asyncio.to_thread(_run_ark_sync)


# MCP Tool Definition
mcp_tool_def = {
    "name": "execute_ark",
    "description": "Execute Ark code via the reference interpreter.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "code": {
                "type": "string",
                "description": "Ark source code to execute"
            },
            "timeout": {
                "type": "number",
                "description": "Execution timeout in seconds (default 30)",
                "default": 30
            }
        },
        "required": ["code"]
    }
}
