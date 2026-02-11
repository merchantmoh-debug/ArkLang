import subprocess
import sys
import os

def test_jailbreak():
    print("Running jailbreak attempt...")

    # Run the ark interpreter on the jailbreak attempt file
    # Ensure we are in the root of the repo
    if not os.path.exists("meta/ark.py"):
        print("Error: must run from repo root")
        sys.exit(1)

    # meta/ark.py expects sys.argv[2] to be the file, so we need a dummy argument at index 1
    # Usage: python meta/ark.py <action> <file>
    cmd = [sys.executable, "meta/ark.py", "run", "tests/jailbreak_attempt.ark"]

    # We expect it to fail
    # We set env var to ensure dangerous execution is NOT allowed (default is false, but being explicit helps)
    env = os.environ.copy()
    env["ALLOW_DANGEROUS_LOCAL_EXECUTION"] = "false"

    result = subprocess.run(cmd, capture_output=True, text=True, env=env)

    print("STDOUT:", result.stdout)
    print("STDERR:", result.stderr)

    if result.returncode == 0:
        print("FAIL: Process exited with 0 (success) but should have failed.")
        sys.exit(1)

    if "SandboxViolation" not in result.stderr:
        print("FAIL: 'SandboxViolation' not found in stderr.")
        sys.exit(1)

    print("SUCCESS: Jailbreak prevented with SandboxViolation.")

if __name__ == "__main__":
    test_jailbreak()
