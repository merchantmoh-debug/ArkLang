#!/usr/bin/env python3
import argparse
import sys
import os
import time
import re

# Add repo root to sys.path
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from meta.ark import run_file, ArkValue, eval_binop, UNIT_VALUE, Scope, INTRINSICS, is_truthy, call_user_func
from meta.compile import compile_to_python, resolve_var, call_func, ArkFunction

VERSION = "ARK OMEGA-POINT v112.0"

class Colors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

def cmd_version(args):
    print(f"{Colors.OKCYAN}{VERSION}{Colors.ENDC}")

def cmd_run(args):
    if not os.path.exists(args.file):
        print(f"{Colors.FAIL}Error: File not found: {args.file}{Colors.ENDC}")
        sys.exit(1)

    print(f"{Colors.OKBLUE}[ARK] Executing {args.file}{Colors.ENDC}")
    start_time = time.time()

    if args.jit:
        print(f"{Colors.WARNING}[JIT] Compiling...{Colors.ENDC}")
        try:
            with open(args.file, "r") as f:
                code = f.read()

            code_obj = compile_to_python(code)

            # Prepare Global Scope
            scope = Scope()
            scope.set("sys", ArkValue("sys", "Namespace"))
            scope.set("math", ArkValue("math", "Namespace"))
            scope.set("true", ArkValue(1, "Integer"))
            scope.set("false", ArkValue(0, "Integer"))

            # Inject sys_args
            args_vals = []
            # sys.argv simulation?
            scope.set("sys_args", ArkValue(args_vals, "List"))

            exec_globals = {
                "scope": scope,
                "ArkValue": ArkValue,
                "ArkFunction": ArkFunction,
                "eval_binop": eval_binop,
                "UNIT_VALUE": UNIT_VALUE,
                "INTRINSICS": INTRINSICS,
                "resolve_var": resolve_var,
                "call_func": call_func,
                "is_truthy": is_truthy,
                "call_user_func": call_user_func # needed if fallback happens
            }

            print(f"{Colors.OKGREEN}[JIT] Running...{Colors.ENDC}")
            exec(code_obj, exec_globals)

        except Exception as e:
            print(f"{Colors.FAIL}JIT Error: {e}{Colors.ENDC}")
            import traceback
            traceback.print_exc()
            sys.exit(1)
    else:
        # Interpreter Mode
        try:
            run_file(args.file)
        except Exception as e:
            print(f"{Colors.FAIL}Runtime Error: {e}{Colors.ENDC}")
            sys.exit(1)

    end_time = time.time()
    print(f"{Colors.OKBLUE}[ARK] Finished in {end_time - start_time:.4f}s{Colors.ENDC}")

def cmd_audit(args):
    if not os.path.exists(args.file):
        print(f"{Colors.FAIL}Error: File not found: {args.file}{Colors.ENDC}")
        sys.exit(1)

    print(f"{Colors.OKBLUE}[AUDIT] Scanning {args.file}...{Colors.ENDC}")
    with open(args.file, "r") as f:
        content = f.read()

    issues = []

    # 1. Check for Dangerous Intrinsics
    dangerous = ["sys.exec", "sys.fs.write", "sys.net.socket"]
    for d in dangerous:
        if d in content:
            issues.append(f"Dangerous Intrinsic Usage: {d}")

    # 2. Check for suspicious string literals (simple regex)
    # e.g. "rm -rf"
    if re.search(r"rm\s+-rf", content):
        issues.append("Suspicious Shell Command: 'rm -rf'")

    # 3. Check for loops without sleep (heuristic)
    # "while true" without "sys.time.sleep"
    if "while true" in content and "sys.time.sleep" not in content:
        issues.append("Potential Infinite Loop (CPU Hog): 'while true' without sleep")

    if issues:
        print(f"{Colors.FAIL}Security Issues Found:{Colors.ENDC}")
        for issue in issues:
            print(f" - {issue}")
        sys.exit(1)
    else:
        print(f"{Colors.OKGREEN}No obvious security issues found.{Colors.ENDC}")

def main():
    parser = argparse.ArgumentParser(description="Ark Sovereign System CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Version
    parser_version = subparsers.add_parser("version", help="Show version")
    parser_version.set_defaults(func=cmd_version)

    # Run
    parser_run = subparsers.add_parser("run", help="Run an Ark script")
    parser_run.add_argument("file", help="Path to Ark file")
    parser_run.add_argument("--jit", action="store_true", help="Enable JIT Compiler (Experimental)")
    parser_run.add_argument("script_args", nargs=argparse.REMAINDER, help="Arguments for the script")
    parser_run.set_defaults(func=cmd_run)

    # Audit
    parser_audit = subparsers.add_parser("audit", help="Audit an Ark script for security")
    parser_audit.add_argument("file", help="Path to Ark file")
    parser_audit.set_defaults(func=cmd_audit)

    args = parser.parse_args()
    args.func(args)

if __name__ == "__main__":
    main()
