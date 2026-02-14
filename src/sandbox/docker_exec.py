import os
import tempfile
import time
from typing import Optional

from .base import CodeSandbox, ExecutionResult, truncate_output

# Security: Whitelist of allowed Docker images
ALLOWED_DOCKER_IMAGES = {
    "python:3.11-slim",
    "python:3.12-slim",
    "python:3.10-slim",
    "python:3.9-slim",
}
DEFAULT_DOCKER_IMAGE = "python:3.11-slim"


class DockerSandbox(CodeSandbox):
    """Docker-based sandbox (opt-in).

    This implementation performs lazy imports and graceful error handling so that
    environments without Docker SDK or daemon do not crash the application. It
    returns a structured error via ExecutionResult when unavailable.
    """
    _client = None

    def _get_client(self):
        """Lazy initialization of the Docker client."""
        if DockerSandbox._client is not None:
            return DockerSandbox._client

        try:
            import docker  # type: ignore
            # Initialize client and store it
            DockerSandbox._client = docker.from_env()
            return DockerSandbox._client
        except Exception:
            # If initialization fails, return None (caller should handle)
            return None

    def _docker_available(self) -> tuple[bool, Optional[str]]:
        try:
            client = self._get_client()

            if client is None:
                # Diagnosis: why did _get_client fail?
                try:
                    import docker  # type: ignore
                    try:
                        docker.from_env()
                    except Exception as e:
                         return False, f"Docker daemon not available: {e}"
                except ImportError as e:
                    return False, f"Docker SDK not installed: {e}"
                return False, "Docker unavailable"

            # Verify connectivity
            try:
                client.ping()
                return True, None
            except Exception as exc:
                # Connection lost or daemon down. Reset client to force re-init next time.
                DockerSandbox._client = None
                return False, f"Docker daemon not available: {exc}"

        except Exception as exc:
            return False, f"Docker check error: {exc}"

    def execute(self, code: str, language: str = "python", timeout: int = 30) -> ExecutionResult:
        ok, reason = self._docker_available()
        start = time.time()
        if not ok:
            return ExecutionResult(
                stdout="",
                stderr=reason or "Docker not available",
                exit_code=1,
                duration=time.time() - start,
                meta={
                    "runtime": "docker",
                    "timed_out": False,
                    "truncated": False,
                },
            )

        if language.lower() != "python":
            return ExecutionResult(
                stdout="",
                stderr=f"Unsupported language: {language}",
                exit_code=1,
                duration=time.time() - start,
                meta={"runtime": "docker", "timed_out": False, "truncated": False},
            )

        # Lazy imports only after availability confirmed
        # import docker # Not strictly needed here as _docker_available ensured it

        # Security: Validate Docker image against a whitelist
        image = os.getenv("DOCKER_IMAGE", DEFAULT_DOCKER_IMAGE)

        if image not in ALLOWED_DOCKER_IMAGES:
            # Fallback to default if image is not whitelisted
            image = DEFAULT_DOCKER_IMAGE

        network_enabled = os.getenv("DOCKER_NETWORK_ENABLED", "false").lower() == "true"
        cpu_limit = os.getenv("DOCKER_CPU_LIMIT", "0.5")
        mem_limit = os.getenv("DOCKER_MEMORY_LIMIT", "256m")

        # Reuse the client verified in _docker_available
        client = self._get_client()
        # Note: In a race condition (threaded), client could become None or invalid between
        # _docker_available and here. But for this sandbox, we assume single-threaded or robust enough.
        # If client is None here (which shouldn't happen if _docker_available is True), we'll crash or need checks.
        if client is None:
             return ExecutionResult(
                stdout="",
                stderr="Docker client lost during execution",
                exit_code=1,
                duration=time.time() - start,
                meta={"runtime": "docker", "timed_out": False, "truncated": False},
            )

        # Prepare a temp script file, then mount/run inside container
        with tempfile.TemporaryDirectory(prefix="ag_sbx_dk_") as tmpdir:
            script_path = os.path.join(tmpdir, "main.py")
            with open(script_path, "w", encoding="utf-8") as f:
                f.write(code)

            mounts = {tmpdir: {"bind": "/work", "mode": "ro"}}

            command = ["python", "/work/main.py"]

            try:
                container = client.containers.run(
                    image=image,
                    command=command,
                    volumes=mounts,
                    network_disabled=(not network_enabled),
                    mem_limit=mem_limit,
                    nano_cpus=int(float(cpu_limit) * 1e9),  # approximate CPU limit
                    detach=True,
                    stdout=True,
                    stderr=True,
                    working_dir="/work",
                    remove=True,
                )

                try:
                    exit_code = container.wait(timeout=timeout)["StatusCode"]
                except Exception:
                    # timeout enforcement: kill the container
                    try:
                        container.kill()
                    except Exception:
                        pass
                    return ExecutionResult(
                        stdout="",
                        stderr=f"Execution timed out after {timeout}s",
                        exit_code=-1,
                        duration=time.time() - start,
                        meta={
                            "runtime": "docker",
                            "timed_out": True,
                            "truncated": False,
                        },
                    )

                logs = container.logs(stdout=True, stderr=True)
                out = logs.decode("utf-8", errors="ignore")

                # Apply truncation
                max_output_kb = int(os.getenv("SANDBOX_MAX_OUTPUT_KB", "10"))
                max_bytes = max_output_kb * 1024
                stdout_trunc, truncated = truncate_output(out, max_bytes)

                return ExecutionResult(
                    stdout=stdout_trunc,
                    stderr="",
                    exit_code=int(exit_code),
                    duration=time.time() - start,
                    meta={
                        "runtime": "docker",
                        "timed_out": False,
                        "truncated": truncated,
                        "resource_limits": {
                            "timeout_sec": timeout,
                            "max_output_kb": max_output_kb,
                        },
                    },
                )
            except Exception as exc:
                return ExecutionResult(
                    stdout="",
                    stderr=f"Docker execution error: {exc}",
                    exit_code=1,
                    duration=time.time() - start,
                    meta={"runtime": "docker", "timed_out": False, "truncated": False},
                )
