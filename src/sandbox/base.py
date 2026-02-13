from dataclasses import dataclass
from typing import Protocol, Dict, Tuple


@dataclass
class ExecutionResult:
    stdout: str
    stderr: str
    exit_code: int
    duration: float
    meta: Dict[str, object]


def truncate_output(text: str, max_bytes: int) -> Tuple[str, bool]:
    if max_bytes <= 0:
        return text, False
    encoded = text.encode("utf-8", errors="ignore")
    if len(encoded) <= max_bytes:
        return text, False
    limit = max(0, max_bytes - 32)
    truncated = encoded[:limit].decode("utf-8", errors="ignore")
    return truncated + "\n... (output truncated)", True


class CodeSandbox(Protocol):
    """Abstract interface for any execution environment."""

    def execute(
        self,
        code: str,
        language: str = "python",
        timeout: int = 30,
    ) -> ExecutionResult:
        """
        Execute the provided code synchronously.
        Must handle timeouts and capture Stdout/Stderr.
        """
        ...
