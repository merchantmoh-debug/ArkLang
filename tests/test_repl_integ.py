import subprocess
import sys
import os
import re

def strip_ansi(text):
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    return ansi_escape.sub('', text)

def run_repl_test():
    # Prepare input with explicit newlines for multi-line test
    input_str = """
func g(x) {
  return x * 2
}
g(21)
:quit
"""
    # Run REPL process
    process = subprocess.Popen(
        [sys.executable, "meta/repl.py"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd=os.getcwd()
    )

    stdout, stderr = process.communicate(input=input_str)

    clean_stdout = strip_ansi(stdout)

    print("STDOUT (Cleaned):", clean_stdout)

    # Check for expected outputs
    assert "=> 42" in clean_stdout, "Multi-line function definition failed"
    # Check if prompt changed to "..."
    if "... " in clean_stdout:
         print("Multi-line prompt detected.")
    else:
         print("Warning: Multi-line prompt not detected (might be stripped or not captured)")

    print("REPL Multi-line Test Passed!")

if __name__ == "__main__":
    run_repl_test()
