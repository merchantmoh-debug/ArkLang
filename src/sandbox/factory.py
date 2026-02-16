import os
import sys
import asyncio
from typing import Dict, Set, Tuple, Optional

from .base import BaseSandbox, SandboxError
from .local import LocalSandbox
from .docker_exec import DockerSandbox

# Singleton cache: (type, capabilities_frozenset) -> instance
_SANDBOX_CACHE: Dict[Tuple[str, frozenset], BaseSandbox] = {}


def create_sandbox(sandbox_type: str = "auto", capabilities: Set[str] = None) -> BaseSandbox:
    """
    Factory to create or retrieve a sandbox instance.

    Args:
        sandbox_type: "auto", "docker", or "local".
        capabilities: Set of capability strings.

    Returns:
        A ready-to-use BaseSandbox instance.

    Raises:
        SandboxError: If the requested sandbox type is unavailable.
    """
    if capabilities is None:
        capabilities = set()

    caps_key = frozenset(capabilities)
    sandbox_type = sandbox_type.lower()

    # Check cache first (for explicit types)
    # Note: "auto" resolves to concrete type, so we cache by concrete type.

    if sandbox_type == "auto":
        # Resolution logic
        # 1. Try Docker
        # We need to check if Docker is available.
        # DockerSandbox has _docker_available method but it's an instance method or class method?
        # In my implementation it's an instance method but uses class-level client.
        # I made it an instance method `_docker_available(self)`.
        # I should probably instantiate it to check, or make it static.
        # Let's try to instantiate DockerSandbox and check.

        # Check if we have a cached Docker instance for these caps
        docker_key = ("docker", caps_key)
        if docker_key in _SANDBOX_CACHE:
            return _SANDBOX_CACHE[docker_key]

        # Try creating DockerSandbox
        try:
            # We can't easily check availability without async await if we use the async method.
            # But _docker_available in DockerSandbox is synchronous internal logic wrapped in async in execute.
            # Actually I made `_docker_available` synchronous in `DockerSandbox` but `execute` calls it via `to_thread`.
            # So I can call it synchronously here.

            # Use a temporary instance to check availability?
            # Or just try to return a DockerSandbox and let it fail at execution time?
            # The prompt says: "auto -> try Docker first, fall back to Local".
            # This implies we should know if Docker works *before* falling back.
            # But `create_sandbox` is synchronous.
            # If I return a DockerSandbox that fails later, it's not "falling back".
            # So I must check availability here.

            ds = DockerSandbox(capabilities)
            # Accessing private method is ugly but practical here, or assume DockerSandbox has a public check?
            # I'll rely on the fact that I wrote DockerSandbox.
            is_available, _ = ds._docker_available()

            if is_available:
                _SANDBOX_CACHE[docker_key] = ds
                return ds

            # If not available, fall back to Local
            # Verify if Local is acceptable? "LocalSandbox is insecure..."
            # Auto implies best effort.

        except Exception:
            # If Docker instantiation fails (e.g. missing deps), fall back
            pass

        # Fallback to Local
        sandbox_type = "local"

    # Handle explicit types
    cache_key = (sandbox_type, caps_key)
    if cache_key in _SANDBOX_CACHE:
        return _SANDBOX_CACHE[cache_key]

    if sandbox_type == "docker":
        ds = DockerSandbox(capabilities)
        # We don't check availability here, we let it fail at runtime if requested explicitly?
        # Or should we raise immediately?
        # "raise if Docker not available"
        is_available, reason = ds._docker_available()
        if not is_available:
             raise SandboxError(f"Docker sandbox unavailable: {reason}")

        _SANDBOX_CACHE[cache_key] = ds
        return ds

    if sandbox_type == "local":
        ls = LocalSandbox(capabilities)
        _SANDBOX_CACHE[cache_key] = ls
        return ls

    raise SandboxError(f"Unknown sandbox type: {sandbox_type}")


if __name__ == "__main__":
    # Test logic
    print("Verifying src/sandbox/factory.py...")

    # 1. Local creation
    sb_local = create_sandbox("local", {"net"})
    assert isinstance(sb_local, LocalSandbox)
    assert "net" in sb_local.get_capabilities()

    # 2. Singleton check
    sb_local2 = create_sandbox("local", {"net"})
    assert sb_local is sb_local2
    print("Singleton pattern: OK")

    # 3. Auto resolution
    # Depends on environment.
    sb_auto = create_sandbox("auto")
    print(f"Auto resolved to: {type(sb_auto).__name__}")

    # 4. Capability check
    sb_caps = create_sandbox("local", {"fs"})
    assert sb_caps is not sb_local # Different capabilities
    print("Capability caching: OK")

    print("Factory verification complete.")
