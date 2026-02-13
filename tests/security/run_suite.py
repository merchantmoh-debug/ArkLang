import subprocess
import sys
import os

# Adjust path to find meta/ark.py assuming running from repo root
REPO_ROOT = os.getcwd()
ARK_INTERPRETER = os.path.join(REPO_ROOT, "meta", "ark.py")

def run_test(filename):
    print(f"Running {filename}...")
    # Ensure ALLOW_DANGEROUS_LOCAL_EXECUTION is NOT set to true
    env = os.environ.copy()
    env["ALLOW_DANGEROUS_LOCAL_EXECUTION"] = "false"

    result = subprocess.run(
        [sys.executable, ARK_INTERPRETER, "run", filename],
        capture_output=True,
        text=True,
        env=env,
        cwd=REPO_ROOT
    )

    if result.returncode == 0:
        print(f"FAIL: {filename} exited successfully (expected failure).")
        return False

    if "SandboxViolation" not in result.stderr:
        print(f"FAIL: {filename} did not report SandboxViolation.")
        print("STDERR output:\n", result.stderr)
        return False

    print(f"PASS: {filename} failed as expected with SandboxViolation.")
    return True

def main():
    tests = [
        "tests/security/path_traversal.ark",
        "tests/security/exec_violation.ark",
        "tests/security/repo_overwrite.ark",
        "tests/security/apps_overwrite.ark",
        "tests/security/root_file_overwrite.ark",
        "tests/security/git_overwrite.ark"
    ]

    failures = 0
    for test in tests:
        if not run_test(test):
            failures += 1

    if failures > 0:
        print(f"{failures} tests failed.")
        sys.exit(1)

    print("All security tests passed.")

if __name__ == "__main__":
    main()
