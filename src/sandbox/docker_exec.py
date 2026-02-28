import asyncio
import os
import sys
import tempfile
import time
from typing import Optional, Set

from .base import (
    BaseSandbox,
    ExecutionResult,
    truncate_output,
    SandboxError,
    SandboxTimeoutError,
)

# Security: Whitelist of allowed Docker images
ALLOWED_DOCKER_IMAGES = {
    "ark-sandbox:latest",
    "python:3.11-slim",
    "python:3.12-slim",
    "python:3.10-slim",
    "python:3.9-slim",
}
DEFAULT_DOCKER_IMAGE = "ark-sandbox:latest"


class DockerSandbox(BaseSandbox):
    """Docker-based sandbox for isolated execution."""

    _client = None

    def __init__(self, capabilities: Set[str] = None):
        super().__init__(capabilities)

    def _get_client(self):
        """Lazy initialization of the Docker client."""
        if DockerSandbox._client is not None:
            return DockerSandbox._client

        try:
            import docker
            DockerSandbox._client = docker.from_env()
            return DockerSandbox._client
        except Exception:
            return None

    def _docker_available(self) -> tuple[bool, Optional[str]]:
        try:
            client = self._get_client()
            if client is None:
                return False, "Docker SDK not installed or daemon not running"
            client.ping()
            return True, None
        except Exception as exc:
            DockerSandbox._client = None
            return False, f"Docker daemon not available: {exc}"

    async def execute(
        self,
        code: str,
        language: str = "python",
        timeout: int = 30,
    ) -> ExecutionResult:
        # Check availability
        ok, reason = await asyncio.to_thread(self._docker_available)
        if not ok:
            return ExecutionResult(
                stdout="",
                stderr=reason or "Docker unavailable",
                exit_code=1,
                duration_ms=0.0,
                truncated=False
            )

        start_time = time.time()

        def run_container_logic():
            # This runs in a thread to do blocking IO (file write, docker start)
            import docker
            client = self._get_client()

            # Create temp dir for this run
            # We use mkdtemp to ensure it persists until we explicitly cleanup or context ends
            # But context manager is safer.
            # We can't yield from thread easily.
            # So we do everything inside the context in the thread?
            # No, we need to return the container object to main thread to wait on it?
            # Container objects are not thread-safe? usually fine.
            # But simpler: run the whole start logic in thread, return container object.

            tmpdir = tempfile.mkdtemp(prefix="ark_docker_")

            try:
                # Write code
                filename = "main.py"
                if language == "javascript": filename = "main.js"
                elif language == "rust": filename = "main.rs"
                elif language == "ark": filename = "main.ark"

                filepath = os.path.join(tmpdir, filename)
                with open(filepath, "w", encoding="utf-8") as f:
                    f.write(code)

                # Make script executable just in case
                os.chmod(filepath, 0o755)

                mounts = {
                    tmpdir: {"bind": "/workspace", "mode": "ro"}
                }

                image = os.getenv("DOCKER_IMAGE", DEFAULT_DOCKER_IMAGE)
                if image not in ALLOWED_DOCKER_IMAGES:
                    image = DEFAULT_DOCKER_IMAGE

                cmd = []
                if language == "python":
                    cmd = ["python", f"/workspace/{filename}"]
                elif language == "javascript":
                    cmd = ["node", f"/workspace/{filename}"]
                elif language == "rust":
                    # Compile to /tmp (tmpfs) and run
                    cmd = ["sh", "-c", f"rustc /workspace/{filename} -o /tmp/main && /tmp/main"]
                elif language == "ark":
                    cmd = ["python", "/app/meta/ark.py", "run", f"/workspace/{filename}"]
                else:
                    raise ValueError(f"Unsupported language: {language}")

                container = client.containers.run(
                    image=image,
                    command=cmd,
                    volumes=mounts,
                    network_disabled=True,
                    mem_limit="256m",
                    nano_cpus=500000000,
                    read_only=True,
                    tmpfs={'/tmp': ''},
                    detach=True,
                    working_dir="/workspace",
                )
                return container, tmpdir
            except Exception as e:
                # Clean up tmpdir if failed
                try:
                    import shutil
                    shutil.rmtree(tmpdir)
                except: pass
                raise e

        # 1. Start Container
        try:
            container, tmpdir = await asyncio.to_thread(run_container_logic)
        except Exception as e:
            return ExecutionResult(
                stdout="",
                stderr=f"Failed to start container: {e}",
                exit_code=1,
                duration_ms=(time.time() - start_time) * 1000,
                truncated=False
            )

        # 2. Wait for completion or timeout
        timed_out = False
        exit_code = -1

        try:
            # We use asyncio.wait_for on a thread that blocks on container.wait()
            # container.wait() returns {'StatusCode': int}
            wait_result = await asyncio.wait_for(
                asyncio.to_thread(container.wait),
                timeout=timeout
            )
            exit_code = wait_result.get("StatusCode", 1)

        except asyncio.TimeoutError:
            timed_out = True
            # Kill container
            try:
                await asyncio.to_thread(container.kill)
            except Exception:
                pass
        except Exception as e:
            # Other error
            try:
                await asyncio.to_thread(container.kill)
            except Exception:
                pass

            # Clean up
            await asyncio.to_thread(cleanup_helper, container, tmpdir)

            return ExecutionResult(
                stdout="",
                stderr=f"Execution error: {e}",
                exit_code=1,
                duration_ms=(time.time() - start_time) * 1000,
                truncated=False
            )

        # 3. Capture Logs
        try:
            # logs() returns bytes
            stdout_bytes = await asyncio.to_thread(container.logs, stdout=True, stderr=False)
            stderr_bytes = await asyncio.to_thread(container.logs, stdout=False, stderr=True)

            stdout_str = stdout_bytes.decode("utf-8", errors="ignore")
            stderr_str = stderr_bytes.decode("utf-8", errors="ignore")
        except Exception as e:
            stdout_str = ""
            stderr_str = f"Failed to retrieve logs: {e}"

        # 4. Cleanup
        await asyncio.to_thread(cleanup_helper, container, tmpdir)

        duration_ms = (time.time() - start_time) * 1000

        # Read configurable max output size (default 100KB)
        max_kb = int(os.environ.get("SANDBOX_MAX_OUTPUT_KB", "100"))
        max_bytes = max_kb * 1024

        trunc_out, is_trunc1 = truncate_output(stdout_str, max_bytes=max_bytes)
        trunc_err, is_trunc2 = truncate_output(stderr_str, max_bytes=max_bytes)

        if timed_out:
            return ExecutionResult(
                stdout=trunc_out,
                stderr="Execution timed out" + ("\n" + trunc_err if trunc_err else ""),
                exit_code=-1,
                duration_ms=duration_ms,
                truncated=(is_trunc1 or is_trunc2)
            )

        return ExecutionResult(
            stdout=trunc_out,
            stderr=trunc_err,
            exit_code=exit_code,
            duration_ms=duration_ms,
            truncated=(is_trunc1 or is_trunc2)
        )

    async def cleanup(self):
        pass


def cleanup_helper(container, tmpdir):
    try:
        container.remove(force=True)
    except:
        pass
    try:
        import shutil
        shutil.rmtree(tmpdir)
    except:
        pass


if __name__ == "__main__":
    async def main():
        print("Verifying src/sandbox/docker_exec.py...")
        sandbox = DockerSandbox()

        # Check availability
        ok, reason = await asyncio.to_thread(sandbox._docker_available)
        if not ok:
            print(f"Docker not available: {reason}")
            print("Skipping actual Docker tests.")
            return

        # Check if we can run python:3.11-slim for testing
        import docker
        client = docker.from_env()
        try:
            client.images.get("python:3.11-slim")
            os.environ["DOCKER_IMAGE"] = "python:3.11-slim"
            print("Using python:3.11-slim for tests.")
        except:
            print("python:3.11-slim not found locally. Tests might fail if pull fails.")

        print("Docker is available. Running tests...")

        # 1. Simple Python
        res = await sandbox.execute('print("Hello Docker")', "python")
        print(f"Result 1: Exit={res.exit_code}, Err={res.stderr}")

        err_lower = res.stderr.lower()
        image_missing = "not found" in err_lower or "pull access denied" in err_lower

        if res.exit_code == 0:
            assert "Hello Docker" in res.stdout
            print("Docker Python: OK")
        elif image_missing:
             print("Docker Python: Skipped (Image missing)")
        else:
            print(f"Docker Python Failed: {res.stderr}")

        # 2. Timeout
        if not image_missing:
            res = await sandbox.execute('import time; time.sleep(2)', "python", timeout=1)
            print(f"Result 2: Exit={res.exit_code}, Err={res.stderr}")
            if res.exit_code == -1:
                assert "timed out" in res.stderr or "Timeout" in res.stderr
                print("Docker Timeout: OK")
            else:
                 print(f"Docker Timeout Test Failed (expected -1): {res.exit_code}")

    asyncio.run(main())
