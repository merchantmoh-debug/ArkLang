import os
import shutil
import subprocess
import sys

def run_command(cmd, cwd=None):
    print(f"Running: {' '.join(cmd)}")
    subprocess.check_call(cmd, cwd=cwd)

def build_site():
    root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    site_dir = os.path.join(root_dir, 'site')
    wasm_dir = os.path.join(site_dir, 'wasm')
    core_dir = os.path.join(root_dir, 'core')
    apps_dir = os.path.join(root_dir, 'apps')
    meta_dir = os.path.join(root_dir, 'meta')

    print(f"Build Root: {root_dir}")

    # 1. Compile WASM
    print("\n[1/4] Compiling Core to WASM...")
    run_command(['cargo', 'build', '--target', 'wasm32-unknown-unknown', '--release'], cwd=core_dir)

    # 2. Prepare Directories
    print("\n[2/4] Preparing site directories...")
    os.makedirs(wasm_dir, exist_ok=True)

    # 3. Copy Assets
    print("\n[3/4] Copying Assets...")

    # Copy WASM
    src_wasm = os.path.join(root_dir, 'target', 'wasm32-unknown-unknown', 'release', 'ark_0_zheng.wasm')
    dst_wasm = os.path.join(wasm_dir, 'ark.wasm')
    if os.path.exists(src_wasm):
        shutil.copy2(src_wasm, dst_wasm)
        print(f"Copied WASM to {dst_wasm}")
    else:
        print(f"ERROR: WASM artifact not found at {src_wasm}")
        sys.exit(1)

    # 4. Compile Snake Logic
    print("\n[4/4] Compiling Snake Game Logic...")
    snake_src = os.path.join(apps_dir, 'snake_browser.ark')
    snake_json = os.path.join(site_dir, 'snake.json')
    compiler_script = os.path.join(meta_dir, 'ark_to_json.py')

    if not os.path.exists(snake_src):
        print(f"ERROR: Snake source not found at {snake_src}")
        sys.exit(1)

    run_command([sys.executable, compiler_script, snake_src, snake_json])

    # Set Snake as Index
    snake_html = os.path.join(site_dir, 'snake.html')
    index_html = os.path.join(site_dir, 'index.html')
    if os.path.exists(snake_html):
        shutil.copy2(snake_html, index_html)
        print(f"Set {snake_html} as {index_html}")
    else:
        print(f"ERROR: snake.html not found at {snake_html}")
        sys.exit(1)

    print("\nBuild Complete. Ready for Deployment.")

if __name__ == "__main__":
    build_site()
