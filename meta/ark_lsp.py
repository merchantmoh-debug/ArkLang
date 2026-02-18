#!/usr/bin/env python3
import sys
import subprocess
import os

def main():
    # Locate repo root
    # This script is in meta/ark_lsp.py
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(script_dir)

    ark_py = os.path.join(script_dir, "ark.py")
    lsp_main = os.path.join(repo_root, "apps", "lsp_main.ark")

    # Command: python meta/ark.py run apps/lsp_main.ark
    # "run" is the ignored first argument for meta/ark.py
    cmd = [sys.executable, ark_py, "run", lsp_main]

    # Environment: Add repo root to PYTHONPATH
    env = os.environ.copy()
    if "PYTHONPATH" in env:
        env["PYTHONPATH"] = repo_root + os.pathsep + env["PYTHONPATH"]
    else:
        env["PYTHONPATH"] = repo_root

    try:
        # Launch subprocess inheriting stdio
        # This pipes stdin/stdout/stderr directly
        proc = subprocess.Popen(cmd, env=env)
        proc.wait()
        sys.exit(proc.returncode)

    except KeyboardInterrupt:
        pass
    except Exception as e:
        sys.stderr.write(f"Error launching Ark LSP: {e}\n")
        sys.exit(1)

if __name__ == "__main__":
    main()
