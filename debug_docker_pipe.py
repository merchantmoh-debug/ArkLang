import time
import sys
import os

pipes = [
    r'\\.\pipe\docker_engine',
    r'\\.\pipe\dockerDesktopLinuxEngine',
    r'\\.\pipe\docker_cli'
]

print("--- Docker Pipe Diagnostics ---")

for pipe in pipes:
    print(f"Testing: {pipe}")
    try:
        # Try to open the pipe in read/write mode
        # In Python, open() with 'r+b' on a pipe path might allow a check
        with open(pipe, 'r+b') as f:
            print(f"  [SUCCESS] Opened {pipe}")
            # Try to write a simple ping? No, just open is enough proof of access
    except FileNotFoundError:
        print(f"  [FAIL] FileNotFoundError: The pipe does not exist.")
    except PermissionError:
        print(f"  [FAIL] PermissionError: Access is denied (ACL Block).")
    except OSError as e:
        print(f"  [FAIL] OSError code {e.errno}: {e}")
    except Exception as e:
        print(f"  [FAIL] Exception: {type(e).__name__}: {e}")

print("-------------------------------")
