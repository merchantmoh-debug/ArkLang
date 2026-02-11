import subprocess
import os

print("Running cargo check...")
with open("build_errors.log", "w") as f:
    # Use shell=True to ensure cargo is found in PATH if needed, though shell=False is better if possible.
    # Trying shell=False first.
    try:
        subprocess.run(["cargo", "check"], stdout=f, stderr=subprocess.STDOUT, shell=True)
    except Exception as e:
        f.write(f"Error running cargo: {e}")

print("Done.")
