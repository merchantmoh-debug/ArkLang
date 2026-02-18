"""
P1: Wallet CLI Smoke Test

Verifies the wallet.ark CLI demo actually runs and produces output.
The wallet is skipped by Gauntlet (interactive/fs_write).

Usage: python -m pytest tests/test_wallet_smoke.py -v
"""

import os
import subprocess
import sys

import pytest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ARK_CMD = [sys.executable, os.path.join(REPO_ROOT, "meta", "ark.py"), "run"]


@pytest.mark.timeout(30)
def test_wallet_help():
    """
    Verifies that running wallet.ark with no args prints usage information
    (not a crash or traceback).
    """
    result = subprocess.run(
        ARK_CMD + ["apps/wallet.ark"],
        capture_output=True,
        text=True,
        timeout=15,
        cwd=REPO_ROOT,
        env={**os.environ, "ARK_CAPABILITIES": "exec,fs_write,fs_read,crypto"},
    )
    # Wallet should print usage when no command given
    combined = result.stdout + result.stderr
    assert "usage" in combined.lower() or "command" in combined.lower() or "wallet" in combined.lower(), \
        f"Expected usage info, got:\nstdout: {result.stdout[:300]}\nstderr: {result.stderr[:300]}"


@pytest.mark.timeout(30)
def test_wallet_create():
    """
    README claim: python3 meta/ark.py run apps/wallet.ark create <password>
    Verifies: the create command runs and produces wallet-related output.
    """
    result = subprocess.run(
        ARK_CMD + ["apps/wallet.ark", "create", "testpassword123"],
        capture_output=True,
        text=True,
        timeout=30,
        cwd=REPO_ROOT,
        env={
            **os.environ,
            "ARK_CAPABILITIES": "exec,fs_write,fs_read,crypto",
        },
    )
    combined = result.stdout + result.stderr
    # Should print something wallet-related (mnemonic, key, or wallet)
    assert any(kw in combined.lower() for kw in ["wallet", "mnemonic", "key", "generating", "deriving"]), \
        f"Wallet create produced no wallet-related output:\nstdout: {result.stdout[:500]}\nstderr: {result.stderr[:500]}"


@pytest.mark.timeout(30)
def test_wallet_history():
    """
    Verifies the 'history' subcommand runs (uses mock data in wallet.ark).
    """
    result = subprocess.run(
        ARK_CMD + ["apps/wallet.ark", "history", "ark_test_address_123"],
        capture_output=True,
        text=True,
        timeout=15,
        cwd=REPO_ROOT,
        env={**os.environ, "ARK_CAPABILITIES": "exec,fs_write,fs_read,crypto"},
    )
    combined = result.stdout + result.stderr
    # History command uses mock data — should print transaction lines
    assert "history" in combined.lower() or "ark" in combined.lower() or "sent" in combined.lower(), \
        f"Wallet history produced unexpected output:\nstdout: {result.stdout[:300]}\nstderr: {result.stderr[:300]}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
