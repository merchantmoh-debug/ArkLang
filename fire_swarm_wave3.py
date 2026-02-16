"""Fire Wave 3: 10 MORE Jules agents — all conflict-free with Waves 1 & 2."""
import requests, json, time, os, sys

KEY = "AQ.Ab8RN6KGwjGH6dWIx9d6CgnkN01pqEMz4hVpv_ixkkriy3vvLQ"
BASE = "https://jules.googleapis.com/v1alpha"
SRC = "sources/github/merchantmoh-debug/ark-compiler"
HDR = {"x-goog-api-key": KEY, "Content-Type": "application/json"}

with open(os.path.join(os.path.dirname(__file__), "ARK_AGENT_PAYLOAD.md"), "r") as f:
    PAYLOAD = f.read()

AGENTS = [
    ("AGENT-21: Rust Compiler Optimization", """You are optimizing and completing the Ark compiler backend.

TARGET FILE: core/src/compiler.rs (ONLY this file)
DO NOT TOUCH: intrinsics.rs, vm.rs, eval.rs, checker.rs, wasm.rs, or any other .rs file

S-LANG TRACE: $CompilerBackend >> $CodeGen !! $ConstFold_DeadElim_RegisterAlloc

MISSION: Read core/src/compiler.rs and implement these compiler optimizations and completions.

TASKS:

1. CONSTANT FOLDING
   - During compilation, detect expressions where both operands are literals
   - Fold `3 + 5` into `8` at compile time instead of emitting ADD instruction
   - Handle: integer arithmetic (+, -, *, /, %), boolean logic (&&, ||, !), string concatenation
   - Create a `fold_constants(ast: &AstNode) -> AstNode` function

2. DEAD CODE ELIMINATION
   - After a `return` statement, remove any subsequent statements in the same block
   - After a `break` or `continue`, remove subsequent loop body statements
   - `if true { ... } else { ... }` -> eliminate the else branch entirely
   - `if false { ... } else { ... }` -> eliminate the if branch, keep else
   - Create a `eliminate_dead_code(ast: &AstNode) -> AstNode` function

3. VARIABLE SCOPE TRACKING
   - Build a scope stack during compilation
   - Each function/block creates a new scope level
   - Variables are resolved to scope-aware indices for faster lookup
   - Detect shadowing and emit a compile warning (not error)

4. COMPILATION ERROR MESSAGES
   - Replace any panic!() or unwrap() with descriptive error returns
   - Error messages should include: source file, line number, column, error description
   - Create a CompileError struct: { message: String, line: usize, column: usize, file: String }
   - Implement Display for CompileError with a nice formatted output

5. OPTIMIZATION PASS PIPELINE
   - Create a `fn optimize(ast: AstNode, level: u8) -> AstNode` function
   - Level 0: no optimizations (debug mode)
   - Level 1: constant folding only
   - Level 2: constant folding + dead code elimination
   - Level 3: all optimizations (future: inlining, loop unrolling)

TESTS:
- test_constant_folding_integers: `3 + 5` folds to `8`
- test_constant_folding_booleans: `true && false` folds to `false`
- test_dead_code_after_return: statements after return are removed
- test_optimization_level_0: no changes at level 0
- test_compile_error_format: verify error message includes line/col

`cargo test` MUST pass."""),

    ("AGENT-22: Rust Runtime Hardening", """You are hardening the Ark runtime system.

TARGET FILE: core/src/runtime.rs (ONLY this file)
DO NOT TOUCH: intrinsics.rs, vm.rs, eval.rs, compiler.rs, or any other .rs file

S-LANG TRACE: $RuntimeHarden >> $MemoryManagement !! $GC_ValuePool_ResourceTracker

MISSION: Read core/src/runtime.rs and implement production hardening.

TASKS:

1. RESOURCE TRACKER
   - Track all allocated resources: file handles, network sockets, thread handles
   - Create a ResourceTracker struct with methods: register(id, type), release(id), cleanup_all()
   - On runtime shutdown, automatically release any unclosed resources
   - Log warnings for resources not explicitly closed (resource leak detection)

2. VALUE POOL / ARENA ALLOCATOR
   - Implement a simple value pool for frequently allocated ArkValues
   - Pool small integers (-128 to 127) — return cached references instead of new allocations
   - Pool common strings ("", "true", "false", "nil") — same idea
   - This reduces allocation pressure significantly for arithmetic-heavy programs

3. MEMORY LIMITS
   - Add a configurable MAX_MEMORY_MB constant (default: 256 MB)
   - Track approximate memory usage: each ArkValue::String tracks its byte length
   - Before allocating large strings or lists, check if usage would exceed limit
   - Return ArkError::OutOfMemory if limit would be exceeded

4. GRACEFUL SHUTDOWN
   - Implement a shutdown sequence: signal all threads to stop, close all sockets, flush all files
   - Handle Ctrl+C (SIGINT on Unix, SetConsoleCtrlHandler on Windows) via ctrlc handler
   - Set a global `SHUTTING_DOWN: AtomicBool` flag that runtime loops check
   - After setting flag, wait up to 5 seconds for threads to join, then force-exit

5. RUNTIME STATISTICS
   - Track: total_instructions_executed, total_function_calls, total_allocations, peak_memory_bytes
   - Create a RuntimeStats struct with these counters
   - Add a `stats()` method that returns a formatted summary string
   - Optionally print stats on shutdown if ARK_RUNTIME_STATS env var is set

6. ERROR RECOVERY
   - Replace all panic!() calls with proper Result returns
   - Create runtime-specific error types: RuntimeError, AllocationError, ResourceError
   - Implement From<std::io::Error> for RuntimeError

TESTS:
- test_resource_tracker_register_release
- test_resource_tracker_cleanup_unreleased
- test_value_pool_integer_caching
- test_memory_limit_enforcement
- test_runtime_stats_counting

`cargo test` MUST pass."""),

    ("AGENT-23: Rust FFI Completion", """You are completing the Ark Foreign Function Interface (FFI).

TARGET FILE: core/src/ffi.rs (ONLY this file)
DO NOT TOUCH: intrinsics.rs, vm.rs, runtime.rs, or any other .rs file

S-LANG TRACE: $FFIComplete >> $CInterop !! $DynLib_TypeConvert_SafeCall

MISSION: Read core/src/ffi.rs and complete it as a production-grade FFI system for calling C libraries from Ark.

TASKS:

1. DYNAMIC LIBRARY LOADING
   - Implement safe wrappers around libloading (or dlopen/LoadLibrary)
   - `ffi_load_library(path: &str) -> Result<LibraryHandle, FfiError>`
   - Store loaded libraries in a global HashMap<String, Library>
   - Unload libraries on runtime shutdown
   - If libloading is not in Cargo.toml, note the dependency but do NOT modify Cargo.toml (other agents may be touching it). Create a file core/ffi_cargo_additions.txt with the needed dependency.

2. FUNCTION LOOKUP
   - `ffi_get_function(lib: &str, func_name: &str) -> Result<FunctionHandle, FfiError>`
   - Cache function pointers for repeated calls
   - Return descriptive error if function not found

3. TYPE CONVERSION: ArkValue <-> C Types
   - ArkValue::Integer -> c_long / i64
   - ArkValue::String -> *const c_char (with null terminator)
   - ArkValue::Boolean -> c_int (0 or 1)
   - ArkValue::List -> raw array pointer + length (for numeric lists)
   - C return values back to ArkValue using the same mapping
   - CRITICAL: All string conversions must use CString to ensure null termination
   - CRITICAL: All pointer conversions must be bounds-checked

4. SAFE FFI CALL WRAPPER
   - `ffi_call(lib: &str, func: &str, args: &[ArkValue], return_type: &str) -> ArkResult`
   - Wrap the actual FFI call in `std::panic::catch_unwind`
   - Set a timeout for FFI calls (default 10 seconds) using a watchdog thread
   - Log all FFI calls for debugging (function name, arg types, return value)

5. FFI ERROR TYPES
```rust
#[derive(Debug)]
enum FfiError {
    LibraryNotFound(String),
    FunctionNotFound(String),
    TypeConversionError(String),
    CallFailed(String),
    Timeout(String),
    InvalidPointer(String),
}
```

6. SECURITY CONSTRAINTS
   - Maintain an ALLOWED_LIBRARIES list (configurable)
   - Reject loading libraries not on the allowlist
   - Reject function names containing suspicious characters
   - Log all FFI operations for audit trail

TESTS:
- test_type_conversion_integer
- test_type_conversion_string
- test_ffi_error_display
- test_security_reject_unlisted_library
- test_cstring_null_termination

`cargo test` MUST pass. (FFI tests that require actual C libraries should be gated behind #[cfg(test)] with mock libraries or libc calls like strlen)."""),

    ("AGENT-24: Rust Blockchain Subsystem", """You are completing the Ark blockchain subsystem.

TARGET FILES: core/src/blockchain.rs + core/src/consensus.rs (ONLY these two files)
DO NOT TOUCH: intrinsics.rs, vm.rs, or any other .rs file

S-LANG TRACE: $BlockchainSubsystem >> $ChainCore !! $Block_Merkle_PoW_Consensus

MISSION: Read core/src/blockchain.rs and core/src/consensus.rs and complete them as a functional in-memory blockchain.

TASKS FOR blockchain.rs:

1. BLOCK STRUCTURE
```rust
#[derive(Debug, Clone)]
struct Block {
    index: u64,
    timestamp: u64,
    data: Vec<Transaction>,
    previous_hash: String,
    hash: String,
    nonce: u64,
}

#[derive(Debug, Clone)]
struct Transaction {
    sender: String,
    receiver: String,
    amount: i64,
    signature: String,  // hex string
}
```

2. MERKLE TREE
   - Compute Merkle root of transactions in a block
   - `fn merkle_root(transactions: &[Transaction]) -> String`
   - Use SHA-256 for hashing, pair and hash up the tree
   - Single transaction: hash of its serialization
   - Empty block: hash of empty string

3. BLOCK VALIDATION
   - `fn validate_block(block: &Block, previous_block: &Block) -> bool`
   - Check: block.previous_hash == previous_block.hash
   - Check: block.index == previous_block.index + 1
   - Check: block.hash starts with required number of zeros (difficulty)
   - Recompute hash and verify it matches block.hash

4. CHAIN STATE
   - `struct Blockchain { chain: Vec<Block>, pending_tx: Vec<Transaction>, difficulty: u32 }`
   - `fn add_transaction(&mut self, tx: Transaction) -> bool` — validate and add to pending
   - `fn mine_block(&mut self) -> Block` — create new block from pending transactions
   - `fn get_balance(&self, address: &str) -> i64` — walk the chain, sum credits - debits
   - `fn is_chain_valid(&self) -> bool` — validate entire chain from genesis

5. GENESIS BLOCK
   - Auto-create genesis block with index=0, previous_hash="0", empty data

TASKS FOR consensus.rs:

1. PROOF OF WORK
   - `fn proof_of_work(block: &mut Block, difficulty: u32)` — find nonce where hash starts with N zeros
   - Use SHA-256(index + timestamp + data + previous_hash + nonce) for hash computation
   - difficulty=1 means hash starts with "0", difficulty=2 means "00", etc.

2. DIFFICULTY ADJUSTMENT
   - Target block time: 10 seconds
   - If last 10 blocks took < 100s total, increase difficulty by 1
   - If last 10 blocks took > 100s total, decrease difficulty by 1 (min 1)

TESTS:
- test_genesis_block_creation
- test_add_transaction
- test_mine_block
- test_chain_validation_valid
- test_chain_validation_tampered (modify a block, verify chain invalid)
- test_merkle_root_single_tx
- test_merkle_root_multiple_tx
- test_balance_tracking
- test_proof_of_work_difficulty_1

`cargo test` MUST pass."""),

    ("AGENT-25: Rust AST & Type System", """You are enhancing the Ark AST and type system.

TARGET FILES: core/src/ast.rs + core/src/types.rs (ONLY these two files)
DO NOT TOUCH: eval.rs, checker.rs, compiler.rs, vm.rs, or any other .rs file

S-LANG TRACE: $ASTTypes >> $TypeTheory !! $ASTNodes_TypeInference_Generics

MISSION: Read core/src/ast.rs and core/src/types.rs and enhance them.

TASKS FOR ast.rs:

1. SOURCE LOCATION TRACKING
   - Add `Span` struct: { start_line: u32, start_col: u32, end_line: u32, end_col: u32, file: String }
   - Add `span: Option<Span>` field to the main AST node enum/struct
   - This enables better error messages ("Error at line 5, column 12 in main.ark")

2. AST VISITOR PATTERN
   - Implement a Visitor trait:
```rust
trait AstVisitor {
    fn visit_expr(&mut self, expr: &Expr) -> VisitResult;
    fn visit_stmt(&mut self, stmt: &Stmt) -> VisitResult;
    fn visit_func(&mut self, func: &FuncDecl) -> VisitResult;
    fn visit_block(&mut self, block: &[Stmt]) -> VisitResult;
}
```
   - Implement `walk_ast(visitor: &mut dyn AstVisitor, ast: &[Stmt])` that traverses the tree

3. AST PRETTY PRINTER
   - `fn pretty_print(ast: &[Stmt], indent: usize) -> String`
   - Produce nicely formatted Ark source code from the AST
   - This is the basis for `ark fmt` (code formatter)
   - Handle: variable declarations, function definitions, if/else, while, for, match, structs

4. MISSING AST NODES (if not present, add them):
   - `Expr::Match { scrutinee, arms }` for match expressions
   - `Expr::Lambda { params, body }` for anonymous functions
   - `Expr::TryCatch { try_block, catch_var, catch_block }` for error handling
   - `Stmt::Import { path, alias }` for imports with optional aliasing
   - `Stmt::StructDecl { name, fields }` for struct definitions

TASKS FOR types.rs:

1. TYPE ENUM ENHANCEMENT
```rust
#[derive(Debug, Clone, PartialEq)]
enum ArkType {
    Integer,
    Float,
    String,
    Boolean,
    List(Box<ArkType>),       // List<T>
    Map(Box<ArkType>, Box<ArkType>),  // Map<K, V>
    Struct(String, Vec<(String, ArkType)>),  // Named struct with fields
    Function(Vec<ArkType>, Box<ArkType>),    // (params) -> return
    Optional(Box<ArkType>),   // T?
    Unit,                     // void/nil
    Any,                      // dynamic type
    Unknown,                  // not yet inferred
}
```

2. TYPE DISPLAY
   - Implement `Display` for `ArkType`
   - Integer -> "Int", String -> "Str", List(Integer) -> "List<Int>", etc.

3. TYPE COMPATIBILITY CHECK
   - `fn is_compatible(expected: &ArkType, actual: &ArkType) -> bool`
   - Any is compatible with everything
   - Unknown is compatible with everything (not yet inferred)
   - List<T> is compatible with List<U> if T is compatible with U
   - Function compatibility: parameter types must match, return type must match

4. TYPE NARROWING
   - `fn narrow(current: &ArkType, evidence: &ArkType) -> ArkType`
   - After `if x != nil`, narrow Optional(T) to T
   - After type check, narrow Any to the specific type

TESTS:
- test_span_creation
- test_ast_pretty_print_simple
- test_ast_visitor_counts_functions
- test_type_display_format
- test_type_compatibility_basic
- test_type_compatibility_generics
- test_type_narrowing

`cargo test` MUST pass."""),

    ("AGENT-26: Rust Crypto Module", """You are completing the standalone Ark crypto module.

TARGET FILE: core/src/crypto.rs (ONLY this file)
DO NOT TOUCH: intrinsics.rs or any other .rs file

NOTE: Agent 03 is adding crypto INTRINSICS (sha512, hmac, etc.) to intrinsics.rs. You are working on crypto.rs which is a SEPARATE, standalone cryptographic utilities module that provides higher-level crypto abstractions.

S-LANG TRACE: $CryptoModule >> $HighLevelCrypto !! $Wallet_KeyDerivation_Signatures

MISSION: Read core/src/crypto.rs and complete it as a high-level cryptographic utilities module.

TASKS:

1. KEY DERIVATION
   - `fn derive_key(master_seed: &[u8], path: &str) -> Vec<u8>`
   - BIP-32 style hierarchical key derivation (simplified)
   - Path format: "m/0/1/2" where each number is a child index
   - Use HMAC-SHA512 for derivation (import from the sha2/hmac crates if available, or use a pure Rust implementation)

2. WALLET ADDRESS GENERATION
   - `fn generate_address(public_key: &[u8]) -> String`
   - SHA-256 of public key -> RIPEMD-160 (or just SHA-256 again for simplicity) -> hex encode
   - Add a 4-byte checksum (first 4 bytes of double-SHA-256)
   - Return as a hex string with "ark:" prefix

3. TRANSACTION SIGNING
   - `struct SignedTransaction { data: Vec<u8>, signature: Vec<u8>, public_key: Vec<u8> }`
   - `fn sign_transaction(data: &[u8], private_key: &[u8]) -> SignedTransaction`
   - `fn verify_transaction(tx: &SignedTransaction) -> bool`
   - Use Ed25519 if the crate is available, otherwise HMAC-based signing as fallback

4. SECURE RANDOM
   - `fn secure_random_bytes(count: usize) -> Vec<u8>`
   - Use OsRng from rand crate if available, otherwise getrandom
   - `fn secure_random_hex(count: usize) -> String` — random bytes as hex string
   - `fn generate_nonce() -> [u8; 32]` — 32-byte random nonce

5. HASH UTILITIES
   - `fn hash_sha256(data: &[u8]) -> [u8; 32]`
   - `fn hash_double_sha256(data: &[u8]) -> [u8; 32]` — SHA256(SHA256(data))
   - `fn hash_to_hex(hash: &[u8]) -> String`

6. CONSTANT-TIME COMPARISON
   - `fn constant_time_eq(a: &[u8], b: &[u8]) -> bool`
   - Prevents timing attacks on signature/hash comparison
   - XOR all bytes, check if result is zero

TESTS:
- test_sha256_known_vector: hash of "" matches known SHA-256
- test_double_sha256
- test_address_generation_format: starts with "ark:", correct length
- test_constant_time_eq_same
- test_constant_time_eq_different
- test_derive_key_deterministic: same seed + path always produces same key
- test_sign_verify_roundtrip

`cargo test` MUST pass."""),

    ("AGENT-27: Python Agent Framework", """You are completing the Ark AI Agent Framework.

TARGET FILES: src/agent.py + src/agents/base_agent.py + src/agents/coder_agent.py + src/agents/researcher_agent.py + src/agents/reviewer_agent.py + src/agents/router_agent.py
DO NOT TOUCH: src/sandbox/*.py, src/mcp_client.py, src/tools/*.py, src/memory.py, src/swarm.py, meta/*.py, core/*.rs

S-LANG TRACE: $AgentFramework >> $MultiAgent !! $Router_Coder_Researcher_Reviewer

MISSION: Read the agent framework files and complete them as a production-grade multi-agent system.

TASKS:

1. BASE AGENT (src/agents/base_agent.py)
   - Ensure the BaseAgent class has:
     - `__init__(self, name, model, system_prompt, tools=None, memory=None)`
     - `async def run(self, task: str) -> str` — main execution method
     - `async def think(self, context: str) -> str` — internal reasoning step
     - `def add_tool(self, tool)` — register a tool the agent can use
     - `def log(self, level, message)` — structured logging with timestamp
   - Error handling: wrap all LLM calls in try/except, retry 3 times with exponential backoff
   - Token counting: track input/output tokens per run

2. ROUTER AGENT (src/agents/router_agent.py)
   - Analyzes incoming tasks and routes to the appropriate specialist agent
   - Decision logic:
     - Contains "write code" / "implement" / "fix bug" -> CoderAgent
     - Contains "research" / "find" / "search" / "analyze" -> ResearcherAgent
     - Contains "review" / "audit" / "check" -> ReviewerAgent
     - Default -> CoderAgent
   - Return routing decision with confidence score
   - Support multi-agent workflows: Router -> Coder -> Reviewer (pipeline)

3. CODER AGENT (src/agents/coder_agent.py)
   - Specialized for code generation and modification
   - System prompt should include Ark language syntax reference
   - Tools: file_read, file_write, run_command, search_code
   - Output format: structured with {files_changed: [], tests_added: [], summary: ""}

4. RESEARCHER AGENT (src/agents/researcher_agent.py)
   - Specialized for information gathering and analysis
   - Tools: web_search, read_url, summarize
   - Maintains a research log with sources and findings
   - Output format: {findings: [], sources: [], confidence: 0.0-1.0}

5. REVIEWER AGENT (src/agents/reviewer_agent.py)
   - Specialized for code review and quality assessment
   - Checks: code style, potential bugs, security issues, test coverage
   - Severity levels: CRITICAL, HIGH, MEDIUM, LOW, INFO
   - Output format: {issues: [{severity, file, line, description, suggestion}], approved: bool}

6. AGENT ORCHESTRATOR (src/agent.py)
   - High-level orchestrator that manages the agent pipeline
   - `async def execute_task(task: str) -> dict` — route and execute
   - Pipeline support: task -> Router -> [Specialist] -> [Reviewer] -> result
   - Conversation history tracking between agents
   - Total token usage reporting

TESTING: Verify all classes can be instantiated without errors. Add basic unit tests to each file using doctest or a `if __name__ == "__main__"` block.

`python -c "from src.agents.base_agent import BaseAgent; print('OK')"` must work."""),

    ("AGENT-28: Python Sandbox System", """You are completing the Ark sandbox execution system.

TARGET FILES: src/sandbox/base.py + src/sandbox/local.py + src/sandbox/docker_exec.py + src/sandbox/factory.py
DO NOT TOUCH: src/agent.py, src/agents/*.py, src/mcp_client.py, src/tools/*.py, meta/*.py, core/*.rs

S-LANG TRACE: $SandboxSystem >> $SecureExec !! $LocalIsolation_DockerExec_Factory

MISSION: Read the sandbox files and complete them as a production-grade code execution sandbox.

TASKS:

1. BASE SANDBOX (src/sandbox/base.py)
   - Abstract base class with interface:
     - `async def execute(self, code: str, language: str, timeout: int = 30) -> ExecutionResult`
     - `async def cleanup(self)` — release resources
     - `def get_capabilities(self) -> set` — return available capabilities
   - ExecutionResult class: { stdout: str, stderr: str, exit_code: int, duration_ms: float, truncated: bool }
   - Max output size: 100KB (truncate with "[TRUNCATED]" marker)

2. LOCAL SANDBOX (src/sandbox/local.py)
   - Execute code locally using subprocess
   - Language support:
     - "ark" -> `python meta/ark.py run <tempfile>`
     - "python" -> `python <tempfile>`
     - "rust" -> compile with `rustc` then execute
     - "javascript" -> `node <tempfile>`
   - SECURITY:
     - Create tempfiles in a dedicated sandbox directory
     - Set resource limits: max CPU time (timeout), max memory (ulimit on Unix)
     - Clean up tempfiles after execution
     - Reject code containing: `import os; os.system`, `subprocess.Popen`, `eval(`, `exec(`
     - Capability checking: only allow operations matching granted capabilities

3. DOCKER SANDBOX (src/sandbox/docker_exec.py)
   - Execute code inside a Docker container for maximum isolation
   - `async def execute(self, code, language, timeout)`:
     a) Write code to a temp file
     b) `docker run --rm --network none --memory 256m --cpus 0.5 --read-only` with code mounted
     c) Capture stdout/stderr
     d) Enforce timeout with `docker kill` if exceeded
   - Container images: ark-sandbox:latest (define Dockerfile requirements)
   - Handle Docker not being available gracefully (fall back to local)
   - Output truncation at 100KB

4. SANDBOX FACTORY (src/sandbox/factory.py)
   - `def create_sandbox(sandbox_type: str = "auto", capabilities: set = None) -> BaseSandbox`
   - "auto" -> try Docker first, fall back to Local
   - "docker" -> Docker only (raise if Docker not available)
   - "local" -> Local only
   - Pass capabilities to the created sandbox
   - Singleton pattern: reuse sandbox instances within the same session

5. ERROR HANDLING
   - SandboxError base exception
   - SandboxTimeoutError — execution exceeded timeout
   - SandboxMemoryError — execution exceeded memory limit
   - SandboxSecurityError — code attempted a blocked operation
   - All errors include: the original code (first 200 chars), language, and error details

TESTING: Add to each file a `if __name__ == "__main__"` block that tests basic functionality.

`python -c "from src.sandbox.factory import create_sandbox; print('OK')"` must work."""),

    ("AGENT-29: MCP Client & Tools", """You are completing the Ark MCP (Model Context Protocol) client and tools.

TARGET FILES: src/mcp_client.py + src/tools/mcp_tools.py + src/tools/execution_tool.py + src/tools/openai_proxy.py + src/tools/ollama_local.py + src/tools/demo_tool.py
DO NOT TOUCH: src/agent.py, src/agents/*.py, src/sandbox/*.py, src/memory.py, meta/*.py, core/*.rs

S-LANG TRACE: $MCPTools >> $ProtocolClient !! $JSONRPCTransport_ToolRegistry_Invoke

MISSION: Read the MCP client and tools, then complete them for production use.

TASKS FOR src/mcp_client.py:

1. JSON-RPC 2.0 TRANSPORT
   - Implement proper JSON-RPC 2.0 message framing (Content-Length header + JSON body)
   - `async def send_request(self, method: str, params: dict) -> dict`
   - `async def send_notification(self, method: str, params: dict)`
   - Request ID tracking with auto-increment
   - Response matching by ID (handle out-of-order responses)

2. MCP LIFECYCLE
   - `async def initialize(self) -> dict` — send initialize, receive capabilities
   - `async def list_tools(self) -> list` — get available tools from server
   - `async def call_tool(self, name: str, arguments: dict) -> dict` — invoke a tool
   - `async def list_resources(self) -> list` — get available resources
   - `async def read_resource(self, uri: str) -> str` — read a resource
   - `async def shutdown(self)` — clean shutdown

3. CONNECTION MANAGEMENT
   - Support stdio transport (subprocess with stdin/stdout pipes)
   - Support SSE transport (HTTP Server-Sent Events) for remote MCP servers
   - Auto-reconnect on connection loss (3 retries with backoff)
   - Connection health check with periodic pings

4. ERROR HANDLING
   - MCPError base exception with error code and message
   - MCPConnectionError — transport failures
   - MCPTimeoutError — request timeout (default 30s)
   - MCPToolError — tool execution failures

TASKS FOR src/tools/:

5. EXECUTION TOOL (src/tools/execution_tool.py)
   - Tool that executes Ark code via the sandbox
   - MCP tool schema: { name: "execute_ark", inputSchema: { code: string, timeout?: number } }
   - Returns: { stdout, stderr, exit_code, duration_ms }

6. OPENAI PROXY (src/tools/openai_proxy.py)
   - Proxy that forwards LLM requests to OpenAI-compatible APIs
   - Support: chat completions, embeddings
   - Configurable base_url for local models (Ollama, LM Studio, etc.)
   - Rate limiting: max N requests per minute
   - Token counting and cost estimation

7. OLLAMA LOCAL (src/tools/ollama_local.py)
   - Direct integration with local Ollama instance
   - `async def generate(self, model: str, prompt: str) -> str`
   - `async def list_models(self) -> list`
   - Health check: verify Ollama is running at localhost:11434
   - Model auto-pull if not available locally

8. TOOL REGISTRY
   - In src/tools/__init__.py or mcp_tools.py:
   - `class ToolRegistry: register(name, handler, schema), get(name), list_all()`
   - Auto-discover tools from src/tools/ directory
   - Validate tool schemas against JSON Schema spec

TESTING: `python -c "from src.mcp_client import MCPClient; print('OK')"` must work. Each tool module should be independently importable."""),

    ("AGENT-30: Web Playground", """You are building the Ark Web Playground — a browser-based IDE for writing and running Ark code.

TARGET FILES: web/index.html + web/main.js + site/index.html + site/js/*.js + site/css/*.css
DO NOT TOUCH: core/*.rs, meta/*.py, src/*.py, apps/*.ark, lib/*.ark

S-LANG TRACE: $WebPlayground >> $BrowserIDE !! $CodeMirror_ExecutionPanel_DarkMode

MISSION: Read the existing web/ and site/ files, then upgrade them into a polished, production-quality web playground.

DESIGN REQUIREMENTS:
- Dark mode by default (think VS Code dark theme)
- Split panel: code editor on left, output console on right
- Responsive: works on desktop and tablet
- Fast: no heavy frameworks, vanilla JS + CodeMirror (via CDN)

TASKS FOR web/index.html + web/main.js:

1. CODE EDITOR
   - Integrate CodeMirror 6 via CDN for syntax highlighting
   - Create a custom Ark language mode with keyword highlighting:
     Keywords: let, func, if, else, while, for, return, import, struct, match, true, false, nil
   - Line numbers, bracket matching, auto-indent
   - Ctrl+Enter to run code

2. OUTPUT CONSOLE
   - Display stdout and stderr from code execution
   - Color stderr in red, stdout in white/green
   - Timestamp each output line
   - "Clear" button to reset console
   - Auto-scroll to bottom on new output

3. EXECUTION
   - "Run" button (and Ctrl+Enter shortcut)
   - POST the code to a local server endpoint: `POST /api/run` with body { code: "..." }
   - Display a "Running..." spinner while waiting
   - Timeout indicator if execution takes > 10 seconds
   - If no server is available, show a message: "Start the Ark server: python meta/ark.py serve"

4. EXAMPLE PROGRAMS
   - Dropdown/sidebar with built-in examples:
     - Hello World: `print("Hello, Ark!")`
     - Fibonacci: recursive fibonacci function
     - Linked List: struct-based linked list
     - HTTP Server: simple server example
     - Crypto: SHA-256 hash example
   - Clicking an example loads it into the editor

5. HEADER/TOOLBAR
   - Ark logo (text-based: "ARK" in bold monospace)
   - Theme toggle (dark/light)
   - Font size controls (+/-)
   - "Share" button (copies code as base64 URL parameter)
   - Link to documentation

TASKS FOR site/index.html (Landing Page):

6. LANDING PAGE
   - Hero section: "ARK — The Sovereign Computing Stack"
   - Subtitle: "JIT-compiled language for zero-cost, localized software"
   - "Try in Playground" button -> links to web/index.html
   - Feature cards: Fast (JIT compiled), Secure (sandboxed), Complete (95+ intrinsics)
   - Getting started section with installation commands
   - Dark gradient background, modern typography (Inter font from Google Fonts)

7. CSS (site/css/style.css)
   - CSS custom properties for theme colors
   - Dark mode: background #1a1a2e, text #e0e0e0, accent #00d4ff
   - Smooth transitions on hover/focus
   - Mobile responsive (flexbox/grid)

VERIFICATION: Open web/index.html in a browser — it should display without errors and look professional. The editor should be functional (even without a server, it should gracefully handle the missing backend)."""),
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
print(f"WAVE 3 DEPLOYED: {ok}/10 agents")
with open("swarm_wave3_log.json", "w") as f:
    json.dump(results, f, indent=2)
print(f"Log: swarm_wave3_log.json")
