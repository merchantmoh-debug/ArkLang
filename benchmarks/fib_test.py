import time
import sys
import os
import subprocess

# Create fib.ark
with open("fib.ark", "w") as f:
    f.write("""
func fib(n) {
    if n < 2 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}
print(fib(25))
""")

start = time.time()
# Run via subprocess to avoid import pollution and sys.argv hacks
# Assuming meta/ark.py is in cwd/meta/ark.py
res = subprocess.run(["python3", "meta/ark.py", "run", "fib.ark"], capture_output=True)
end = time.time()

if res.returncode != 0:
    print(f"Error running fib.ark: {res.stderr.decode()}")
else:
    print(f"Output: {res.stdout.decode().strip()}")
    print(f"Time taken: {end - start:.4f}s")
