"""Fire Wave 2: 10 MORE Jules agents on DIFFERENT files (zero overlap with Wave 1)."""
import requests, json, time, os, sys

KEY = "AQ.Ab8RN6KGwjGH6dWIx9d6CgnkN01pqEMz4hVpv_ixkkriy3vvLQ"
BASE = "https://jules.googleapis.com/v1alpha"
SRC = "sources/github/merchantmoh-debug/ark-compiler"
HDR = {"x-goog-api-key": KEY, "Content-Type": "application/json"}

with open(os.path.join(os.path.dirname(__file__), "ARK_AGENT_PAYLOAD.md"), "r") as f:
    PAYLOAD = f.read()

AGENTS = [
    ("AGENT-11: Rust VM Hardening", """You are hardening and completing the Ark Virtual Machine (VM).

TARGET FILE: core/src/vm.rs (ONLY this file — do NOT touch intrinsics.rs or any other file)

S-LANG TRACE: $VMHardening >> $StackMachine !! $ErrorRecovery_OpValidation_GC

MISSION: The Ark VM in core/src/vm.rs is the stack-based execution engine. Your job is to HARDEN it for production use.

Read core/src/vm.rs and perform these improvements:

1. STACK OVERFLOW PROTECTION
   - Add a configurable MAX_STACK_DEPTH constant (default: 10_000)
   - Before every push operation, check if stack.len() >= MAX_STACK_DEPTH
   - If overflow, return a descriptive ArkError::StackOverflow instead of silently growing
   - Add the StackOverflow variant to the error enum if not present

2. INSTRUCTION VALIDATION
   - Before executing any instruction, validate that the operands exist on the stack
   - For binary ops (ADD, SUB, MUL, DIV, MOD): verify stack has >= 2 elements
   - For unary ops (NEG, NOT): verify stack has >= 1 element
   - Return ArkError::StackUnderflow with the instruction name instead of panicking

3. DIVISION BY ZERO GUARD
   - In the DIV and MOD instruction handlers, check if the divisor is zero
   - Return ArkError::DivisionByZero instead of panicking

4. EXECUTION STEP COUNTER
   - Add a step_count: u64 field to the VM struct
   - Increment on every instruction
   - Add a MAX_STEPS: u64 constant (default: 10_000_000 — 10 million)
   - If step_count > MAX_STEPS, return ArkError::ExecutionTimeout

5. INSTRUCTION TRACE MODE
   - Add a trace: bool field to the VM struct (default: false)
   - When trace is true, print each instruction BEFORE executing it (opcode + operands)
   - This enables debugging without a debugger

TESTS (add to bottom of vm.rs or core/tests/test_vm.rs):
- test_stack_overflow_protection: push 10_001 values, verify StackOverflow error
- test_stack_underflow_on_add: execute ADD with empty stack, verify StackUnderflow
- test_division_by_zero: execute DIV with 0 divisor, verify DivisionByZero
- test_execution_timeout: loop that exceeds MAX_STEPS, verify timeout error

`cargo test` MUST pass. Do NOT modify any file other than vm.rs (and optionally a test file)."""),

    ("AGENT-12: Rust Evaluator Completion", """You are completing the Ark tree-walking evaluator.

TARGET FILE: core/src/eval.rs (ONLY this file — do NOT touch intrinsics.rs, vm.rs, or any other file)

S-LANG TRACE: $EvalComplete >> $TreeWalker !! $MatchExpr_ForLoop_ErrorHandling

MISSION: core/src/eval.rs is the tree-walking interpreter/evaluator. Read it and complete any missing AST node evaluation cases.

TASKS:

1. MATCH EXPRESSION EVALUATION
   - If `match` expressions are not fully implemented, complete them
   - Match should evaluate the scrutinee, then compare against each pattern
   - Support literal patterns (integers, strings, booleans), variable binding patterns, and a catch-all `_` wildcard
   - Execute the body of the first matching arm
   - If no arm matches, return ArkValue::Unit

2. FOR-LOOP EVALUATION
   - If `for` loops are stub/incomplete, complete them
   - `for x in list { ... }` should iterate over ArkValue::List elements
   - `for i in range(start, end) { ... }` should iterate over integer range
   - Support `break` and `continue` via special return values or error types

3. ERROR RECOVERY
   - Wrap all AST evaluation in proper error handling
   - Replace any panic!() or unwrap() calls with proper Result returns
   - Add descriptive error messages that include the AST node type and position

4. STRING INTERPOLATION
   - If string interpolation (`"Hello {name}"`) is not evaluated, implement it
   - Parse `{expr}` inside string literals, evaluate the expression, convert to string
   - Nested braces `{{` should produce literal `{`

5. IMPORT STATEMENT
   - If import evaluation is incomplete, implement file-based import
   - `import "path/to/module.ark"` should read the file, parse it, and evaluate in the current scope
   - Prevent circular imports by tracking imported paths in a HashSet

TESTS (add to core/tests/test_eval.rs or bottom of eval.rs):
- test_match_literal_patterns
- test_match_wildcard
- test_for_loop_list
- test_for_loop_break (verify break exits loop)
- test_error_on_undefined_variable

`cargo test` MUST pass."""),

    ("AGENT-13: Rust Type Checker Enhancement", """You are enhancing the Ark type checker.

TARGET FILE: core/src/checker.rs (ONLY this file — do NOT touch any other .rs file)

S-LANG TRACE: $TypeChecker >> $StaticAnalysis !! $InferenceEngine_StructTypes_Warnings

MISSION: core/src/checker.rs performs static type checking on Ark ASTs. Read it and add these capabilities.

TASKS:

1. TYPE INFERENCE FOR VARIABLE DECLARATIONS
   - `x := 42` should infer x as Integer
   - `s := "hello"` should infer s as String
   - `b := true` should infer b as Boolean
   - `lst := [1, 2, 3]` should infer lst as List<Integer>
   - Store inferred types in a symbol table (HashMap<String, ArkType>)

2. FUNCTION SIGNATURE TYPE CHECKING
   - Check that function call arguments match the expected parameter types
   - For known intrinsics, use a hardcoded type signature table
   - Report type mismatches as warnings (not errors — Ark is gradually typed)

3. STRUCT FIELD VALIDATION
   - When accessing struct.field, verify the struct type has that field
   - Report unknown field access as a warning

4. UNUSED VARIABLE DETECTION
   - Track which variables are used after declaration
   - At the end of a scope, report any declared-but-unused variables as warnings
   - Variable names starting with `_` should be exempt (convention for intentionally unused)

5. DEAD CODE DETECTION
   - After a `return` statement in a function body, flag subsequent statements as dead code
   - After a `break` in a loop, flag subsequent statements as dead code
   - Report as informational warnings, not errors

6. RETURN TYPE CONSISTENCY
   - For each function, collect all return expression types
   - If a function returns Integer in one path and String in another, emit a warning
   - Allow functions without explicit return (implicitly return Unit)

TESTS (add at bottom of checker.rs or in core/tests/test_checker.rs):
- test_integer_inference
- test_string_inference
- test_unused_variable_warning
- test_dead_code_after_return
- test_type_mismatch_warning

`cargo test` MUST pass. Output ONLY warnings, NEVER hard errors (preserve Ark's dynamic nature)."""),

    ("AGENT-14: Standard Library Expansion", """You are expanding the Ark Standard Library.

TARGET DIRECTORY: lib/std/ (only NEW files and modifications to EXISTING lib/std/*.ark files)
DO NOT TOUCH: core/src/*, meta/*.py, tests/*.ark, apps/*.ark

S-LANG TRACE: $StdLib >> $ArkModules !! $List_Map_Set_JSON_Assert

MISSION: The Ark standard library in lib/std/ has basic modules. Expand it with 5 new modules and enhance 2 existing ones.

NEW MODULES TO CREATE:

1. lib/std/list.ark — List utilities
```ark
// List manipulation functions
func map(lst, f) {
    result := []
    for item in lst {
        result = list.append(result, f(item))
    }
    return result
}

func filter(lst, predicate) {
    result := []
    for item in lst {
        if predicate(item) {
            result = list.append(result, item)
        }
    }
    return result
}

func reduce(lst, initial, f) {
    acc := initial
    for item in lst {
        acc = f(acc, item)
    }
    return acc
}

func find(lst, predicate) {
    for item in lst {
        if predicate(item) { return item }
    }
    return nil
}

func zip(a, b) { ... }
func flatten(lst) { ... }
func sort(lst) { ... }  // Simple insertion sort
func reverse(lst) { ... }
func unique(lst) { ... }
func chunk(lst, size) { ... }
```

2. lib/std/map.ark — Dictionary/Map utilities
```ark
func keys(m) { ... }
func values(m) { ... }
func entries(m) { ... }
func merge(a, b) { ... }
func has_key(m, key) { ... }
func get_or_default(m, key, default) { ... }
```

3. lib/std/json.ark — JSON convenience layer (wraps sys.json intrinsics)
```ark
func parse(s) { return sys.json.parse(s) }
func stringify(v) { return sys.json.stringify(v) }
func pretty(v) { return sys.json.stringify(v, 2) }  // with indent
```

4. lib/std/assert.ark — Test assertions
```ark
func equal(a, b, msg) {
    if a != b { print("FAIL:", msg, "expected", a, "got", b); sys.exit(1) }
    print("PASS:", msg)
}
func truthy(v, msg) { if !v { print("FAIL:", msg); sys.exit(1) } else { print("PASS:", msg) } }
func falsy(v, msg) { ... }
func throws(f, msg) { ... }  // Test that a function throws
```

5. lib/std/fmt.ark — String formatting
```ark
func pad_left(s, width, char) { ... }
func pad_right(s, width, char) { ... }
func center(s, width, char) { ... }
func truncate(s, max_len) { ... }
func table(rows, headers) { ... }  // ASCII table formatter
```

ENHANCE EXISTING:

6. lib/std/string.ark — Add functions if missing:
   - split(s, delimiter)
   - join(lst, delimiter)
   - trim(s)
   - starts_with(s, prefix)
   - ends_with(s, suffix)
   - replace(s, old, new)
   - to_upper(s), to_lower(s)

7. lib/std/math.ark — Add functions if missing:
   - clamp(value, min, max)
   - lerp(a, b, t)
   - gcd(a, b)
   - lcm(a, b)
   - is_prime(n)
   - factorial(n)

VERIFICATION: Each new file should have a comment block at the top with module description. All function names should match the patterns used in existing lib/std/ files."""),

    ("AGENT-15: CI/CD Pipeline Enhancement", """You are enhancing the Ark CI/CD pipeline.

TARGET FILE: .github/workflows/ci.yml (ONLY this file — and optionally NEW workflow files in .github/workflows/)
DO NOT TOUCH: any source code files, tests, or documentation

S-LANG TRACE: $CIPipeline >> $GitHubActions !! $Matrix_Cache_Artifacts_Security

MISSION: Read .github/workflows/ci.yml and upgrade it to a production-grade CI pipeline.

TASKS:

1. MULTI-OS MATRIX
   - Add a strategy matrix with: ubuntu-latest, windows-latest, macos-latest
   - All test jobs should run on all 3 OS

2. RUST CI JOB
   - Job name: `rust-ci`
   - Steps: checkout, install Rust stable, cargo fmt --check, cargo clippy -- -D warnings, cargo test, cargo build --release
   - Cache: ~/.cargo/registry, ~/.cargo/git, target/ using actions/cache@v4
   - Upload build artifact: target/release/ark-core (or .exe on Windows)

3. PYTHON CI JOB
   - Job name: `python-ci`
   - Steps: checkout, setup Python 3.11+, pip install -r requirements.txt, pip install lark z3-solver
   - Run: python -m pytest meta/ -v
   - Run: python meta/gauntlet.py (with timeout 300s)
   - Cache: pip cache

4. SECURITY SCAN JOB
   - Job name: `security`
   - Run `cargo audit` (install cargo-audit first)
   - Run `pip-audit` for Python dependencies
   - Run `bandit -r meta/` for Python security linting

5. RELEASE JOB (on tag push only)
   - Job name: `release`
   - Trigger: on push tags `v*`
   - Build release binaries for all 3 OS
   - Create GitHub Release with binaries attached
   - Use actions/create-release@v1 and actions/upload-release-asset@v1

6. BADGE GENERATION
   - Add workflow status badge to the top of the workflow file as a comment
   - Ensure the workflow name is descriptive: "Ark Compiler CI"

7. OPTIONAL: Create .github/workflows/nightly.yml
   - Runs on schedule: cron '0 3 * * *' (3 AM UTC daily)
   - Runs the full Gauntlet test suite with ALL capabilities enabled
   - Reports results as a GitHub Actions annotation

VERIFICATION: The workflow YAML must be valid. Use proper indentation (2 spaces). All job names must be unique. All `runs-on` fields must be valid."""),

    ("AGENT-16: Documentation Generator", """You are building an automatic API documentation generator for Ark.

TARGET FILE: meta/docgen.py (NEW file)
DO NOT TOUCH: any existing files — only create NEW files

S-LANG TRACE: $DocGen >> $IntrospectionParser !! $Markdown_HTML_Index

MISSION: Create meta/docgen.py that reads the Ark codebase and generates API documentation.

THE SCRIPT SHOULD:

1. PARSE INTRINSICS FROM meta/ark_intrinsics.py
   - Read the INTRINSICS dictionary at the bottom of ark_intrinsics.py
   - For each intrinsic name (e.g., "sys.json.parse"), extract:
     a) The function name it maps to (e.g., sys_json_parse)
     b) The docstring of that Python function (if any)
     c) The number of parameters (from the function signature)
   - Group intrinsics by category (sys, crypto, math, net, chain, etc.)

2. PARSE STANDARD LIBRARY FROM lib/std/*.ark
   - Read each .ark file in lib/std/
   - Find all `func` declarations using regex: `func\\s+(\\w+)\\s*\\(([^)]*)`
   - Extract function name and parameter list
   - Extract any comment block above the func as documentation

3. GENERATE docs/API_REFERENCE.md
   - Title: "# Ark API Reference"
   - Table of contents with links to each section
   - For each category:
     - Category header (## System, ## Crypto, ## Math, etc.)
     - For each intrinsic:
       - Name, parameter list, description
       - Example usage (generate basic examples from names)
   - Standard Library section with all lib/std/ functions

4. GENERATE docs/STDLIB_REFERENCE.md
   - Dedicated standard library documentation
   - One section per module (string, math, io, crypto, net, etc.)
   - Function signatures, parameter descriptions
   - Usage examples

5. GENERATE docs/QUICK_START.md
   - Installation instructions
   - Hello World example
   - Basic syntax overview (variables, functions, if/else, loops, structs)
   - How to run: `python meta/ark.py run hello.ark`

6. CLI INTERFACE:
```python
if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Ark Documentation Generator")
    parser.add_argument("--output-dir", default="docs", help="Output directory")
    parser.add_argument("--format", choices=["md", "html"], default="md")
    args = parser.parse_args()
```

7. OPTIONAL: HTML OUTPUT
   - If --format html, wrap the markdown in a simple HTML template
   - Use a dark theme with syntax highlighting via highlight.js CDN
   - Generate an index.html with navigation

TESTING: Create tests/test_docgen.py:
- test_parse_intrinsics_finds_all
- test_parse_stdlib_finds_functions
- test_output_file_created

`python meta/docgen.py` must run without errors and produce docs/API_REFERENCE.md."""),

    ("AGENT-17: REPL Enhancement", """You are enhancing the Ark REPL (Read-Eval-Print-Loop).

TARGET FILE: meta/repl.py (ONLY modify this file)
DO NOT TOUCH: ark.py, ark_intrinsics.py, ark_interpreter.py, or any other file

S-LANG TRACE: $REPL >> $InteractiveShell !! $History_Completion_MultiLine

MISSION: Read meta/repl.py and upgrade it into a production-quality interactive REPL.

TASKS:

1. COMMAND HISTORY
   - Store command history in memory (list)
   - Up/Down arrow keys navigate history (if readline is available)
   - Use Python's `readline` module on Unix, or `pyreadline3` fallback on Windows
   - Save history to ~/.ark_history on exit, load on startup
   - Wrap readline import in try/except for graceful degradation

2. TAB COMPLETION
   - Complete Ark keywords: let, func, if, else, while, for, return, import, struct, match, true, false, nil
   - Complete intrinsic names: sys.print, sys.exit, math.sin, crypto.sha256, etc.
   - Complete variable names from the current session scope
   - Register completer with readline.set_completer()

3. MULTI-LINE INPUT
   - Detect incomplete expressions: open braces {, open parens (, trailing backslash \\
   - Switch to continuation prompt "... " until the expression is complete
   - Count open/close braces to determine completeness

4. REPL COMMANDS (prefix with `:`)
   - `:help` — Show available commands
   - `:reset` — Clear the current session (reset all variables)
   - `:load <file>` — Load and execute an .ark file in the current session
   - `:save <file>` — Save current session history to a file
   - `:type <expr>` — Show the type of an expression without executing it
   - `:env` — Show all variables in the current scope
   - `:quit` or `:exit` — Exit the REPL

5. PRETTY OUTPUT
   - Format output values with color (if terminal supports ANSI):
     - Strings in green
     - Numbers in cyan
     - Booleans in yellow
     - Lists/structs in white with indentation
   - Use `\\033[` ANSI codes, wrapped in a helper that checks os.name and TERM

6. ERROR DISPLAY
   - Catch parse/runtime errors and display them with:
     - Error type (ParseError, RuntimeError, TypeError)
     - Line/column number if available
     - Colored red error message
   - Do NOT crash the REPL on errors — print error and continue

7. STARTUP BANNER
```
  ____  ____  _   _
 / _  ||  _ \\| | / /
| |_| || |_) | |/ /
|  _  ||  _ <|   <
| | | || | \\ \\| |\\ \\
|_| |_||_|  \\_\\_| \\_\\  v0.1.0

Type :help for commands, :quit to exit
>>>
```

TESTING: Manually verify:
- Start REPL: `python meta/repl.py`
- Type `1 + 2` -> should print 3
- Type `:help` -> should show commands
- Type `:env` -> should show empty scope
- Type `x := 42` then `:env` -> should show x=42
- Test multi-line: type `func f() {` then `return 1` then `}` -> should work

`python meta/repl.py` must start without errors."""),

    ("AGENT-18: Security Audit Enhancement", """You are enhancing the Ark security audit system.

TARGET FILE: meta/ark_security.py (ONLY this file)
DO NOT TOUCH: ark_intrinsics.py, ark.py, gauntlet.py, or any other file

S-LANG TRACE: $SecurityAudit >> $StaticAnalysis !! $Sanitize_SAST_Report

MISSION: Read meta/ark_security.py and upgrade it into a comprehensive security scanner.

TASKS:

1. STATIC ANALYSIS RULES — Add detection for:
   a) SQL Injection patterns: string concatenation with SQL keywords (SELECT, INSERT, UPDATE, DELETE, DROP)
   b) Command Injection: use of sys.exec, sys.shell, os_command without sanitization
   c) Path Traversal: file operations with ".." in path arguments
   d) Hardcoded Secrets: regex for API keys, passwords, tokens (patterns like "sk-...", "AKIA...", "password = ")
   e) Infinite Loop Risk: while(true) or for loops without break conditions
   f) Unsafe Deserialization: json.parse of untrusted input without validation

2. CAPABILITY AUDIT
   - Scan .ark files and list ALL intrinsic calls used
   - Map each call to its required sandbox capability (thread, net, fs_write, fs_read, crypto)
   - Generate a "capability manifest" showing minimum permissions needed
   - Output format:
```
FILE: apps/server.ark
CAPABILITIES REQUIRED: net, fs_read
INTRINSICS USED: net.http_serve, io.read_file
RISK LEVEL: MEDIUM
```

3. DEPENDENCY AUDIT
   - Scan `import` statements in all .ark files
   - Check if imported modules exist in lib/std/
   - Report missing imports as warnings
   - Check for circular import chains (A imports B imports A)

4. REPORT GENERATION
   - Generate reports in 3 formats:
     a) Console output (colorized, summary)
     b) JSON (machine-readable)
     c) Markdown (docs/SECURITY_REPORT.md)
   - Each finding includes: severity (CRITICAL/HIGH/MEDIUM/LOW/INFO), file, line, description, recommendation

5. CLI INTERFACE:
```python
if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Ark Security Scanner")
    parser.add_argument("path", help="File or directory to scan")
    parser.add_argument("--format", choices=["console", "json", "markdown"], default="console")
    parser.add_argument("--severity", choices=["critical", "high", "medium", "low", "info"], default="low",
                        help="Minimum severity to report")
    parser.add_argument("-o", "--output", help="Output file")
```

6. SECURE DEFAULTS CHECKER
   - Verify that the sandbox is enabled by default in production mode
   - Check that ARK_CAPABILITIES is not set to "*" (wildcard = all capabilities)
   - Warn if security.json has `permissive: true`

TESTING: Create a test file `tests/test_security_scanner.py`:
- test_detects_path_traversal
- test_detects_hardcoded_secret
- test_capability_manifest_generation
- test_json_report_format

`python meta/ark_security.py apps/ --format json` must produce valid JSON output."""),

    ("AGENT-19: Benchmark Suite", """You are building a comprehensive benchmark suite for Ark.

TARGET DIRECTORY: benchmarks/ (ONLY files in this directory)
DO NOT TOUCH: any files outside benchmarks/

S-LANG TRACE: $BenchmarkSuite >> $Performance !! $Fibonacci_Sort_IO_Comparison

MISSION: Create a complete benchmark suite that measures Ark's performance across multiple dimensions.

FILES TO CREATE/MODIFY:

1. benchmarks/run_benchmarks.py (NEW) — Main benchmark runner
```python
import subprocess, time, json, statistics, argparse, os

class BenchmarkResult:
    def __init__(self, name, times, iterations):
        self.name = name
        self.times = times
        self.iterations = iterations
        self.mean = statistics.mean(times)
        self.median = statistics.median(times)
        self.stdev = statistics.stdev(times) if len(times) > 1 else 0
        self.min_time = min(times)
        self.max_time = max(times)

def run_ark(file_path, timeout=30):
    start = time.perf_counter()
    result = subprocess.run(
        ["python", "meta/ark.py", "run", file_path],
        capture_output=True, text=True, timeout=timeout,
        cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    )
    elapsed = time.perf_counter() - start
    return elapsed, result.returncode == 0
```

2. benchmarks/bench_fibonacci.ark — Recursive fibonacci(30)
3. benchmarks/bench_factorial_large.ark — factorial(100) using iterative loop
4. benchmarks/bench_string_ops.ark — 10000 string concatenations
5. benchmarks/bench_list_ops.ark — Create list of 10000 elements, map, filter, reduce
6. benchmarks/bench_struct_ops.ark — Create 1000 structs, access fields
7. benchmarks/bench_math.ark — 10000 arithmetic operations (add, mul, div, mod)
8. benchmarks/bench_crypto.ark — 100 SHA256 hashes
9. benchmarks/bench_json.ark — Parse and stringify 100 JSON objects
10. benchmarks/bench_io.ark — Read/write 100 small files (if fs_write capability)

BENCHMARK RUNNER FEATURES:
- Run each benchmark N times (default: 5)
- Report: mean, median, stdev, min, max execution time
- Compare against previous run (load/save results to benchmarks/results.json)
- Print regression warnings if any benchmark is >10% slower
- CLI: `python benchmarks/run_benchmarks.py --iterations 5 --format table`
- Table output format:
```
BENCHMARK              MEAN      MEDIAN    STDEV     MIN       MAX       STATUS
fibonacci              1.234s    1.230s    0.012s    1.220s    1.250s    OK
string_ops             0.456s    0.450s    0.008s    0.440s    0.470s    +5.2% REGRESSION
```

HISTORICAL TRACKING:
- Save results to benchmarks/results.json with timestamp
- Each run appends to the history array
- `python benchmarks/run_benchmarks.py --compare` shows trend for last 5 runs

VERIFICATION: `python benchmarks/run_benchmarks.py --iterations 1` must complete without errors and produce a results table."""),

    ("AGENT-20: WASM Target Preparation", """You are preparing the Ark Rust core for WebAssembly (WASM) compilation.

TARGET FILE: core/src/wasm.rs (ONLY this file)
DO NOT TOUCH: intrinsics.rs, vm.rs, eval.rs, lib.rs, or any other file

S-LANG TRACE: $WASMTarget >> $NoStdRefactor !! $WasmBindgen_Exports_Memory

MISSION: Read core/src/wasm.rs and complete it as the WASM interface layer for the Ark runtime.

The goal is that `wasm-pack build core/` should produce a .wasm file usable from JavaScript.

TASKS:

1. WASM EXPORTS — Using wasm-bindgen, expose these functions:
```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn ark_eval(source: &str) -> String {
    // Parse and evaluate Ark source code
    // Return the result as a JSON string
    // Catch panics and return error JSON: {"error": "..."}
}

#[wasm_bindgen]
pub fn ark_parse(source: &str) -> String {
    // Parse Ark source code into AST
    // Return the AST as a JSON string
    // On parse error, return {"error": "...", "line": N, "column": N}
}

#[wasm_bindgen]
pub fn ark_check(source: &str) -> String {
    // Run the type checker on source code
    // Return warnings/errors as JSON array
}

#[wasm_bindgen]
pub fn ark_format(source: &str) -> String {
    // Format Ark source code (pretty-print)
    // Return the formatted source
}

#[wasm_bindgen]
pub fn ark_version() -> String {
    "0.1.0".to_string()
}
```

2. PANIC HANDLER
```rust
#[cfg(target_arch = "wasm32")]
use std::panic;

#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(target_arch = "wasm32")]
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}
```

3. MEMORY MANAGEMENT
   - Use wee_alloc or the default allocator
   - Ensure no file I/O, networking, or threading is called in WASM mode
   - Gate unsafe intrinsics behind `#[cfg(not(target_arch = "wasm32"))]`

4. JAVASCRIPT INTEROP TYPES
   - ArkValue -> JsValue serialization
   - Convert ArkValue::Integer -> JsValue::from(i64)
   - Convert ArkValue::String -> JsValue::from_str(s)
   - Convert ArkValue::List -> js_sys::Array
   - Convert ArkValue::Struct -> js_sys::Object

5. ERROR HANDLING
   - All functions must return String (JSON) rather than Result
   - Errors are encoded as {"error": "description"} JSON
   - Never panic in exported functions — always catch and convert to error JSON

6. DOCUMENTATION
   - Add doc comments to all exported functions
   - Include usage examples in comments for JS consumers

OPTIONAL: If wasm_bindgen is not already in Cargo.toml, note that the Cargo.toml should be modified but do NOT modify it yourself (Agent 3 or 4 may be touching it). Instead, create a file `core/wasm_cargo_additions.txt` with the exact dependencies to add.

TESTS (if possible without wasm-pack):
- Unit tests that call ark_eval, ark_parse, ark_version directly (they work in native Rust too)
- test_ark_eval_simple: ark_eval("1 + 2") returns a result containing "3"
- test_ark_parse_error: ark_parse("!!!") returns error JSON
- test_ark_version: ark_version() == "0.1.0"

`cargo test` MUST pass (for native tests)."""),
]

results = []
for i, (title, prompt) in enumerate(AGENTS, 1):
    full_prompt = PAYLOAD + "\n" + prompt
    payload = {
        "prompt": full_prompt,
        "title": title,
        "sourceContext": {
            "source": SRC,
            "githubRepoContext": {"startingBranch": "main"}
        }
    }
    print(f"[{i}/10] {title}...", end=" ", flush=True)
    sys.stdout.flush()
    try:
        r = requests.post(f"{BASE}/sessions", headers=HDR, json=payload, timeout=30)
        if r.status_code == 200:
            sid = r.json().get("name", "?")
            print(f"OK -> {sid}")
            results.append({"title": title, "session": sid, "status": "OK"})
        else:
            print(f"FAIL {r.status_code}: {r.text[:150]}")
            results.append({"title": title, "session": None, "status": f"ERR_{r.status_code}"})
    except Exception as e:
        print(f"FAIL: {e}")
        results.append({"title": title, "session": None, "status": str(e)})
    sys.stdout.flush()
    time.sleep(2)

print(f"\n{'='*60}")
ok = sum(1 for r in results if r["status"] == "OK")
print(f"WAVE 2 DEPLOYED: {ok}/10 agents")
with open("swarm_wave2_log.json", "w") as f:
    json.dump(results, f, indent=2)
print(f"Log: swarm_wave2_log.json")
