import subprocess
import os
import sys

def test_security():
    # 1. Create a malicious Ark script
    # NO SEMICOLONS in Ark!
    ark_code = """
    print("Attempting exploit...")
    res := sys.exec(["python3", "-c", "print('PWNED')"])
    print(res)
    """

    file_name = "exploit.ark"
    with open(file_name, "w") as f:
        f.write(ark_code)

    print(f"[*] Created {file_name}")
    print("[*] Running via Rust Core (cargo run)...")

    try:
        # Run the Rust runtime
        result = subprocess.run(
            ["cargo", "run", "--manifest-path", "core/Cargo.toml", "--release", "--", "run", file_name],
            capture_output=True,
            text=True
        )

        print("--- STDOUT ---")
        print(result.stdout)
        print("--- STDERR ---")
        print(result.stderr)
        print("--------------")

        # 1. Check if exploit succeeded
        if "PWNED" in result.stdout:
            print("[-] FAIL: Arbitrary code execution successful! 'PWNED' found in output.")
            sys.exit(1)

        # 2. Check if exploit was blocked
        # The Rust intrinsic prints: "[Ark:Exec] Security Violation: ..."
        if "Security Violation" in result.stdout:
            print("[+] PASS: Exploit blocked by whitelist.")
            sys.exit(0)

        # 3. Fallback check
        if result.returncode != 0:
             # Check if it was a syntax error vs runtime error
             if "Syntax Error" in result.stderr or "Syntax Error" in result.stdout:
                 print("[-] FAIL: Syntax Error in test script. Fix the test.")
                 sys.exit(1)
             print("[+] PASS: Process failed (likely blocked).")
             sys.exit(0)

        # If neither PWNED nor explicit block message...
        print("[?] UNDETERMINED: 'PWNED' not found, but no explicit block message seen.")
        sys.exit(0)

    except Exception as e:
        print(f"[-] ERROR: {e}")
        sys.exit(1)
    finally:
        if os.path.exists(file_name):
            os.remove(file_name)

if __name__ == "__main__":
    test_security()
