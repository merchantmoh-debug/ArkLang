"""
ARK SWARM DISPATCH — Jules CLI Edition
Reads ARK_AGENT_PAYLOAD.md, appends each task prompt, fires via `jules new`.
"""
import subprocess
import os
import sys
import json
import time

REPO = "merchantmoh-debug/ark-compiler"
PAYLOAD_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ARK_AGENT_PAYLOAD.md")
LOG_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "swarm_dispatch_log.json")

with open(PAYLOAD_PATH, "r") as f:
    PAYLOAD = f.read()

TASKS = [
    {
        "id": "AGENT-01-RUST-DATA-OPS",
        "prompt": """You are implementing the DATA OPERATIONS subsystem in the Ark compiler's Rust runtime.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILE: core/src/intrinsics.rs

S-LANG TRACE: $DataOps >> $RustIntrinsics !! $ListStructPow

MISSION BRIEFING:
The Ark Sovereign Computing Stack has a Python reference runtime (meta/ark_intrinsics.py) that implements ~95 intrinsic functions. The Rust runtime in core/src/intrinsics.rs has parity gaps. Your mission is to close the DATA OPERATIONS gap by implementing these 4 intrinsic functions with PRODUCTION-GRADE quality.

DELIVERABLES — implement each with full error handling, type checking, and edge case coverage:

1. `intrinsic_list_pop(args: &[ArkValue]) -> ArkResult`
   - Remove and return the LAST element from an ArkValue::List
   - If list is empty, return ArkValue::Unit (NOT a panic, NOT an unwrap)
   - Mutate the list in-place if possible, or return a new list minus the last element
   - Reference: Python's `sys_list_pop` in meta/ark_intrinsics.py

2. `intrinsic_list_delete(args: &[ArkValue]) -> ArkResult`
   - Takes 2 args: (list, index)
   - Remove element at the given integer index from the list
   - If index is out of bounds, return an error ArkValue (NOT a panic)
   - Reference: Python's `sys_list_delete` in meta/ark_intrinsics.py

3. `intrinsic_struct_has(args: &[ArkValue]) -> ArkResult`
   - Takes 2 args: (struct, field_name_string)
   - Check if the struct (HashMap<String, ArkValue>) contains the given field
   - Return ArkValue::Boolean(true) or ArkValue::Boolean(false)
   - Reference: Python's `sys_struct_has` in meta/ark_intrinsics.py

4. `intrinsic_pow_mod(args: &[ArkValue]) -> ArkResult`
   - Takes 3 args: (base, exponent, modulus) — all integers
   - Compute (base^exponent) % modulus using modular exponentiation
   - Use the binary exponentiation algorithm (square-and-multiply) for efficiency — do NOT use naive repeated multiplication
   - Handle edge cases: exponent=0 returns 1, modulus=1 returns 0
   - Reference: Python's `sys_pow_mod` in meta/ark_intrinsics.py

REGISTRATION:
After implementing all 4 functions, register them in the intrinsic dispatch match block:
- "sys.list.pop" => intrinsic_list_pop
- "sys.list.delete" => intrinsic_list_delete
- "sys.struct.has" => intrinsic_struct_has
- "math.pow_mod" => intrinsic_pow_mod

TESTING — THE RMA MANDATE (Law 7):
Create unit tests in core/src/intrinsics.rs (or core/tests/) for EACH function:
- test_list_pop_normal, test_list_pop_empty
- test_list_delete_valid, test_list_delete_out_of_bounds
- test_struct_has_exists, test_struct_has_missing
- test_pow_mod_basic, test_pow_mod_edge_cases (exp=0, mod=1)

VERIFICATION: `cargo test` must pass with 0 failures.

CONSTRAINT TUNNEL:
- NO unwrap() on user data
- NO panic!() on bad input — return proper error values
- NO placeholder/stub implementations
- NO "todo!()" macros"""
    },
    {
        "id": "AGENT-02-RUST-SYSTEM-IO",
        "prompt": """You are implementing the SYSTEM I/O subsystem in the Ark compiler's Rust runtime.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILE: core/src/intrinsics.rs

S-LANG TRACE: $SystemIO >> $RustIntrinsics !! $SleepReadWriteExtract

MISSION BRIEFING:
The Python reference runtime (meta/ark_intrinsics.py) has I/O intrinsics for sleep, file reading, stdin, stdout, async file ops, and code extraction. Your mission: implement all 6 in Rust with PRODUCTION-GRADE quality.

DELIVERABLES:

1. `intrinsic_time_sleep(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: milliseconds (integer)
   - Use `std::thread::sleep(Duration::from_millis(ms as u64))`
   - Validate: ms must be non-negative. If negative, return error.
   - Return ArkValue::Unit on success

2. `intrinsic_io_read_bytes(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: file_path (string)
   - Read entire file as bytes using `std::fs::read(path)`
   - Return ArkValue::List containing ArkValue::Integer for each byte
   - If file doesn't exist, return error ArkValue with descriptive message
   - SECURITY: Validate path doesn't contain ".." traversal or null bytes

3. `intrinsic_io_read_line(args: &[ArkValue]) -> ArkResult`
   - Takes 0 args
   - Read one line from stdin using `std::io::stdin().read_line()`
   - Trim trailing newline
   - Return ArkValue::String

4. `intrinsic_io_write(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: string to write to stdout
   - Use `std::io::stdout().write_all(s.as_bytes())`
   - Flush stdout after writing
   - Return ArkValue::Unit

5. `intrinsic_io_read_file_async(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: file_path (string)
   - If tokio is available (feature flag), use `tokio::fs::read_to_string`
   - Otherwise, spawn a `std::thread` to read the file and return immediately with a "pending" marker
   - Return ArkValue::String with file contents (blocking fallback is acceptable for MVP)
   - SECURITY: Same path validation as read_bytes

6. `intrinsic_extract_code(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: markdown_string
   - Use regex to extract content between triple-backtick fenced code blocks
   - Return ArkValue::List of ArkValue::String (one per code block found)
   - If no code blocks found, return empty list
   - Add `regex` crate to Cargo.toml if not already present

REGISTRATION in dispatch match:
- "sys.time.sleep" => intrinsic_time_sleep
- "sys.io.read_bytes" => intrinsic_io_read_bytes
- "sys.io.read_line" => intrinsic_io_read_line
- "sys.io.write" => intrinsic_io_write
- "sys.io.read_file_async" => intrinsic_io_read_file_async
- "sys.extract_code" => intrinsic_extract_code

TESTING:
- test_time_sleep_valid, test_time_sleep_negative
- test_io_read_bytes_exists, test_io_read_bytes_missing, test_io_read_bytes_traversal_attack
- test_io_write_basic
- test_extract_code_with_blocks, test_extract_code_empty

VERIFICATION: `cargo test` must pass. `cargo clippy` should be clean."""
    },
    {
        "id": "AGENT-03-RUST-CRYPTO",
        "prompt": """You are implementing the CRYPTOGRAPHY subsystem in the Ark compiler's Rust runtime.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILE: core/src/intrinsics.rs
ALSO MODIFY: core/Cargo.toml (add crypto crate dependencies)

S-LANG TRACE: $CryptoSubsystem >> $RustCrates !! $SHA512_HMAC_PBKDF2_AES_Ed25519

MISSION BRIEFING:
The Python reference runtime uses hashlib, hmac, and cryptography libraries. Your mission: implement ALL 9 crypto intrinsics using real Rust crypto crates. ZERO mocks. Every function must produce output identical to standard cryptographic specifications.

CARGO.TOML DEPENDENCIES — Add these to [dependencies] in core/Cargo.toml:
```toml
sha2 = "0.10"
hmac = "0.12"
pbkdf2 = { version = "0.12", features = ["hmac"] }
aes-gcm = "0.10"
ed25519-dalek = { version = "2.1", features = ["rand_core"] }
rand = "0.8"
hex = "0.4"
```

DELIVERABLES:

1. `intrinsic_crypto_sha512(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: data (string)
   - Compute SHA-512 hash using `sha2::Sha512`
   - Return ArkValue::String containing lowercase hex digest (128 chars)

2. `intrinsic_crypto_hmac_sha512(args: &[ArkValue]) -> ArkResult`
   - Takes 2 args: (key, message) both strings
   - Compute HMAC-SHA512 using `hmac::Hmac<Sha512>`
   - Return ArkValue::String containing lowercase hex digest

3. `intrinsic_crypto_pbkdf2(args: &[ArkValue]) -> ArkResult`
   - Takes 4 args: (password, salt, iterations, key_length) — strings/integers
   - Use PBKDF2-HMAC-SHA512
   - Return ArkValue::String containing hex-encoded derived key

4. `intrinsic_crypto_aes_gcm_encrypt(args: &[ArkValue]) -> ArkResult`
   - Takes 3 args: (plaintext, key_hex_32bytes, nonce_hex_12bytes)
   - Use AES-256-GCM from `aes_gcm` crate
   - Decode hex key (32 bytes) and nonce (12 bytes)
   - Return ArkValue::String containing hex-encoded ciphertext+tag

5. `intrinsic_crypto_aes_gcm_decrypt(args: &[ArkValue]) -> ArkResult`
   - Takes 3 args: (ciphertext_hex, key_hex, nonce_hex)
   - Decrypt and verify authentication tag
   - Return ArkValue::String containing plaintext on success
   - Return error ArkValue if decryption/auth fails (DO NOT PANIC)

6. `intrinsic_crypto_random_bytes(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: count (integer)
   - Generate N cryptographically secure random bytes using `rand::rngs::OsRng`
   - Return ArkValue::String containing hex-encoded bytes

7. `intrinsic_crypto_ed25519_generate(args: &[ArkValue]) -> ArkResult`
   - Takes 0 args
   - Generate Ed25519 keypair using `ed25519_dalek::SigningKey`
   - Return ArkValue::Struct with fields {public_key: hex_string, private_key: hex_string}

8. `intrinsic_crypto_ed25519_sign(args: &[ArkValue]) -> ArkResult`
   - Takes 2 args: (message, private_key_hex)
   - Sign the message bytes with the Ed25519 private key
   - Return ArkValue::String containing hex-encoded 64-byte signature

9. `intrinsic_crypto_ed25519_verify(args: &[ArkValue]) -> ArkResult`
   - Takes 3 args: (message, signature_hex, public_key_hex)
   - Verify the Ed25519 signature
   - Return ArkValue::Boolean(true) if valid, ArkValue::Boolean(false) if invalid
   - NEVER panic on invalid signatures — return false

REGISTRATION:
"crypto.sha512" => intrinsic_crypto_sha512
"crypto.hmac_sha512" => intrinsic_crypto_hmac_sha512
"crypto.pbkdf2" => intrinsic_crypto_pbkdf2
"crypto.aes_gcm_encrypt" => intrinsic_crypto_aes_gcm_encrypt
"crypto.aes_gcm_decrypt" => intrinsic_crypto_aes_gcm_decrypt
"crypto.random_bytes" => intrinsic_crypto_random_bytes
"crypto.ed25519_generate" => intrinsic_crypto_ed25519_generate
"crypto.ed25519_sign" => intrinsic_crypto_ed25519_sign
"crypto.ed25519_verify" => intrinsic_crypto_ed25519_verify

TESTING — write tests that verify:
- SHA512 of "" matches known hash "cf83e1357eef..."
- HMAC-SHA512 with known key/message produces known output
- Ed25519 generate -> sign -> verify round-trip works
- AES-GCM encrypt -> decrypt round-trip recovers plaintext
- AES-GCM decrypt with wrong key returns error (not panic)
- Random bytes returns correct length

VERIFICATION: `cargo test` and `cargo clippy` must pass."""
    },
    {
        "id": "AGENT-04-RUST-NETWORKING",
        "prompt": """You are implementing the NETWORKING subsystem in the Ark compiler's Rust runtime.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILE: core/src/intrinsics.rs
ALSO MODIFY: core/Cargo.toml

S-LANG TRACE: $NetworkStack >> $RustTcpHttp !! $SocketManager_HttpClient

MISSION BRIEFING:
The Python reference runtime has a full TCP socket manager (global dict of socket FDs) and HTTP client/server. Your mission: implement the equivalent in Rust using std::net and ureq.

CARGO.TOML — Add:
```toml
ureq = "2.9"
```

ARCHITECTURE:
Use a global `lazy_static` or `once_cell` Mutex<HashMap<i64, TcpStream>> for socket management.
Each socket gets a monotonically increasing integer ID.
```rust
use std::sync::{Mutex, atomic::{AtomicI64, Ordering}};
use std::collections::HashMap;
use std::net::TcpStream;

static NEXT_SOCKET_ID: AtomicI64 = AtomicI64::new(1);
lazy_static::lazy_static! {
    static ref SOCKET_MAP: Mutex<HashMap<i64, TcpStream>> = Mutex::new(HashMap::new());
}
```

DELIVERABLES:

1. `intrinsic_net_http_request(args: &[ArkValue]) -> ArkResult`
   - Takes 2-3 args: (method_string, url_string, optional_body_string)
   - method = "GET" or "POST"
   - Use `ureq` crate for HTTP requests
   - Return ArkValue::Struct { status: Integer, body: String, headers: String }
   - Handle timeouts (10s default), connection errors gracefully — return error struct, NOT panic

2. `intrinsic_net_http_serve(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: port (integer)
   - Bind TcpListener to 0.0.0.0:port
   - Accept ONE connection, read HTTP request, return the raw request as ArkValue::String
   - This is a blocking call (matches Python behavior)
   - Return error if port in use

3. `intrinsic_net_socket_bind(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: port (integer)
   - Create TcpListener bound to 0.0.0.0:port
   - Store the listener (you may need a separate map for listeners vs streams)
   - Return ArkValue::Integer(socket_id)

4. `intrinsic_net_socket_accept(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: listener_socket_id (integer)
   - Accept a connection on the listener
   - Store the new TcpStream in SOCKET_MAP
   - Return ArkValue::Integer(new_socket_id)

5. `intrinsic_net_socket_connect(args: &[ArkValue]) -> ArkResult`
   - Takes 2 args: (host_string, port_integer)
   - Connect TcpStream to host:port
   - Store in SOCKET_MAP
   - Return ArkValue::Integer(socket_id) on success, error on failure

6. `intrinsic_net_socket_send(args: &[ArkValue]) -> ArkResult`
   - Takes 2 args: (socket_id, data_string)
   - Write data bytes to the TcpStream
   - Return ArkValue::Integer(bytes_sent)

7. `intrinsic_net_socket_recv(args: &[ArkValue]) -> ArkResult`
   - Takes 1-2 args: (socket_id, optional_max_bytes_integer)
   - Read from TcpStream into buffer (default 4096 bytes)
   - Return ArkValue::String

8. `intrinsic_net_socket_close(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: socket_id
   - Remove from SOCKET_MAP (drop triggers TCP close)
   - Return ArkValue::Boolean(true) if found, false if not

9. `intrinsic_net_socket_set_timeout(args: &[ArkValue]) -> ArkResult`
   - Takes 2 args: (socket_id, timeout_ms)
   - Set read and write timeouts on the TcpStream
   - Return ArkValue::Unit

TESTING:
- test_socket_bind_close (bind, verify ID, close)
- test_http_request_invalid_url (should return error, not panic)
- test_socket_close_nonexistent (should return false)

VERIFICATION: `cargo test` must pass."""
    },
    {
        "id": "AGENT-05-RUST-ADVANCED-RUNTIME",
        "prompt": """You are implementing the ADVANCED RUNTIME subsystem in the Ark compiler's Rust runtime.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILE: core/src/intrinsics.rs

S-LANG TRACE: $AdvancedRuntime >> $RustThreadsEvents !! $ThreadSpawn_EventPoll_FuncApply

MISSION BRIEFING:
These are the highest-complexity intrinsics. They involve threading, event queues, higher-order function application, and self-evaluation. Implement with maximum care.

ARCHITECTURE:
```rust
use std::sync::{Mutex, atomic::{AtomicI64, Ordering}};
use std::collections::{HashMap, VecDeque};
use std::thread::{self, JoinHandle};

static NEXT_THREAD_ID: AtomicI64 = AtomicI64::new(1);
lazy_static::lazy_static! {
    static ref THREAD_MAP: Mutex<HashMap<i64, JoinHandle<()>>> = Mutex::new(HashMap::new());
    static ref EVENT_QUEUE: Mutex<VecDeque<ArkValue>> = Mutex::new(VecDeque::new());
}
```

DELIVERABLES:

1. `intrinsic_thread_spawn(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: an ArkValue representing a callable (function reference or closure)
   - Spawn a new OS thread using `std::thread::spawn`
   - The thread should execute the callable. If the ArkValue type system doesn't directly support calling, spawn with a no-op and document the limitation.
   - Store JoinHandle in THREAD_MAP
   - Return ArkValue::Integer(thread_id)
   - CRITICAL: The spawned thread must NOT panic — wrap execution in catch_unwind

2. `intrinsic_thread_join(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: thread_id (integer)
   - Remove JoinHandle from THREAD_MAP and call .join()
   - Return ArkValue::Boolean(true) if join succeeded, error if thread panicked

3. `intrinsic_event_poll(args: &[ArkValue]) -> ArkResult`
   - Takes 0 args
   - Pop the front of EVENT_QUEUE
   - Return the ArkValue if queue is non-empty, ArkValue::Unit if empty
   - This is non-blocking

4. `intrinsic_event_push(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: any ArkValue
   - Push onto the back of EVENT_QUEUE
   - Return ArkValue::Unit

5. `intrinsic_func_apply(args: &[ArkValue]) -> ArkResult`
   - Takes 2 args: (function_value, arguments_list)
   - If the runtime supports calling ArkValue::Function, call it with the provided args
   - If not directly possible (common in compiled runtimes), implement as:
     a) Check if function_value is a String (function name) and look it up in intrinsics
     b) If it's an intrinsic name, call the corresponding intrinsic with the args list
   - Return the result of the function call

6. `intrinsic_vm_eval(args: &[ArkValue]) -> ArkResult`
   - Takes 1 arg: source_code (string)
   - This is the "eval" intrinsic — it should parse and evaluate Ark source code
   - For the Rust runtime, this is HARD. Acceptable MVP implementations:
     a) Shell out to `python meta/ark.py <tempfile>` and capture output
     b) Return an error stating "vm.eval requires the Python runtime"
   - Document clearly which approach you chose and why
   - Return ArkValue::String with the evaluation output

REGISTRATION:
"sys.thread.spawn" => intrinsic_thread_spawn
"sys.thread.join" => intrinsic_thread_join
"sys.event.poll" => intrinsic_event_poll
"sys.event.push" => intrinsic_event_push
"sys.func.apply" => intrinsic_func_apply
"sys.vm.eval" => intrinsic_vm_eval

TESTING:
- test_event_push_poll (push 3 events, poll 3, verify order, poll empty returns Unit)
- test_thread_spawn_join (spawn no-op thread, join, verify success)
- test_func_apply_intrinsic (apply an intrinsic by name)

VERIFICATION: `cargo test` must pass."""
    },
    {
        "id": "AGENT-06-GAUNTLET-SANDBOX-FIX",
        "prompt": """You are fixing the Ark Gauntlet test suite sandbox failures.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILE: meta/gauntlet.py
ALSO EXAMINE: meta/ark_intrinsics.py (sandbox capability system), tests/*.ark

S-LANG TRACE: $GauntletFix >> $SandboxCapabilities !! $AnnotationParser_PerTestEnv

MISSION BRIEFING:
The Ark Gauntlet (meta/gauntlet.py) runs all .ark files as regression tests. Currently 11 tests FAIL because they require sandbox capabilities (thread, net, fs_write) that the Gauntlet doesn't grant. These are NOT bugs in the tests — they are correct programs that need elevated permissions.

CURRENT FAILURES (all sandbox violations):
1. test_async.ark — needs fs_write
2. nts_compliance.ark — needs fs_write
3. test_miner_stratum.ark — needs thread
4. test_net_dispatch.ark — needs thread
5. test_net_p2p.ark — needs thread
6. test_noise.ark — needs thread
7. test_threading.ark — needs thread
8. test_net_socket.ark — needs net
9. debug_write.ark — needs fs_write
10. test_net.ark — needs net
11. test_audio.ark — needs fs_write

SOLUTION — Implement capability annotations:

STEP 1: Define the annotation format. At the top of each .ark test file that needs capabilities, add a comment:
```
// @capabilities: thread,net
```
or
```
// @capabilities: fs_write
```

STEP 2: Add ALL the missing annotations to the 11 test files listed above. Read each file, determine what capabilities it needs (match the sandbox error messages), and add the annotation comment as the FIRST line.

STEP 3: Modify meta/gauntlet.py to:
a) Parse the `// @capabilities:` annotation from each test file before running it
b) Set the `ARK_CAPABILITIES` environment variable for that specific test process
c) The logic should be: if the file has `// @capabilities: X,Y`, set `ARK_CAPABILITIES=X,Y` in the subprocess env
d) If no annotation, run with the default (no capabilities)

STEP 4: For test_net.ark and test_net_http.ark, these may also fail due to network timing issues on Windows. If, after granting the `net` capability, they still fail with WinError 10054, add a `// @flaky` annotation and have gauntlet.py retry flaky tests once before marking as FAIL.

VERIFICATION PROTOCOL:
1. Run `python meta/gauntlet.py`
2. Expected result: 72+ PASS, 0 FAIL (some may SKIP as interactive)
3. ALL previously-failing tests must now either PASS or be marked FLAKY
4. NO existing passing tests may break

CONSTRAINT TUNNEL:
- Do NOT grant capabilities globally (that defeats the sandbox)
- Do NOT disable the sandbox
- Do NOT modify ark_intrinsics.py sandbox logic
- Each test must declare ONLY the minimum capabilities it needs"""
    },
    {
        "id": "AGENT-07-PACKAGE-MANAGER",
        "prompt": """You are building the Ark Package Manager MVP.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET DIRECTORY: meta/pkg/

S-LANG TRACE: $PackageManager >> $CLIRegistryResolver !! $InitInstallListPublish

MISSION BRIEFING:
Ark needs a package manager. The shell script `ark-pkg` in the repo root already calls `python3 -m meta.pkg.cli "$@"`. Your job: make that work.

FILE STRUCTURE TO CREATE:

```
meta/pkg/__init__.py     — Package init, version string
meta/pkg/cli.py          — CLI entry point (argparse-based)
meta/pkg/registry.py     — Package resolution and download logic
meta/pkg/manifest.py     — ark.toml parsing and writing (TOML format)
tests/test_pkg.py        — Unit tests
```

DELIVERABLES:

### 1. `ark-pkg init` (cli.py + manifest.py)
- Create `ark.toml` in the current directory with this structure:
```toml
[package]
name = "<directory_name>"
version = "0.1.0"
description = ""
author = ""

[dependencies]
```
- If ark.toml already exists, print error and exit 1
- Use Python's `tomllib` (3.11+) for reading and manual string building for writing (avoid tomli-w dependency)

### 2. `ark-pkg install <name>` (cli.py + registry.py)
- Resolve package from GitHub: `https://raw.githubusercontent.com/merchantmoh-debug/ark-packages/main/<name>/lib.ark`
- Download the .ark file(s) to `lib/<name>/` directory in the current project
- Add the package to `[dependencies]` in ark.toml: `<name> = "latest"`
- Print success message with installed path
- Handle HTTP errors gracefully (404 = package not found, timeout, etc.)
- Use `urllib.request` (stdlib only, no pip dependencies)

### 3. `ark-pkg list` (cli.py + manifest.py)
- Read ark.toml
- Print all entries in [dependencies] with name and version
- If no dependencies, print "No dependencies installed."

### 4. `ark-pkg publish` (cli.py)
- Create a tarball (<name>-<version>.tar.gz) of the current directory
- Exclude: .git/, __pycache__/, *.pyc, node_modules/
- Print the tarball path on success
- Use `tarfile` module (stdlib)

### 5. `ark-pkg search <query>` (cli.py + registry.py)
- Search for packages matching query in the registry
- For MVP, just check if `https://raw.githubusercontent.com/merchantmoh-debug/ark-packages/main/<query>/lib.ark` returns 200
- Print "Found: <query>" or "Not found: <query>"

CLI ENTRY POINT (meta/pkg/cli.py):
```python
def main():
    parser = argparse.ArgumentParser(prog="ark-pkg", description="Ark Package Manager")
    sub = parser.add_subparsers(dest="command")
    sub.add_parser("init", help="Initialize a new Ark project")
    install_p = sub.add_parser("install", help="Install a package")
    install_p.add_argument("name", help="Package name")
    sub.add_parser("list", help="List installed packages")
    sub.add_parser("publish", help="Package for distribution")
    search_p = sub.add_parser("search", help="Search for packages")
    search_p.add_argument("query", help="Search query")
    args = parser.parse_args()
    # dispatch...

if __name__ == "__main__":
    main()
```

TESTING (tests/test_pkg.py):
- test_init_creates_manifest — run init in tmpdir, verify ark.toml created with correct structure
- test_init_already_exists — verify error on double-init
- test_list_empty — init then list, verify "No dependencies"
- test_install_mock — mock urllib.request.urlopen, verify file downloaded to lib/<name>/
- test_publish_creates_tarball — init, add a file, publish, verify .tar.gz exists

Use `unittest.mock.patch` and `tempfile.TemporaryDirectory` for isolated testing.

VERIFICATION: `python -m pytest tests/test_pkg.py -v` must pass."""
    },
    {
        "id": "AGENT-08-BYTECODE-COMPILER",
        "prompt": """You are building the Ark Bytecode Compilation Target.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILES: meta/bytecode.py (NEW), meta/compile.py (MODIFY), meta/ark.py (MODIFY)

S-LANG TRACE: $BytecodeTarget >> $StackMachine !! $Emit_Disasm_Execute

MISSION BRIEFING:
The Ark compiler currently emits JSON IR. You are adding a `--target bytecode` flag that generates a compact, binary stack-based bytecode format. This is the first step toward a standalone Ark VM.

### BYTECODE SPECIFICATION (ARKB Format v1)

FILE HEADER (8 bytes):
```
Bytes 0-3: "ARKB" (magic number, ASCII)
Byte 4:    0x01 (format version)
Bytes 5-7: 0x00 0x00 0x00 (reserved)
```

CONSTANT POOL (variable length):
```
2 bytes: number of constants (big-endian u16)
For each constant:
  1 byte: type tag (0x01=Integer, 0x02=String, 0x03=Boolean, 0x04=Float)
  For Integer: 8 bytes (big-endian i64)
  For String: 2 bytes length + UTF-8 bytes
  For Boolean: 1 byte (0x00=false, 0x01=true)
  For Float: 8 bytes (IEEE 754 f64)
```

INSTRUCTION SET (1 byte opcode + operands):
```
0x01 PUSH_CONST <u16 index>    — Push constant from pool
0x02 LOAD_VAR <u16 name_idx>   — Load variable (name from const pool)
0x03 STORE_VAR <u16 name_idx>  — Store TOS into variable
0x04 ADD                       — Pop 2, push sum
0x05 SUB                       — Pop 2, push difference
0x06 MUL                       — Pop 2, push product
0x07 DIV                       — Pop 2, push quotient
0x08 CMP_EQ                    — Pop 2, push boolean equal
0x09 CMP_LT                    — Pop 2, push boolean less-than
0x0A CMP_GT                    — Pop 2, push boolean greater-than
0x0B JUMP <u16 offset>         — Unconditional jump
0x0C JUMP_IF_FALSE <u16 offset>— Pop TOS, jump if false
0x0D CALL <u16 name_idx> <u8 argc> — Call function with N args
0x0E RETURN                    — Return TOS from function
0x0F PRINT                     — Pop TOS and print to stdout
0x10 HALT                      — Stop execution
0x11 POP                       — Discard TOS
0x12 DUP                       — Duplicate TOS
0x13 NEG                       — Negate TOS (arithmetic)
0x14 NOT                       — Logical NOT TOS
0x15 MOD                       — Pop 2, push modulus
```

### FILE 1: meta/bytecode.py (NEW)

```python
class BytecodeEmitter:
    def __init__(self):
        self.constants = []      # List of (type_tag, value)
        self.instructions = []   # List of (opcode, *operands)
        self.const_map = {}      # value -> index (dedup)

    def add_constant(self, value) -> int: ...
    def emit(self, opcode: int, *operands): ...
    def compile_ast(self, ast_node): ...  # Walk the AST and emit bytecode
    def to_bytes(self) -> bytes: ...       # Serialize to ARKB format
    def write_file(self, path: str): ...

class BytecodeDisassembler:
    def __init__(self, data: bytes):
        self.data = data
        self.pos = 0

    def disassemble(self) -> str: ...  # Return human-readable disassembly

def main():
    # CLI: python meta/bytecode.py disasm <file.arkb>
    # CLI: python meta/bytecode.py info <file.arkb>  (show header + const pool)
```

### FILE 2: meta/compile.py (MODIFY)
- Add `--target` argument: `json` (default) or `bytecode`
- When target=bytecode: parse source -> AST -> BytecodeEmitter -> write .arkb file
- Output file: input filename with .arkb extension (or use -o flag)

### FILE 3: meta/ark.py (MODIFY)
- Add `compile` subcommand that invokes compile.py
- `python meta/ark.py compile hello.ark --target bytecode -o hello.arkb`

### TESTING:
Create `tests/test_bytecode.py`:
- test_constant_pool_integers
- test_constant_pool_strings
- test_emit_push_add_print (emit a simple "1 + 2" program)
- test_roundtrip_disasm (emit -> serialize -> disassemble -> verify opcodes)
- test_header_magic (verify ARKB header)

Create `tests/hello_bytecode.ark` — a simple program like:
```
x := 1 + 2
print(x)
```
And verify `python meta/ark.py compile tests/hello_bytecode.ark --target bytecode` produces a valid .arkb file that disassembles correctly.

VERIFICATION: `python -m pytest tests/test_bytecode.py -v` must pass."""
    },
    {
        "id": "AGENT-09-PARITY-LEDGER",
        "prompt": """You are updating the Ark Intrinsic Parity Ledger and README.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILES: INTRINSIC_PARITY.md, README.md

S-LANG TRACE: $ParityLedger >> $AuditCount !! $UpdateMarkers_RecalcPercentage

MISSION BRIEFING:
The INTRINSIC_PARITY.md file tracks which Python intrinsics have been implemented in Rust. Several intrinsics have been recently implemented but the ledger hasn't been updated. Your job: audit the actual code and update the ledger to reflect ground truth.

STEP 1: READ THE PYTHON INTRINSICS
- Open meta/ark_intrinsics.py
- Find the INTRINSICS dictionary (near the bottom of the file, ~line 1300+)
- Count ALL registered intrinsic names
- This is your "Total Python Intrinsics" number

STEP 2: READ THE RUST INTRINSICS
- Open core/src/intrinsics.rs
- Find the dispatch match block
- Count ALL registered intrinsic names
- This is your "Total Rust Intrinsics" number
- Note which ones are actual implementations vs stubs/todo!()

STEP 3: UPDATE INTRINSIC_PARITY.md
- For each intrinsic, mark:
  - ✅ if implemented in BOTH Python and Rust with real logic
  - 🟡 if implemented in Python but Rust is a stub/todo!()
  - ❌ if only in Python, not in Rust at all
- Update the summary section at the top:
  - Total Intrinsics: <count>
  - Rust Parity: <count> / <total> (<percentage>%)
  - Remaining Debt: <count>
- Organize by category (IO, Crypto, Math, Net, Chain, Thread, etc.)

STEP 4: MARK THESE AS HAVING PARITY (if they exist in Rust):
These were recently implemented or already existed:
- sys.json.parse ✅
- sys.json.stringify ✅
- sys.exit ✅
- sys.log ✅
- sys.html_escape ✅
- sys.z3.verify ✅

STEP 5: UPDATE README.md
- Find the section about intrinsic parity (or "Status" or "Progress")
- Update the numbers to match the new INTRINSIC_PARITY.md counts
- If there's a badge, update the badge numbers
- Update the Gauntlet test count: "61+ tests passing"

VERIFICATION:
- Open INTRINSIC_PARITY.md and manually verify 3 random intrinsics:
  a) Pick one marked ✅ — verify it exists in BOTH Python and Rust code
  b) Pick one marked 🟡 — verify it's a stub in Rust
  c) Pick one marked ❌ — verify it's absent from Rust
- The total count must match the actual INTRINSICS dict length in ark_intrinsics.py

CONSTRAINT TUNNEL:
- Do NOT guess counts — read the actual source files
- Do NOT mark something as ✅ unless you verified the Rust implementation is real (not a todo!())
- The percentage must be calculated from actual counts, not estimated"""
    },
    {
        "id": "AGENT-10-LSP-SERVER",
        "prompt": """You are completing the Ark Language Server Protocol (LSP) implementation.

REPOSITORY: merchantmoh-debug/ark-compiler
TARGET FILES: apps/lsp_main.ark, apps/lsp.ark, meta/ark_lsp.py (NEW if needed)

S-LANG TRACE: $LSPServer >> $JSONRPC2_StdioTransport !! $Initialize_Completion_Hover_Diagnostics

MISSION BRIEFING:
The Ark LSP server enables IDE integration (autocomplete, hover info, go-to-definition, error diagnostics). The existing implementation in apps/lsp_main.ark and apps/lsp.ark may be incomplete or have stubs. Your job: make it fully functional.

ARCHITECTURE:
The LSP communicates over stdio using JSON-RPC 2.0 protocol. Messages have:
- Header: `Content-Length: <length>\\r\\n\\r\\n`
- Body: JSON-RPC 2.0 message

You may implement this either:
a) In pure Ark (apps/lsp_main.ark) — preferred if the Ark runtime can handle it
b) In Python (meta/ark_lsp.py) — fallback if Ark's I/O capabilities are insufficient
c) Hybrid — Python handles JSON-RPC transport, calls Ark parser for analysis

DELIVERABLES — Implement these LSP methods:

### 1. `initialize` (request)
- Respond with server capabilities:
```json
{
    "capabilities": {
        "textDocumentSync": 1,
        "completionProvider": {"triggerCharacters": ["."]},
        "hoverProvider": true,
        "definitionProvider": true,
        "diagnosticProvider": {"interFileDependencies": false, "workspaceDiagnostics": false}
    },
    "serverInfo": {"name": "ark-lsp", "version": "0.1.0"}
}
```

### 2. `textDocument/didOpen` (notification)
- Store the document URI and content in an in-memory document store (dict)
- Run diagnostics on the opened file
- Send `textDocument/publishDiagnostics` with any parse errors

### 3. `textDocument/didChange` (notification)
- Update the stored document content
- Re-run diagnostics
- Send updated `textDocument/publishDiagnostics`

### 4. `textDocument/completion` (request)
- Return completions based on context:
  a) Ark keywords: let, func, if, else, while, for, return, import, struct, match, true, false
  b) Built-in intrinsics: sys.print, sys.exit, sys.log, math.sin, math.cos, crypto.sha256, etc. (read from known list)
  c) If after a dot (trigger character), complete struct fields or module members
- Each completion item: {label, kind, detail, documentation}

### 5. `textDocument/hover` (request)
- When hovering over a known intrinsic name, return its documentation
- When hovering over a keyword, return its syntax description
- Return {contents: {kind: "markdown", value: "<documentation>"}}

### 6. `textDocument/definition` (request)
- For functions defined in the current file: find the `func <name>` declaration and return its location
- Parse the file, build a symbol table of function definitions with line numbers
- Return {uri, range: {start: {line, character}, end: {line, character}}}

### 7. Diagnostics Engine
- Parse the .ark file using the Ark parser (meta/ark.py or meta/parser.py)
- Capture parse errors with line numbers
- Convert to LSP diagnostic format: {range, severity: 1=Error, message, source: "ark"}
- Send via `textDocument/publishDiagnostics` notification

TRANSPORT LAYER (if creating meta/ark_lsp.py):
```python
import sys
import json

def read_message() -> dict:
    headers = {}
    while True:
        line = sys.stdin.buffer.readline().decode('utf-8').strip()
        if not line:
            break
        key, value = line.split(': ', 1)
        headers[key] = value
    content_length = int(headers['Content-Length'])
    body = sys.stdin.buffer.read(content_length).decode('utf-8')
    return json.loads(body)

def send_message(msg: dict):
    body = json.dumps(msg)
    header = f"Content-Length: {len(body)}\\r\\n\\r\\n"
    sys.stdout.buffer.write(header.encode('utf-8'))
    sys.stdout.buffer.write(body.encode('utf-8'))
    sys.stdout.buffer.flush()
```

TESTING:
Create a test script tests/test_lsp.py that:
1. Starts the LSP server as a subprocess
2. Sends an `initialize` request via stdin
3. Verifies the response contains the expected capabilities
4. Sends a `textDocument/didOpen` with a simple .ark file
5. Sends a `textDocument/completion` request and verifies keywords are returned
6. Sends `shutdown` and `exit` requests

VERIFICATION:
1. `python meta/ark_lsp.py` (or equivalent) starts without errors
2. The test script passes
3. If possible, test with VSCode by adding a launch configuration"""
    },
]

def build_prompt(task_prompt: str) -> str:
    return f"{PAYLOAD}\n{task_prompt}"

def dispatch_agent(task: dict) -> dict:
    agent_id = task["id"]
    full_prompt = build_prompt(task["prompt"])
    
    # Write prompt to temp file (CLI has char limits)
    prompt_file = os.path.join(os.path.dirname(os.path.abspath(__file__)), f".prompt_{agent_id}.txt")
    with open(prompt_file, "w", encoding="utf-8") as f:
        f.write(full_prompt)
    
    try:
        # Use jules new with --repo flag, piping the prompt from file
        result = subprocess.run(
            ["jules", "new", "--repo", REPO, full_prompt],
            capture_output=True, text=True, timeout=30,
            cwd=os.path.dirname(os.path.abspath(__file__))
        )
        
        output = result.stdout.strip() + result.stderr.strip()
        success = result.returncode == 0
        
        return {
            "id": agent_id,
            "success": success,
            "output": output,
            "dispatched_at": __import__("time").strftime("%Y-%m-%dT%H:%M:%SZ", __import__("time").gmtime())
        }
    except subprocess.TimeoutExpired:
        return {"id": agent_id, "success": False, "output": "TIMEOUT", "dispatched_at": ""}
    except Exception as e:
        return {"id": agent_id, "success": False, "output": str(e), "dispatched_at": ""}
    finally:
        if os.path.exists(prompt_file):
            os.remove(prompt_file)

def main():
    print(f"{'='*60}")
    print(f"  ARK SINGULARITY SPRINT — JULES SWARM DISPATCH")
    print(f"  Agents: {len(TASKS)}")
    print(f"  Repo:   {REPO}")
    print(f"  Payload: {len(PAYLOAD)} bytes ({PAYLOAD_PATH})")
    print(f"{'='*60}")
    
    results = []
    for i, task in enumerate(TASKS, 1):
        print(f"\n  [{i}/{len(TASKS)}] Dispatching: {task['id']}...", end=" ", flush=True)
        result = dispatch_agent(task)
        results.append(result)
        
        if result["success"]:
            print(f"✅")
            print(f"           {result['output'][:120]}")
        else:
            print(f"❌")
            print(f"           {result['output'][:200]}")
        
        __import__("time").sleep(2)  # Rate limit between dispatches
    
    # Save results
    with open(LOG_PATH, "w") as f:
        json.dump(results, f, indent=2)
    
    succeeded = sum(1 for r in results if r["success"])
    print(f"\n{'='*60}")
    print(f"  DISPATCH COMPLETE: {succeeded}/{len(results)} agents deployed")
    print(f"  Log: {LOG_PATH}")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
