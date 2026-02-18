import sys
import os
import subprocess

# Ensure we can import meta.ark if needed, but we run it as subprocess
ARK_PATH = "meta/ark.py"
STD_TIME = "lib/std/time.ark"
STD_CRYPTO = "lib/std/crypto.ark"
TEST_ARK = "temp_test_std.ark"

def main():
    if not os.path.exists(STD_TIME):
        print(f"Error: {STD_TIME} not found")
        sys.exit(1)
    if not os.path.exists(STD_CRYPTO):
        print(f"Error: {STD_CRYPTO} not found")
        sys.exit(1)

    with open(STD_TIME, "r") as f:
        time_code = f.read()
    with open(STD_CRYPTO, "r") as f:
        crypto_code = f.read()

    test_script = """
print("Testing Time...")
t := time.now()
print(t)

print("Testing Crypto...")
h := crypto.hash("hello")
print(h)

if t > 0 {
    print("Time OK")
} else {
    print("Time FAIL")
}

// SHA256 of "hello"
expected := "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"

if h == expected {
    print("Crypto OK")
} else {
    print("Crypto FAIL: Expected " + expected + ", Got " + h)
}
"""

    full_code = time_code + "\n" + crypto_code + "\n" + test_script

    with open(TEST_ARK, "w") as f:
        f.write(full_code)

    print(f"Running {TEST_ARK}...")
    # meta/ark.py expects an argument before the file path (likely a command like 'run' or just ignored)
    result = subprocess.run([sys.executable, ARK_PATH, "run", TEST_ARK], capture_output=True, text=True)

    print("STDOUT:", result.stdout)
    print("STDERR:", result.stderr)

    if "Time OK" in result.stdout and "Crypto OK" in result.stdout:
        print("VERIFICATION SUCCESS")
        os.remove(TEST_ARK)
        sys.exit(0)
    else:
        print("VERIFICATION FAILED")
        sys.exit(1)

if __name__ == "__main__":
    main()
