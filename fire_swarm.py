"""Fire all 10 Jules agents via REST API."""
import requests, json, time, os

KEY = "AQ.Ab8RN6KGwjGH6dWIx9d6CgnkN01pqEMz4hVpv_ixkkriy3vvLQ"
BASE = "https://jules.googleapis.com/v1alpha"
SRC = "sources/github/merchantmoh-debug/ark-compiler"
HDR = {"x-goog-api-key": KEY, "Content-Type": "application/json"}

# Load ARK payload
with open(os.path.join(os.path.dirname(__file__), "ARK_AGENT_PAYLOAD.md"), "r") as f:
    PAYLOAD = f.read()

AGENTS = [
    ("AGENT-01: Rust Data Ops", """You are implementing the DATA OPERATIONS subsystem in the Ark compiler's Rust runtime.

TARGET FILE: core/src/intrinsics.rs

S-LANG TRACE: $DataOps >> $RustIntrinsics !! $ListStructPow

MISSION: Close the DATA OPERATIONS gap by implementing these 4 intrinsic functions with PRODUCTION-GRADE quality. Reference meta/ark_intrinsics.py for exact behavior.

DELIVERABLES:

1. `intrinsic_list_pop(args: &[ArkValue]) -> ArkResult` — Remove and return LAST element from ArkValue::List. If empty, return ArkValue::Unit (NOT panic).

2. `intrinsic_list_delete(args: &[ArkValue]) -> ArkResult` — Takes (list, index). Remove element at index. Out of bounds = error (NOT panic).

3. `intrinsic_struct_has(args: &[ArkValue]) -> ArkResult` — Takes (struct, field_name). Return ArkValue::Boolean.

4. `intrinsic_pow_mod(args: &[ArkValue]) -> ArkResult` — Takes (base, exp, mod). Binary exponentiation (square-and-multiply). Edge: exp=0->1, mod=1->0.

REGISTRATION in dispatch match: "sys.list.pop", "sys.list.delete", "sys.struct.has", "math.pow_mod"

TESTS: test_list_pop_normal/empty, test_list_delete_valid/oob, test_struct_has_exists/missing, test_pow_mod_basic/edges.

CONSTRAINTS: NO unwrap() on user data. NO panic!(). NO todo!(). NO stub implementations. `cargo test` MUST pass."""),

    ("AGENT-02: Rust System I/O", """You are implementing the SYSTEM I/O subsystem in the Ark compiler's Rust runtime.

TARGET FILE: core/src/intrinsics.rs

S-LANG TRACE: $SystemIO >> $RustIntrinsics !! $SleepReadWriteExtract

DELIVERABLES:

1. `intrinsic_time_sleep(args)` — Sleep for N milliseconds via std::thread::sleep. Validate non-negative.

2. `intrinsic_io_read_bytes(args)` — Read file as Vec<u8>, return List of Integers. SECURITY: reject path traversal ("..") and null bytes.

3. `intrinsic_io_read_line(args)` — Read one stdin line, trim newline, return String.

4. `intrinsic_io_write(args)` — Write string to stdout, flush. Return Unit.

5. `intrinsic_io_read_file_async(args)` — Read file in thread. Blocking fallback acceptable for MVP.

6. `intrinsic_extract_code(args)` — Extract fenced code blocks from markdown using regex. Add `regex` to Cargo.toml.

REGISTRATION: "sys.time.sleep", "sys.io.read_bytes", "sys.io.read_line", "sys.io.write", "sys.io.read_file_async", "sys.extract_code"

TESTS: sleep valid/negative, read_bytes exists/missing/traversal, write basic, extract_code with blocks/empty.

`cargo test` and `cargo clippy` MUST pass."""),

    ("AGENT-03: Rust Cryptography", """You are implementing the CRYPTOGRAPHY subsystem in the Ark compiler's Rust runtime.

TARGET: core/src/intrinsics.rs + core/Cargo.toml

S-LANG TRACE: $CryptoSubsystem >> $RustCrates !! $SHA512_HMAC_PBKDF2_AES_Ed25519

Add to Cargo.toml: sha2="0.10", hmac="0.12", pbkdf2={version="0.12",features=["hmac"]}, aes-gcm="0.10", ed25519-dalek={version="2.1",features=["rand_core"]}, rand="0.8", hex="0.4"

DELIVER ALL 9 crypto intrinsics with REAL crypto (ZERO mocks):
1. crypto_sha512 — SHA-512 hex digest
2. crypto_hmac_sha512 — HMAC-SHA512 hex
3. crypto_pbkdf2 — PBKDF2-HMAC-SHA512 hex
4. crypto_aes_gcm_encrypt — AES-256-GCM encrypt, return hex
5. crypto_aes_gcm_decrypt — Decrypt+verify tag, error on failure (NOT panic)
6. crypto_random_bytes — N secure random bytes, hex
7. crypto_ed25519_generate — Keypair struct {public_key, private_key}
8. crypto_ed25519_sign — Sign message, return hex signature
9. crypto_ed25519_verify — Verify signature, return boolean (NEVER panic on invalid)

TESTS: SHA512 of "" matches known hash, HMAC with known vectors, Ed25519 generate->sign->verify roundtrip, AES-GCM encrypt->decrypt roundtrip, wrong key returns error.

`cargo test` MUST pass."""),

    ("AGENT-04: Rust Networking", """You are implementing the NETWORKING subsystem in the Ark compiler's Rust runtime.

TARGET: core/src/intrinsics.rs + Cargo.toml (add ureq="2.9")

S-LANG TRACE: $NetworkStack >> $RustTcpHttp !! $SocketManager_HttpClient

ARCHITECTURE: Global Mutex<HashMap<i64, TcpStream>> for socket management. AtomicI64 for monotonic IDs.

DELIVER 9 networking intrinsics:
1. net_http_request(method, url, body?) — GET/POST via ureq. Return struct {status, body, headers}.
2. net_http_serve(port) — Bind TcpListener, accept ONE connection, return raw request.
3. net_socket_bind(port) — Bind 0.0.0.0:port, return socket_id.
4. net_socket_accept(listener_id) — Accept connection, return new socket_id.
5. net_socket_connect(host, port) — Connect TcpStream, return socket_id.
6. net_socket_send(socket_id, data) — Write bytes, return bytes_sent.
7. net_socket_recv(socket_id, max_bytes?) — Read from stream, return String.
8. net_socket_close(socket_id) — Remove+drop, return Boolean.
9. net_socket_set_timeout(socket_id, timeout_ms) — Set read/write timeouts.

TESTS: socket_bind_close, http_request_invalid_url (error not panic), close_nonexistent (returns false).

`cargo test` MUST pass."""),

    ("AGENT-05: Rust Advanced Runtime", """You are implementing the ADVANCED RUNTIME subsystem in the Ark compiler's Rust runtime.

TARGET: core/src/intrinsics.rs

S-LANG TRACE: $AdvancedRuntime >> $RustThreadsEvents !! $ThreadSpawn_EventPoll_FuncApply

ARCHITECTURE: Global Mutex<HashMap<i64, JoinHandle<()>>> for threads. Global Mutex<VecDeque<ArkValue>> for events.

DELIVER 6 intrinsics:
1. thread_spawn(callable) — Spawn OS thread via std::thread::spawn. Wrap in catch_unwind. Store JoinHandle. Return thread_id.
2. thread_join(thread_id) — Join thread. Return Boolean(true) on success.
3. event_poll() — Pop front of event queue. Return ArkValue or Unit if empty. Non-blocking.
4. event_push(value) — Push onto event queue. Return Unit.
5. func_apply(function_value, args_list) — If function is an intrinsic name string, call the intrinsic with args. Return result.
6. vm_eval(source_code) — Parse and eval Ark source. MVP: shell out to `python meta/ark.py <tempfile>`. Return String.

REGISTRATION: "sys.thread.spawn", "sys.thread.join", "sys.event.poll", "sys.event.push", "sys.func.apply", "sys.vm.eval"

TESTS: event push/poll ordering, thread_spawn_join, func_apply intrinsic by name.

`cargo test` MUST pass."""),

    ("AGENT-06: Gauntlet Sandbox Fix", """You are fixing the Ark Gauntlet test suite sandbox failures.

TARGET: meta/gauntlet.py + tests/*.ark (add annotations)

S-LANG TRACE: $GauntletFix >> $SandboxCapabilities !! $AnnotationParser_PerTestEnv

11 tests FAIL due to sandbox capability violations:
- fs_write: test_async.ark, nts_compliance.ark, debug_write.ark, test_audio.ark
- thread: test_miner_stratum.ark, test_net_dispatch.ark, test_net_p2p.ark, test_noise.ark, test_threading.ark
- net: test_net_socket.ark, test_net.ark

SOLUTION:
1. Add `// @capabilities: <comma-separated>` as FIRST LINE of each failing test
2. Modify gauntlet.py to parse this annotation and set ARK_CAPABILITIES env var per-test subprocess
3. Add `// @flaky` annotation support — retry flaky tests once before FAIL
4. Mark test_net.ark and test_net_http.ark as flaky (WinError 10054)

VERIFICATION: `python meta/gauntlet.py` must show 72+ PASS, 0 FAIL (only SKIPs for interactive apps).

CONSTRAINTS: Do NOT grant caps globally. Do NOT disable sandbox. Do NOT modify ark_intrinsics.py. Each test declares MINIMUM required caps only."""),

    ("AGENT-07: Package Manager MVP", """You are building the Ark Package Manager MVP.

TARGET: meta/pkg/__init__.py, meta/pkg/cli.py, meta/pkg/registry.py, meta/pkg/manifest.py, tests/test_pkg.py

S-LANG TRACE: $PackageManager >> $CLIRegistryResolver !! $InitInstallListPublish

The shell script `ark-pkg` in repo root calls `python3 -m meta.pkg.cli "$@"`.

DELIVER 5 commands:
1. `ark-pkg init` — Create ark.toml with [package] name/version/description and [dependencies]
2. `ark-pkg install <name>` — Download from github.com/merchantmoh-debug/ark-packages/<name>/lib.ark to lib/<name>/. Update ark.toml. Use urllib (stdlib only).
3. `ark-pkg list` — Print [dependencies] from ark.toml
4. `ark-pkg publish` — Create <name>-<version>.tar.gz excluding .git/__pycache__ etc. Use tarfile module.
5. `ark-pkg search <query>` — Check if package exists at registry URL

CLI via argparse. Use tomllib (3.11+) for reading, manual string building for writing.

TESTS (tests/test_pkg.py): test_init_creates_manifest, test_init_already_exists, test_list_empty, test_install_mock (mock urlopen), test_publish_creates_tarball. Use unittest.mock.patch and tempfile.

`python -m pytest tests/test_pkg.py -v` MUST pass."""),

    ("AGENT-08: Bytecode Compiler", """You are building the Ark Bytecode Compilation Target.

TARGET: meta/bytecode.py (NEW), meta/compile.py (MODIFY), meta/ark.py (MODIFY)

S-LANG TRACE: $BytecodeTarget >> $StackMachine !! $Emit_Disasm_Execute

ARKB FORMAT v1:
- Header: "ARKB" + version 0x01 + 3 reserved bytes
- Constant pool: u16 count, then type-tagged values (0x01=Int/8bytes, 0x02=String/len+UTF8, 0x03=Bool/1byte, 0x04=Float/8bytes)
- Instructions: 1-byte opcode + operands. Opcodes: PUSH_CONST(0x01,u16), LOAD_VAR(0x02,u16), STORE_VAR(0x03,u16), ADD(0x04), SUB(0x05), MUL(0x06), DIV(0x07), CMP_EQ(0x08), CMP_LT(0x09), CMP_GT(0x0A), JUMP(0x0B,u16), JUMP_IF_FALSE(0x0C,u16), CALL(0x0D,u16,u8), RETURN(0x0E), PRINT(0x0F), HALT(0x10), POP(0x11), DUP(0x12), NEG(0x13), NOT(0x14), MOD(0x15)

meta/bytecode.py: BytecodeEmitter class (compile_ast, to_bytes, write_file) + BytecodeDisassembler class (disassemble -> human-readable)
meta/compile.py: Add --target bytecode flag
meta/ark.py: Add compile subcommand

TESTS (tests/test_bytecode.py): constant pool, emit push+add+print, roundtrip disasm, header magic verification.

`python -m pytest tests/test_bytecode.py -v` MUST pass."""),

    ("AGENT-09: Parity Ledger Update", """You are updating the Ark Intrinsic Parity Ledger.

TARGET: INTRINSIC_PARITY.md, README.md

S-LANG TRACE: $ParityLedger >> $AuditCount !! $UpdateMarkers_RecalcPercentage

STEPS:
1. Read meta/ark_intrinsics.py INTRINSICS dict — count ALL registered intrinsic names = Total Python
2. Read core/src/intrinsics.rs dispatch match — count ALL registered = Total Rust. Note stubs vs real implementations.
3. Update INTRINSIC_PARITY.md: Mark ✅ (both real), 🟡 (Rust stub), ❌ (Python only). Categories: IO, Crypto, Math, Net, Chain, Thread.
4. Mark as ✅: sys.json.parse, sys.json.stringify, sys.exit, sys.log, sys.html_escape, sys.z3.verify
5. Update summary: Total, Rust Parity count/total (percentage), Remaining Debt
6. Update README.md: parity numbers, Gauntlet badge (61+ tests passing)

CONSTRAINTS: Read actual source files. Do NOT guess counts. Do NOT mark ✅ unless Rust impl is real (not todo!()). Percentage from actual counts."""),

    ("AGENT-10: LSP Server", """You are completing the Ark Language Server Protocol (LSP) implementation.

TARGET: apps/lsp_main.ark, apps/lsp.ark, meta/ark_lsp.py (NEW if needed)

S-LANG TRACE: $LSPServer >> $JSONRPC2_StdioTransport !! $Initialize_Completion_Hover_Diagnostics

DELIVER these LSP methods over stdio JSON-RPC 2.0:

1. `initialize` — Return capabilities: textDocumentSync=1, completionProvider with "." trigger, hoverProvider, definitionProvider, diagnosticProvider. serverInfo: ark-lsp v0.1.0.

2. `textDocument/didOpen` — Store doc in memory. Run diagnostics. Publish parse errors.

3. `textDocument/didChange` — Update stored doc. Re-run diagnostics.

4. `textDocument/completion` — Return: Ark keywords (let, func, if, else, while, for, return, import, struct, match, true, false), built-in intrinsics (sys.print, sys.exit, math.sin, crypto.sha256, etc.), struct fields after dot.

5. `textDocument/hover` — Intrinsic docs, keyword syntax descriptions. Return markdown content.

6. `textDocument/definition` — Find `func <name>` in current file, return location.

7. Diagnostics — Parse .ark file, capture errors with line numbers, send as LSP diagnostics.

TRANSPORT: Content-Length header + JSON body over stdin/stdout.

TEST (tests/test_lsp.py): Start LSP subprocess, send initialize, verify capabilities, send didOpen, send completion, verify keywords returned, send shutdown+exit.

The LSP must start with `python meta/ark_lsp.py` and respond correctly."""),
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
    try:
        r = requests.post(f"{BASE}/sessions", headers=HDR, json=payload, timeout=30)
        if r.status_code == 200:
            sid = r.json().get("name", "?")
            print(f"✅ {sid}")
            results.append({"title": title, "session": sid, "status": "OK"})
        else:
            print(f"❌ {r.status_code}: {r.text[:150]}")
            results.append({"title": title, "session": None, "status": f"ERR_{r.status_code}"})
    except Exception as e:
        print(f"❌ {e}")
        results.append({"title": title, "session": None, "status": str(e)})
    time.sleep(2)

print(f"\n{'='*60}")
ok = sum(1 for r in results if r["status"] == "OK")
print(f"DEPLOYED: {ok}/10 agents")
with open("swarm_dispatch_log.json", "w") as f:
    json.dump(results, f, indent=2)
print(f"Log: swarm_dispatch_log.json")
