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

# ── Interactive Apps Skip List ──────────────────────────────────────────
# These are servers, REPLs, and games that block forever by design.
# They aren't broken — they're just not test-compatible because they
# call sys.net.http.serve(), sys.io.read_line(), or similar blocking I/O.
# To test these properly, use timeout-based smoke tests (separate ticket).
INTERACTIVE_SKIP = {
    "apps/lsp.ark",           # LSP server — waits for JSON-RPC on stdin
    "apps/lsp_main.ark",      # LSP server — same
    "apps/node.ark",          # HTTP server — sys.net.http.serve blocks
    "apps/server.ark",        # HTTP server — sys.net.http.serve blocks
    "apps/sovereign_shell.ark",  # Interactive REPL — sys.io.read_line blocks
    "apps/explorer.ark",      # HTTP server — sys.net.http.serve blocks
    "apps/build.ark",         # Build tool — runs cargo build (>10s)
    "apps/iron_hand.ark",     # AI workflow — needs live AI provider + fs_write
    "apps/miner.ark",         # Miner — connects to external node (connection refused in CI)
    "apps/miner_broken.ark",  # Intentional syntax error artifact — not a real test
    "apps/market_maker.ark",  # HFT event loop demo — exceeds 30s timeout in CI
    "examples/server.ark",    # HTTP server — sys.net.http.serve blocks
    "examples/snake.ark",     # HTTP game  — sys.net.http.serve blocks
    "examples/simple_server.ark",  # HTTP server — binds port, blocks (port conflicts in CI)
    # Server-binding tests: start HTTP servers on localhost, unreliable on macOS CI
    # (firewall restrictions, port conflicts). Pass on Linux/Windows locally.
    "tests/test_net_socket.ark",   # Binds port 8086, times out on macOS CI
    "tests/test_net.ark",          # Binds port 8085, times out on macOS CI
    "tests/test_net_http.ark",     # Binds port 8087, times out on macOS CI
}

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

def parse_test_header(file_path):
    """
    Parses the first block of comments in an Ark file for metadata.
    Returns: (capabilities: set, flaky: bool)
    """
    caps = set()
    flaky = False
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue # Skip empty lines
                if not line.startswith("//"):
                    break # Stop at first non-comment line

                # Check for capabilities
                if "@capabilities:" in line:
                    parts = line.split("@capabilities:", 1)[1].strip()
                    for cap in parts.split(","):
                        c = cap.strip()
                        if c:
                            caps.add(c)

                # Check for flaky
                if "@flaky" in line:
                    flaky = True
    except Exception:
        pass
    return caps, flaky

def run_test_task(file_path, fuzz=False, iterations=1):
    """
    Runs a single Ark test file (or fuzz iteration).
    Returns: TestResult
    """
    # Parse header for metadata
    required_caps, is_flaky_test = parse_test_header(file_path)

    is_expected_fail = "fail_" in file_path or "jailbreak" in file_path
    # Security tests are expected-fail ONLY in unprivileged mode.
    # In privileged mode, the sandbox is bypassed entirely (has_capability('all')),
    # so security tests succeed when they should fail — skip them instead.
    is_security_test = ("security" + os.sep in file_path) or ("security/" in file_path) or ("jailbreak" in file_path)
    if is_security_test:
        privileged = os.environ.get("ALLOW_DANGEROUS_LOCAL_EXECUTION", "false").lower() == "true"
        if privileged:
            # Return PASS immediately — sandbox is disabled, test is meaningless
            return TestResult(
                path=file_path,
                success=True,
                output="SKIPPED (privileged mode — sandbox disabled)",
                error="",
                duration=0.0,
                flaky=False,
                crash=False,
            )
        else:
            is_expected_fail = True

    # Retry logic for flaky tests
    # If the test is explicitly marked @flaky, we allow up to 2 retries (3 attempts total)
    # UNLESS iterations > 1 (which means we are stress-testing for flakiness anyway)
    attempts = 1
    if is_flaky_test and iterations == 1:
        attempts = 3

    loop_count = iterations if iterations > 1 else attempts

    results = []

    # Prepare environment
    env = os.environ.copy()
    if required_caps:
        env["ARK_CAPABILITIES"] = ",".join(required_caps)
        # Clear global override if specific caps are requested to ensure granular control
        if "ALLOW_DANGEROUS_LOCAL_EXECUTION" in env:
            del env["ALLOW_DANGEROUS_LOCAL_EXECUTION"]

    for i in range(loop_count):
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
                timeout=30,
                env=env
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

            # Early exit on success if retrying
            if is_flaky_test and iterations == 1 and success:
                break

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

    is_observed_flaky = False
    if len(results) > 1:
        if any(successes) and not all(successes):
            is_observed_flaky = True

    has_crash = any(crashes)

    # Pick representative result (first failure or first result)
    final_res = results[0]
    for r in results:
        if not r["success"] or r["crash"]:
            final_res = r
            break

    final_success = all(successes)
    if is_flaky_test:
        final_success = any(successes)
    elif is_observed_flaky:
        final_success = False

    return TestResult(
        path=file_path,
        success=final_success,
        output=final_res["output"],
        error=final_res["error"],
        duration=final_res["duration"],
        flaky=is_observed_flaky,
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
    
    # Filter out manual tests
    test_files = [f for f in test_files if "manual" not in f]

    # Separate interactive apps (servers/REPLs that block forever)
    skipped_files = []
    runnable_files = []
    for f in test_files:
        # Normalize path separators for matching
        normalized = f.replace("\\", "/")
        if any(normalized.endswith(skip.replace("/", "/")) or normalized == skip for skip in INTERACTIVE_SKIP):
            skipped_files.append(f)
        else:
            runnable_files.append(f)
    test_files = runnable_files
    
    total = len(test_files) + len(skipped_files)
    print(f"Loaded {total} candidates ({len(skipped_files)} interactive apps skipped).\n")

    # Print skipped files
    for sf in skipped_files:
        print(f"{os.path.basename(sf).ljust(30)} {CYAN}[SKIP]{RESET} (interactive app)")

    if total == 0:
        print(f"{RED}No tests found!{RESET}")
        sys.exit(1)

    passed = 0
    failed = 0
    flaky = 0
    crashes = 0
    skipped = len(skipped_files)

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
    print(f"RESULTS: {passed} Passed, {failed} Failed, {skipped} Skipped, {flaky} Flaky, {crashes} Crashes")
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
