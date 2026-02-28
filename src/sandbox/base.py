from abc import ABC, abstractmethod
from dataclasses import dataclass, field
import time
from typing import Set, Tuple


class SandboxError(Exception):
    """Base exception for sandbox errors."""
    def __init__(self, message: str, code: str = None, language: str = None):
        super().__init__(message)
        self.code = code
        self.language = language

class SandboxTimeoutError(SandboxError):
    """Raised when execution exceeds the timeout."""
    pass

class SandboxMemoryError(SandboxError):
    """Raised when execution exceeds memory limits."""
    pass

class SandboxSecurityError(SandboxError):
    """Raised when execution violates security policies."""
    pass


@dataclass
class ExecutionResult:
    stdout: str
    stderr: str
    exit_code: int
    duration_ms: float
    truncated: bool = False
    timed_out: bool = False

    def __init__(self, stdout: str = "", stderr: str = "", exit_code: int = 0,
                 duration_ms: float = 0.0, truncated: bool = False,
                 timed_out: bool = False, duration: float = None, meta: dict = None):
        self.stdout = stdout
        self.stderr = stderr
        self.exit_code = exit_code
        self.duration_ms = duration_ms if duration is None else duration
        self.truncated = truncated
        self.timed_out = timed_out

    @property
    def meta(self):
        """Backward-compat dict access for test assertions."""
        return {
            "truncated": self.truncated,
            "timed_out": self.timed_out,
            "duration_ms": self.duration_ms,
        }

    def __str__(self):
        return (
            f"Exit Code: {self.exit_code}\n"
            f"Duration: {self.duration_ms:.2f}ms\n"
            f"Truncated: {self.truncated}\n"
            f"Stdout: {self.stdout}\n"
            f"Stderr: {self.stderr}"
        )


def truncate_output(text: str, max_bytes: int = 100 * 1024) -> Tuple[str, bool]:
    """Truncates output to max_bytes and appends a marker if truncated."""
    if not text:
        return "", False

    # 0 or negative means unlimited
    if max_bytes <= 0:
        return text, False

    encoded = text.encode("utf-8", errors="ignore")
    if len(encoded) <= max_bytes:
        return text, False

    trailer = "\n... (output truncated)"
    # Reserve space for trailer (use 32 as reservation size to match test expectations)
    limit = max(0, max_bytes - 32)
    truncated_bytes = encoded[:limit]

    # Decode safely (drop partial multi-byte chars)
    truncated_text = truncated_bytes.decode("utf-8", errors="ignore")

    return truncated_text + trailer, True


class BaseSandbox(ABC):
    """Abstract base class for sandbox execution environments."""

    def __init__(self, capabilities: Set[str] = None):
        self.capabilities = capabilities or set()

    @abstractmethod
    def execute(
        self,
        code: str,
        language: str = "python",
        timeout: int = 30,
    ) -> ExecutionResult:
        """
        Execute the provided code.
        Must handle timeouts and capture Stdout/Stderr.
        """
        pass

    def cleanup(self):
        """Release any resources held by the sandbox."""
        pass

    def get_capabilities(self) -> Set[str]:
        """Return the set of available capabilities."""
        return self.capabilities


if __name__ == "__main__":
    # verification
    import asyncio

    print("Verifying src/sandbox/base.py...")

    # 1. Test truncate_output
    text = "A" * 200
    trunc, is_trunc = truncate_output(text, max_bytes=100)
    assert is_trunc
    assert len(trunc) <= 100 + len("\n[TRUNCATED]") # roughly
    assert trunc.endswith("[TRUNCATED]")
    print("truncate_output: OK")

    # 2. Test ExecutionResult
    res = ExecutionResult(stdout="out", stderr="err", exit_code=0, duration_ms=10.5, truncated=False)
    assert res.stdout == "out"
    print("ExecutionResult: OK")

    # 3. Test Exceptions
    try:
        raise SandboxTimeoutError("Timed out", code="print(1)", language="python")
    except SandboxError as e:
        assert e.code == "print(1)"
        print("Exceptions: OK")

    print("Base verification complete.")
