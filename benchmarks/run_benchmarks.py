import subprocess
import time
import json
import statistics
import argparse
import os
import sys
from datetime import datetime

class BenchmarkResult:
    def __init__(self, name, times, iterations):
        self.name = name
        self.times = times
        self.iterations = iterations
        self.mean = statistics.mean(times)
        self.median = statistics.median(times)
        self.stdev = statistics.stdev(times) if len(times) > 1 else 0.0
        self.min_time = min(times)
        self.max_time = max(times)
        self.timestamp = datetime.now().isoformat()

    def to_dict(self):
        return {
            "name": self.name,
            "mean": self.mean,
            "median": self.median,
            "stdev": self.stdev,
            "min": self.min_time,
            "max": self.max_time,
            "iterations": self.iterations,
            "timestamp": self.timestamp
        }

def run_ark(file_path, timeout=60):
    start = time.perf_counter()
    try:
        # Determine root directory (parent of benchmarks/)
        root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        # Ark runner command
        cmd = [sys.executable, "meta/ark.py", "run", file_path]

        env = os.environ.copy()
        env["ALLOW_DANGEROUS_LOCAL_EXECUTION"] = "true"

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=root_dir,
            env=env
        )
        elapsed = time.perf_counter() - start

        if result.returncode != 0:
            print(f"\n[ERROR] Benchmark {file_path} failed with return code {result.returncode}")
            print(f"Stderr: {result.stderr}")
            return None, False

        return elapsed, True
    except subprocess.TimeoutExpired:
        print(f"\n[ERROR] Benchmark {file_path} timed out after {timeout}s")
        return None, False
    except Exception as e:
        print(f"\n[ERROR] Exception running {file_path}: {e}")
        return None, False

def load_history(results_file):
    if os.path.exists(results_file):
        try:
            with open(results_file, "r") as f:
                return json.load(f)
        except:
            return []
    return []

def save_history(results_file, history, new_results):
    # Append new results to history
    # We store a list of runs, where each run is a list of benchmark results
    run_entry = {
        "timestamp": datetime.now().isoformat(),
        "results": [r.to_dict() for r in new_results]
    }
    history.append(run_entry)
    # Keep only last 50 runs to avoid infinite growth
    if len(history) > 50:
        history = history[-50:]

    with open(results_file, "w") as f:
        json.dump(history, f, indent=2)

def main():
    parser = argparse.ArgumentParser(description="Ark Benchmark Suite")
    parser.add_argument("--iterations", type=int, default=5, help="Number of iterations per benchmark")
    parser.add_argument("--compare", action="store_true", help="Compare with previous run")
    parser.add_argument("--format", choices=["table", "json"], default="table", help="Output format")
    parser.add_argument("--filter", type=str, help="Filter benchmarks by name substring")
    args = parser.parse_args()

    bench_dir = os.path.dirname(os.path.abspath(__file__))
    files = [f for f in os.listdir(bench_dir) if f.endswith(".ark") and f.startswith("bench_")]
    files.sort()

    if args.filter:
        files = [f for f in files if args.filter in f]

    results = []

    print(f"Running {len(files)} benchmarks with {args.iterations} iterations each...")
    print("-" * 80)

    for f in files:
        file_path = os.path.join("benchmarks", f)
        times = []
        name = f.replace("bench_", "").replace(".ark", "")

        sys.stdout.write(f"Running {name:<20} ")
        sys.stdout.flush()

        success = True
        for i in range(args.iterations):
            t, ok = run_ark(file_path)
            if not ok:
                success = False
                break
            times.append(t)
            sys.stdout.write(".")
            sys.stdout.flush()

        if success:
            res = BenchmarkResult(name, times, args.iterations)
            results.append(res)
            print(" DONE")
        else:
            print(" FAILED")

    print("-" * 80)

    # History handling
    results_file = os.path.join(bench_dir, "results.json")
    history = load_history(results_file)

    # Comparison Baseline (last successful run)
    baseline = {}
    if history:
        last_run = history[-1]["results"]
        for r in last_run:
            baseline[r["name"]] = r

    # Output
    if args.format == "table":
        print(f"{'BENCHMARK':<20} {'MEAN':<10} {'MEDIAN':<10} {'STDEV':<10} {'MIN':<10} {'MAX':<10} {'STATUS':<15}")
        print("-" * 95)
        for r in results:
            mean_str = f"{r.mean:.4f}s"
            median_str = f"{r.median:.4f}s"
            stdev_str = f"{r.stdev:.4f}s"
            min_str = f"{r.min_time:.4f}s"
            max_str = f"{r.max_time:.4f}s"

            status = "OK"
            if r.name in baseline:
                prev = baseline[r.name]["mean"]
                diff = (r.mean - prev) / prev * 100
                if diff > 10.0:
                    status = f"+{diff:.1f}% REGRESSION"
                elif diff < -10.0:
                    status = f"{diff:.1f}% IMPROVED"
                else:
                    status = f"{diff:+.1f}%"

            print(f"{r.name:<20} {mean_str:<10} {median_str:<10} {stdev_str:<10} {min_str:<10} {max_str:<10} {status:<15}")

    elif args.format == "json":
        print(json.dumps([r.to_dict() for r in results], indent=2))

    save_history(results_file, history, results)

if __name__ == "__main__":
    main()
