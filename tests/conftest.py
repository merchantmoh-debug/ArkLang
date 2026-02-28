"""
conftest.py — Nuclear Test Isolation for Ark Compiler Test Suite

Root cause: Several tests corrupt sys.modules by replacing real modules
with MagicMock objects (at module level or in setUp). This poisons subsequent
tests that try to importlib.reload() the corrupted modules.

Strategy:
1. SESSION-SCOPE: Eagerly import and snapshot all protected modules at session start
   (before any test-level corruption can occur).
2. FUNCTION-SCOPE: Restore all protected modules after each test function.
3. ENV CLEANUP: Snapshot and restore critical environment variables.
"""
import sys
import os
import pytest


# ─── Session-Level Module Snapshot ───────────────────────────────────────────
# Captured ONCE at conftest import time, before any test collection runs.
# This is the "golden" state that all tests should see.

_PROTECTED_MODULE_NAMES = [
    "src.config",
    "src.sandbox.local",
    "src.sandbox.docker_exec",
    "src.sandbox.factory",
    "src.mcp_client",
    "pydantic",
    "pydantic_settings",
    "lark",
]

# Eagerly import the critical modules to establish "golden" state
try:
    import src.config  # noqa: F401
except Exception:
    pass

try:
    import lark  # noqa: F401
except Exception:
    pass

# Snapshot the golden state
_GOLDEN_MODULES = {}
for _mod_name in _PROTECTED_MODULE_NAMES:
    if _mod_name in sys.modules:
        _GOLDEN_MODULES[_mod_name] = sys.modules[_mod_name]

# Capture lark.__spec__ specifically (it gets corrupted by mock injection)
_LARK_SPEC_GOLDEN = None
if "lark" in sys.modules and hasattr(sys.modules["lark"], "__spec__"):
    _LARK_SPEC_GOLDEN = sys.modules["lark"].__spec__


@pytest.fixture(autouse=True)
def _decontaminate_modules():
    """
    After each test: restore protected modules to golden state,
    undoing any sys.modules corruption.
    """
    yield

    # ── POST-TEST CLEANUP ──
    for mod_name, golden_module in _GOLDEN_MODULES.items():
        current = sys.modules.get(mod_name)
        if current is not golden_module:
            # Module was corrupted — restore it
            sys.modules[mod_name] = golden_module

    # Restore lark.__spec__ if it was corrupted
    if _LARK_SPEC_GOLDEN is not None and "lark" in sys.modules:
        try:
            if sys.modules["lark"].__spec__ is not _LARK_SPEC_GOLDEN:
                sys.modules["lark"].__spec__ = _LARK_SPEC_GOLDEN
        except (AttributeError, TypeError):
            pass


# ─── Environment Variable Safety ────────────────────────────────────────────

_ENV_KEYS_TO_PROTECT = [
    "ALLOW_DANGEROUS_LOCAL_EXECUTION",
    "SECRET_TOKEN",
    "HOST_SECRET",
    "SANDBOX_MAX_OUTPUT_KB",
    "ARK_CAPABILITIES",
    "DOCKER_IMAGE",
]


@pytest.fixture(autouse=True)
def _clean_env_vars():
    """Snapshot and restore critical environment variables after each test."""
    saved = {}
    for key in _ENV_KEYS_TO_PROTECT:
        if key in os.environ:
            saved[key] = os.environ[key]

    yield

    for key in _ENV_KEYS_TO_PROTECT:
        if key in saved:
            os.environ[key] = saved[key]
        else:
            os.environ.pop(key, None)
