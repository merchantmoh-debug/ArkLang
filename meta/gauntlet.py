import os
import sys
import glob
import time
import subprocess
import argparse
import random
import string
import concurrent.futures
from dataclasses import dataclass
from typing import List, Optional

# ANSI Colors
RED = "\033[91m"
GREEN = "\033[92m"
YELLOW = "\033[93m"
BLUE = "\033[94m"
MAGENTA = "\033[95m"
CYAN = "\033[96m"
RESET = "\033[0m"

@dataclass
class TestResult:
    path: str
    success: bool
    output: str
    error: str
    duration: float
    flaky: bool = False
    crash: bool = False

# Fuzz Helper
def generate_fuzz_input(size=1024):
    return os.urandom(size)

def run_test_task(file_path, fuzz=False, iterations=1):
    """
    Runs a single Ark test file (or fuzz iteration).
    Returns: TestResult
    """
    is_expected_fail = "fail_" in file_path or "jailbreak" in file_path

    results = []

    for _ in range(iterations):
        cmd = [sys.executable, "meta/ark.py", "run", file_path]
        
        input_data = b"!exit\n"
        if fuzz:
            # Generate random size between 100 and 5000 bytes
            input_data = generate_fuzz_input(random.randint(100, 5000))
        
        start = time.time()
        try:
            # Run the process
            proc = subprocess.run(
                cmd,
                input=input_data,
                capture_output=True,
                text=False, # Use bytes mode for input/output
                timeout=10,
                env=os.environ # Propagate ALLOW_DANGEROUS_LOCAL_EXECUTION
            )
            duration = time.time() - start

            crash = False
            success = False

            if proc.returncode != 0:
                if is_expected_fail:
                    success = True
                else:
                    if proc.returncode < 0: # Signal (e.g. SIGSEGV)
                        crash = True
                    success = False
            else:
                success = not is_expected_fail # Failed to fail if expected

            # Decode output safely for reporting
            stdout_text = proc.stdout.decode('utf-8', errors='replace')
            stderr_text = proc.stderr.decode('utf-8', errors='replace')

            results.append({
                "success": success,
                "output": stdout_text,
                "error": stderr_text,
                "duration": duration,
                "crash": crash
            })

        except subprocess.TimeoutExpired:
            results.append({
                "success": False,
                "output": "",
                "error": "TIMEOUT",
                "duration": time.time() - start,
                "crash": False
            })

    # Aggregate
    successes = [r["success"] for r in results]
    crashes = [r["crash"] for r in results]

    is_flaky = False
    if len(results) > 1:
        if any(successes) and not all(successes):
            is_flaky = True

    has_crash = any(crashes)

    # Pick representative result (first failure or first result)
    final_res = results[0]
    for r in results:
        if not r["success"] or r["crash"]:
            final_res = r
            break

    return TestResult(
        path=file_path,
        success=all(successes) and not is_flaky, # Strict success
        output=final_res["output"],
        error=final_res["error"],
        duration=final_res["duration"],
        flaky=is_flaky,
        crash=has_crash
    )

def main():
    parser = argparse.ArgumentParser(description="The Gauntlet: Ark Regression & Fuzzing Engine")
    parser.add_argument("--fuzz", action="store_true", help="Enable Fuzzing Mode (random byte-noise inputs)")
    parser.add_argument("--workers", type=int, default=os.cpu_count() or 4, help="Number of parallel workers")
    parser.add_argument("--iterations", type=int, default=1, help="Runs per test (to detect flakiness)")
    parser.add_argument("--privileged", action="store_true", help="Enable ALLOW_DANGEROUS_LOCAL_EXECUTION (for net/thread tests)")
    args = parser.parse_args()

    # Set Environment Variable if Privileged
    if args.privileged:
        os.environ["ALLOW_DANGEROUS_LOCAL_EXECUTION"] = "true"

    print(f"{YELLOW}=========================================={RESET}")
    print(f"{YELLOW}   THE GAUNTLET: ARK REGRESSION SUITE     {RESET}")
    print(f"{YELLOW}   Mode: {'FUZZING (BYTES)' if args.fuzz else 'STANDARD'}{RESET}")
    print(f"{YELLOW}   Privileged: {'YES' if args.privileged else 'NO'}{RESET}")
    print(f"{YELLOW}   Workers: {args.workers}{RESET}")
    print(f"{YELLOW}   Iterations: {args.iterations}{RESET}")
    print(f"{YELLOW}=========================================={RESET}")

    # 1. Discovery
    test_files = glob.glob("tests/**/*.ark", recursive=True) + \
                 glob.glob("apps/**/*.ark", recursive=True) + \
                 glob.glob("benchmarks/**/*.ark", recursive=True) + \
                 glob.glob("examples/**/*.ark", recursive=True)
    
    # Filter out specific files if needed
    test_files = [f for f in test_files if "manual" not in f]
    
    total = len(test_files)
    print(f"Loaded {total} candidates.\n")

    if total == 0:
        print(f"{RED}No tests found!{RESET}")
        sys.exit(1)

    passed = 0
    failed = 0
    flaky = 0
    crashes = 0

    start_time = time.time()

    # Parallel Execution
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.workers) as executor:
        # Map file paths to futures
        future_to_file = {
            executor.submit(run_test_task, f, args.fuzz, args.iterations): f
            for f in test_files
        }
        
        for future in concurrent.futures.as_completed(future_to_file):
            f = future_to_file[future]
            test_name = os.path.basename(f)

            try:
                result = future.result()

                # Report
                status_color = GREEN if result.success else RED
                status_text = "PASS" if result.success else "FAIL"

                if result.flaky:
                    status_text = "FLAKY"
                    status_color = MAGENTA
                    flaky += 1
                elif result.crash:
                    status_text = "CRASH"
                    status_color = RED
                    crashes += 1
                    failed += 1
                elif result.success:
                    passed += 1
                else:
                    failed += 1

                print(f"{test_name.ljust(30)} {status_color}[{status_text}]{RESET} ({result.duration:.2f}s)")

                if not result.success or result.crash or result.flaky:
                    if result.crash:
                        print(f"  {RED}>> SYSTEM CRASH (Signal/Segfault){RESET}")
                    if result.flaky:
                         print(f"  {MAGENTA}>> FLAKY BEHAVIOR DETECTED{RESET}")
                    if result.error:
                        lines = result.error.splitlines()
                        for line in lines[-3:]:
                            print(f"  {RED}>> {line}{RESET}")

            except Exception as exc:
                print(f"{test_name.ljust(30)} {RED}[ERROR]{RESET} {exc}")
                failed += 1

    total_duration = time.time() - start_time
    print(f"\n{YELLOW}=========================================={RESET}")
    print(f"RESULTS: {passed} Passed, {failed} Failed, {flaky} Flaky, {crashes} Crashes")
    print(f"Total Time: {total_duration:.2f}s")
    
    if failed == 0 and crashes == 0:
        if flaky > 0:
            print(f"{MAGENTA}WARNING: SYSTEM UNSTABLE (FLAKY TESTS DETECTED).{RESET}")
            sys.exit(0)
        else:
            print(f"{GREEN}ALL SYSTEMS GREEN. RELEASE CANDIDATE VERIFIED.{RESET}")
            sys.exit(0)
    else:
        print(f"{RED}CRITICAL SYSTEM FAILURE. DO NOT RELEASE.{RESET}")
        sys.exit(1)

if __name__ == "__main__":
    main()
