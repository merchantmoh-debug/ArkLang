import os
import sys
from .base import CodeSandbox
from .local import LocalSandbox


def get_sandbox() -> CodeSandbox:
    """Factory method to obtain the configured executor.

    Supported types: docker (default), local (opt-in), e2b (future)
    Raises RuntimeError if the requested type module is unavailable.
    """
    mode = os.getenv("SANDBOX_TYPE")
    if mode is None:
        mode = "docker"  # Secure default
    else:
        mode = mode.lower()

    if mode == "docker":
        try:
            from .docker_exec import DockerSandbox  # type: ignore

            return DockerSandbox()
        except ImportError:
            raise RuntimeError(
                "Docker sandbox requested but 'docker' package is not installed."
            )
        except Exception as e:
            raise RuntimeError(f"Failed to initialize Docker sandbox: {e}")

    if mode == "e2b":
        try:
            from .e2b_exec import E2BSandbox  # type: ignore

            return E2BSandbox()
        except ImportError:
            raise RuntimeError(
                "E2B sandbox requested but 'e2b' package is not installed."
            )
        except Exception as e:
            raise RuntimeError(f"Failed to initialize E2B sandbox: {e}")

    if mode == "local":
        print(
            "WARNING: LocalSandbox is insecure and allows arbitrary code execution on the host machine. Use with caution.",
            file=sys.stderr,
        )
        return LocalSandbox()

    raise ValueError(f"Unknown sandbox type: {mode}")
