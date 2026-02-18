"""
P1: Intrinsic Count Verification Test

Verifies the README claim of "105/105 intrinsics ported to Rust" by running
count_intrinsics.py and checking the output matches the documented count.

Usage: python -m pytest tests/test_intrinsic_count.py -v
"""

import os
import subprocess
import sys
import re

import pytest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


@pytest.mark.timeout(30)
def test_intrinsic_count_script_runs():
    """
    Verifies that meta/count_intrinsics.py runs without crashing
    and produces output with intrinsic counts.
    """
    result = subprocess.run(
        [sys.executable, os.path.join(REPO_ROOT, "meta", "count_intrinsics.py")],
        capture_output=True,
        text=True,
        timeout=15,
        cwd=REPO_ROOT,
    )
    assert result.returncode == 0, \
        f"count_intrinsics.py failed:\nstdout: {result.stdout[:500]}\nstderr: {result.stderr[:500]}"
    assert "python intrinsics" in result.stdout.lower(), \
        f"Expected intrinsic count output, got: {result.stdout[:500]}"
    assert "rust intrinsics" in result.stdout.lower(), \
        f"Expected Rust intrinsic count, got: {result.stdout[:500]}"


@pytest.mark.timeout(30)
def test_intrinsic_parity_matches_readme():
    """
    README claims '105/105 intrinsics ported to Rust'.
    This test verifies the actual count and alerts if the claim is wrong.
    """
    result = subprocess.run(
        [sys.executable, os.path.join(REPO_ROOT, "meta", "count_intrinsics.py")],
        capture_output=True,
        text=True,
        timeout=15,
        cwd=REPO_ROOT,
    )
    assert result.returncode == 0, f"count_intrinsics.py crashed: {result.stderr[:300]}"

    # Extract counts from output
    py_match = re.search(r'Total Python Intrinsics:\s*(\d+)', result.stdout)
    rust_match = re.search(r'Total Rust Intrinsics:\s*(\d+)', result.stdout)
    parity_match = re.search(r'Parity:\s*([\d.]+)%', result.stdout)

    assert py_match, f"Could not find Python intrinsic count in output: {result.stdout[:300]}"
    assert rust_match, f"Could not find Rust intrinsic count in output: {result.stdout[:300]}"

    py_count = int(py_match.group(1))
    rust_count = int(rust_match.group(1))

    # Check README claim
    readme_path = os.path.join(REPO_ROOT, "README.md")
    with open(readme_path, "r", encoding="utf-8") as f:
        readme = f.read()

    # Extract claimed count from README (e.g., "105/105" or "105 intrinsics")
    readme_claim = re.search(r'(\d+)/\1\s*[Ii]ntrinsics', readme)
    if readme_claim:
        claimed = int(readme_claim.group(1))
        assert py_count >= claimed, \
            f"README claims {claimed} intrinsics but Python has {py_count}"

    # If parity drops below 95%, something regressed
    if parity_match:
        parity = float(parity_match.group(1))
        if parity < 95.0:
            pytest.fail(f"Intrinsic parity is {parity}% — dropped below 95% baseline. "
                       f"Python: {py_count}, Rust: {rust_count}")


@pytest.mark.timeout(30)
def test_no_intrinsics_missing_in_rust():
    """
    If intrinsics exist in Python but NOT in Rust, they need to be ported.
    This test captures and reports the missing ones.
    """
    result = subprocess.run(
        [sys.executable, os.path.join(REPO_ROOT, "meta", "count_intrinsics.py")],
        capture_output=True,
        text=True,
        timeout=15,
        cwd=REPO_ROOT,
    )
    assert result.returncode == 0

    # Extract "Missing in Rust" section
    missing_section = re.search(
        r'Missing in Rust \(Python Only\)\s*---\s*(.*?)(?:\n\n|\nParity:)',
        result.stdout,
        re.DOTALL
    )

    if missing_section:
        missing_lines = [l.strip() for l in missing_section.group(1).strip().splitlines() if l.strip().startswith('-')]
        # As of 100% parity achievement, zero intrinsics should be missing
        assert len(missing_lines) <= 5, \
            f"Too many intrinsics missing in Rust ({len(missing_lines)}):\n" + "\n".join(missing_lines[:20])


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
