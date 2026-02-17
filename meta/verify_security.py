import sys
import subprocess
import os

def verify_fail(path):
    print(f"Verifying security block for {path}...")

    if not os.path.exists(path):
        print(f"Error: Test file not found: {path}")
        sys.exit(1)

    # Command: python meta/ark.py run <path>
    cmd = [sys.executable, "meta/ark.py", "run", path]

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True
    )

    if result.returncode == 0:
        print(f"FAIL: {path} executed successfully (Exit 0) but was expected to fail.")
        print("Stdout:\n" + result.stdout)
        print("Stderr:\n" + result.stderr)
        sys.exit(1)

    # Accept "SandboxViolation" or "RuntimeError: Access outside working directory" as valid blocks
    if "SandboxViolation" in result.stderr or "Access outside working directory" in result.stderr:
        print(f"PASS: {path} blocked with SandboxViolation/RuntimeError.")
        print(f"Output: {result.stderr.strip()}")
        sys.exit(0)
    else:
        print(f"FAIL: {path} failed (Exit {result.returncode}) but NOT due to SandboxViolation.")
        print("Stderr output:\n" + result.stderr)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python meta/verify_security.py <ark_script>")
        sys.exit(1)

    script_path = sys.argv[1]
    verify_fail(script_path)
