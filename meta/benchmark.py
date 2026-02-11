
import subprocess
import time
import os
import sys

def run_benchmark(runtime_cmd, label):
    print(f"\n--- Running {label} ---")
    start_time = time.time()
    try:
        # Capture output to avoid spamming, but print if error
        result = subprocess.run(
            runtime_cmd, 
            capture_output=True, 
            text=True, 
            check=True
        )
        end_time = time.time()
        duration = end_time - start_time
        print(result.stdout)
        print(f"Time: {duration:.4f} seconds")
        return duration
    except subprocess.CalledProcessError as e:
        print(f"Error: {e}")
        print(f"Stderr: {e.stderr}")
        return None

def main():
    ark_file = "benchmarks/benchmark_suite.ark"
    
    if not os.path.exists(ark_file):
        print(f"Error: {ark_file} not found.")
        return

    # 1. Compile (Python Compiler) -> benchmark.json
    print("Compiling...")
    subprocess.run(
        ["python", "meta/compile.py", ark_file, "benchmark.json"],
        check=True
    )

    # 2. Run Python Runtime (Reference)
    # python meta/ark.py benchmark.json
    py_time = run_benchmark(
        ["python", "meta/ark.py", "benchmark.json"], 
        "Python Reference Runtime"
    )

    # 3. Run Rust VM (Optimized)
    # cargo run --bin ark_loader -- benchmark.json
    # Build release first for fairness?
    print("\nBuilding Rust Optimized Release...")
    subprocess.run(
        ["cargo", "build", "--release", "--bin", "ark_loader"],
        cwd="core",
        check=True
    )
    
    loader_path = "core/target/release/ark_loader.exe"
    if not os.path.exists(loader_path):
        # Fallback to absolute path or just cargo run if path issue
        rust_cmd = ["cargo", "run", "--release", "--bin", "ark_loader", "--", "../benchmark.json"]
        cwd = "core"
    else:
        rust_cmd = [loader_path, "benchmark.json"]
        cwd = None # Run from root if using exe path relative to root? No, check path.

    # Actually simpler to just use cargo run from root with manifest-path
    rust_cmd = ["cargo", "run", "--release", "--manifest-path", "core/Cargo.toml", "--bin", "ark_loader", "--", "benchmark.json"]
    
    rust_time = run_benchmark(
        rust_cmd,
        "Rust Bytecode VM"
    )

    if py_time and rust_time:
        speedup = py_time / rust_time
        print(f"\n--- RESULTS ---")
        print(f"Python: {py_time:.4f}s")
        print(f"Rust:   {rust_time:.4f}s")
        print(f"Speedup: {speedup:.2f}x")

if __name__ == "__main__":
    main()
