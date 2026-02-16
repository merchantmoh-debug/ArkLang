"""Fire Wave 4: Final 7 agents — covers ALL remaining untouched files."""
import requests, json, time, os, sys

KEY = "AQ.Ab8RN6KGwjGH6dWIx9d6CgnkN01pqEMz4hVpv_ixkkriy3vvLQ"
BASE = "https://jules.googleapis.com/v1alpha"
SRC = "sources/github/merchantmoh-debug/ark-compiler"
HDR = {"x-goog-api-key": KEY, "Content-Type": "application/json"}

with open(os.path.join(os.path.dirname(__file__), "ARK_AGENT_PAYLOAD.md"), "r") as f:
    PAYLOAD = f.read()

AGENTS = [
    ("AGENT-31: Python Interpreter Hardening", """You are hardening the Ark Python tree-walking interpreter.

TARGET FILE: meta/ark_interpreter.py (ONLY this file)
DO NOT TOUCH: ark_intrinsics.py, ark.py, ark_parser.py, ark_security.py, gauntlet.py, compile.py, repl.py, or any other file

S-LANG TRACE: $InterpreterHarden >> $TreeWalker !! $ErrorRecovery_ScopeChain_TailCall

MISSION: Read meta/ark_interpreter.py and harden it for production use.

TASKS:

1. ERROR RECOVERY WITH LINE NUMBERS
   - Every ArkRuntimeError should include the source line number and column
   - When evaluating an AST node, wrap in try/except and re-raise with context:
     `raise ArkRuntimeError(f"Line {node.line}: {original_error}")`
   - Create a formatted traceback that shows the call stack:
     ```
     Traceback (most recent call last):
       File "main.ark", line 15, in fibonacci
       File "main.ark", line 8, in main
     RuntimeError: Division by zero
     ```

2. SCOPE CHAIN OPTIMIZATION
   - If scope lookup is linear search through parent scopes, optimize it
   - Use a dict-based scope chain with fast parent lookup
   - Variables should resolve in O(1) for local scope, O(depth) for outer scopes
   - Add scope depth tracking for debugging

3. TAIL CALL OPTIMIZATION (TCO)
   - Detect tail-recursive function calls (last expression is a self-call)
   - Convert tail calls into loop iterations to prevent stack overflow
   - This allows `func factorial(n, acc) { if n <= 1 { return acc } return factorial(n-1, n*acc) }` to run without blowing the Python stack

4. RECURSION DEPTH LIMIT
   - Add MAX_RECURSION_DEPTH = 1000 (configurable via env var ARK_MAX_RECURSION)
   - Track current depth in the interpreter state
   - Raise ArkRuntimeError("Maximum recursion depth exceeded") instead of Python RecursionError

5. ASSERTION & INVARIANT CHECKS
   - Before evaluating binary operations, verify both operands are valid ArkValues
   - Before function calls, verify the callee is actually callable
   - Before list indexing, verify the index is an integer and within bounds
   - All checks should produce descriptive error messages

6. PERFORMANCE: VARIABLE CACHING
   - Cache frequently accessed variables (those accessed > 10 times) in a fast lookup dict
   - Clear cache when a variable is reassigned
   - This is an optimization for hot loops

TESTING: Run the existing Gauntlet to verify no regressions:
`python meta/gauntlet.py` should produce the same PASS/FAIL counts as before.

DO NOT break any existing tests. All changes must be backward-compatible."""),

    ("AGENT-32: Parser & Grammar Enhancement", """You are enhancing the Ark parser and grammar.

TARGET FILES: meta/ark_parser.py + meta/ark.lark (ONLY these two files)
DO NOT TOUCH: ark_interpreter.py, ark_intrinsics.py, ark.py, compile.py, or any other file

S-LANG TRACE: $ParserGrammar >> $LarkPEG !! $NewSyntax_ErrorRecovery_AST

MISSION: Read meta/ark_parser.py and meta/ark.lark, then enhance them.

TASKS FOR meta/ark.lark (Grammar):

1. ADD MISSING SYNTAX (if not already present):
   - Match expressions: `match expr { pattern => body, ... }`
   - Lambda/anonymous functions: `|x, y| { x + y }` or `func(x, y) { x + y }`
   - Try-catch: `try { ... } catch e { ... }`
   - String interpolation: `f"Hello {name}, you are {age} years old"`
   - Range literals: `1..10` or `1..=10` (inclusive)
   - Optional chaining: `obj?.field` (returns nil if obj is nil)
   - Pipe operator: `x |> f |> g` equivalent to `g(f(x))`
   - Multi-line strings: triple-quote `\"""...\"""`

2. OPERATOR PRECEDENCE (verify and fix if needed):
   - Highest: () [] . ?. 
   - Unary: ! - ~
   - Multiplicative: * / %
   - Additive: + -
   - Comparison: < > <= >= == !=
   - Logical: && ||
   - Pipe: |>
   - Assignment: = := += -= *= /=

3. COMMENT HANDLING:
   - Single-line: `// comment`
   - Multi-line: `/* comment */`
   - Doc comments: `/// This is a doc comment` (preserved in AST for documentation)

TASKS FOR meta/ark_parser.py:

4. ERROR RECOVERY
   - When a parse error occurs, try to recover and continue parsing
   - Skip to the next statement boundary (newline or semicolon) and continue
   - Collect ALL errors, not just the first one
   - Return partial AST + list of errors

5. AST ENRICHMENT
   - Add line number and column to every AST node
   - Add source file name to the root AST node
   - Store doc comments (///) attached to the following function/struct declaration

6. PARSER DIAGNOSTICS
   - Track: total tokens parsed, parse time in ms, number of AST nodes generated
   - Print diagnostics when ARK_PARSE_DEBUG env var is set

TESTING:
- Parse `x := 1 + 2` and verify AST structure
- Parse a function definition and verify parameter extraction
- Parse invalid syntax and verify error message includes line number
- If you added new syntax, create small test cases for each

Run `python meta/gauntlet.py` to verify no regressions. DO NOT break existing tests."""),

    ("AGENT-33: JSON Transpiler Completion", """You are completing the Ark-to-JSON transpilation pipeline.

TARGET FILE: meta/ark_to_json.py (ONLY this file)
DO NOT TOUCH: ark_parser.py, ark_interpreter.py, ark.py, compile.py, or any other file

S-LANG TRACE: $JSONTranspiler >> $ASTSerialization !! $Emit_Roundtrip_Schema

MISSION: Read meta/ark_to_json.py and complete it as a robust AST-to-JSON pipeline.

TASKS:

1. COMPLETE AST-TO-JSON SERIALIZATION
   - Every AST node type must have a JSON serialization
   - Each node should serialize as: { "type": "NodeType", "line": N, "col": N, ...fields }
   - Handle all expression types: Binary, Unary, Call, Index, FieldAccess, Literal, Identifier
   - Handle all statement types: VarDecl, FuncDecl, If, While, For, Return, Import, Block
   - Handle structs, match expressions, try-catch

2. JSON-TO-AST DESERIALIZATION (ROUNDTRIP)
   - `def json_to_ast(json_data: dict) -> AstNode`
   - Parse the JSON back into AST nodes
   - This enables: parse -> JSON -> save to file -> load -> execute
   - Verify roundtrip: ast_to_json(json_to_ast(ast_to_json(ast))) == ast_to_json(ast)

3. JSON SCHEMA GENERATION
   - `def generate_schema() -> dict` — return a JSON Schema that validates Ark AST JSON
   - The schema should define all valid node types, their required and optional fields
   - This allows external tools to validate Ark AST files

4. PRETTY JSON OUTPUT
   - Default output: compact (no whitespace)
   - `--pretty` flag: indented with 2 spaces, sorted keys
   - `--minify` flag: single line, no spaces
   - Include a comment header: `// Generated by ark_to_json.py v0.1.0`

5. SOURCE MAP GENERATION
   - When transpiling, generate a mapping from JSON positions to source positions
   - Format: `{ "version": 3, "mappings": [...] }` (simplified source map)
   - This enables debugging: when a JSON node causes an error, trace back to source line

6. CLI INTERFACE
```python
if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Ark AST to JSON")
    parser.add_argument("input", help="Input .ark file")
    parser.add_argument("-o", "--output", help="Output .json file (default: stdout)")
    parser.add_argument("--pretty", action="store_true", help="Pretty-print JSON")
    parser.add_argument("--schema", action="store_true", help="Output JSON Schema instead")
    parser.add_argument("--roundtrip", action="store_true", help="Test roundtrip accuracy")
```

TESTING:
- Transpile tests/sanity.ark to JSON, verify valid JSON output
- Roundtrip test: parse -> json -> parse -> json, verify identical
- Schema validation: generated schema validates generated JSON

`python meta/ark_to_json.py meta/test.ark --pretty` must work without errors."""),

    ("AGENT-34: Memory + Swarm System", """You are completing the Ark Memory and Swarm orchestration system.

TARGET FILES: src/memory.py + src/swarm.py + src/config.py (ONLY these three files)
DO NOT TOUCH: src/agent.py, src/agents/*.py, src/sandbox/*.py, src/mcp_client.py, src/tools/*.py, meta/*.py, core/*.rs

S-LANG TRACE: $MemorySwarm >> $Persistence !! $EncryptedStore_SwarmOrch_Config

TASKS FOR src/memory.py:

1. ENCRYPTED MEMORY STORE
   - `class MemoryManager` with encrypted persistence
   - Store memories as encrypted JSON using Fernet (from cryptography library) or AES via stdlib
   - Key derivation from a master password using PBKDF2
   - File storage: ~/.ark/memory/<namespace>.enc
   - Methods:
     - `store(key: str, value: any, namespace: str = "default")`
     - `recall(key: str, namespace: str = "default") -> any`
     - `search(query: str, namespace: str = "default") -> list` — fuzzy text search
     - `forget(key: str, namespace: str = "default")`
     - `list_keys(namespace: str = "default") -> list`

2. CONVERSATION HISTORY
   - `class ConversationHistory` that extends MemoryManager
   - Store conversation turns: {role: "user"|"assistant", content: str, timestamp: str}
   - `add_turn(role, content)`
   - `get_context(max_turns: int = 10) -> list` — returns last N turns
   - `summarize() -> str` — generate a summary of the conversation (for context window management)

3. VECTOR MEMORY (SIMPLE)
   - Simple TF-IDF based semantic search (no external deps)
   - `store_embedding(key: str, text: str)` — compute TF-IDF vector from text
   - `search_similar(query: str, top_k: int = 5) -> list` — cosine similarity search
   - This enables the agent to recall relevant past information

TASKS FOR src/swarm.py:

4. SWARM ORCHESTRATOR
   - `class SwarmOrchestrator` that manages multiple agents
   - Methods:
     - `add_agent(agent)` — register an agent in the swarm
     - `async def execute(task: str, strategy: str = "router") -> dict` — execute with strategy
     - `async def execute_parallel(tasks: list) -> list` — run multiple tasks in parallel
     - `async def execute_pipeline(task: str, pipeline: list) -> dict` — sequential agent chain
   - Strategies:
     - "router": use RouterAgent to decide which agent handles the task
     - "broadcast": send task to all agents, merge results
     - "consensus": send to 3 agents, take majority result
   - Inter-agent communication via a message bus (simple queue)
   - Progress tracking: emit events for task start/complete/error

5. SWARM MONITORING
   - Track: tasks_completed, tasks_failed, total_tokens_used, average_latency
   - `def status() -> dict` — return swarm health metrics
   - `def report() -> str` — formatted status report

TASKS FOR src/config.py:

6. CONFIGURATION SYSTEM
   - `class ArkConfig` — hierarchical configuration with precedence:
     1. Environment variables (ARK_*) — highest priority
     2. Config file (~/.ark/config.toml or config.json)
     3. Default values — lowest priority
   - Key settings:
     - ARK_MODEL: LLM model name (default: "gpt-4")
     - ARK_TEMPERATURE: LLM temperature (default: 0.7)
     - ARK_MAX_TOKENS: max output tokens (default: 4096)
     - ARK_SANDBOX_TYPE: "auto" | "docker" | "local"
     - ARK_MEMORY_KEY: master encryption key for memory
     - ARK_DEBUG: enable debug logging
   - `config = ArkConfig()` — singleton instance
   - `config.get("key", default=None)` — get a value with precedence chain

TESTING:
`python -c "from src.memory import MemoryManager; print('OK')"` must work.
`python -c "from src.swarm import SwarmOrchestrator; print('OK')"` must work.
`python -c "from src.config import ArkConfig; print('OK')"` must work."""),

    ("AGENT-35: Feature Apps Polish", """You are polishing and completing the Ark feature applications.

TARGET FILES: apps/server.ark + apps/explorer.ark + apps/miner.ark + apps/wallet.ark + apps/sovereign_shell.ark
DO NOT TOUCH: apps/lsp.ark, apps/lsp_main.ark, core/*.rs, meta/*.py, src/*.py, lib/*.ark

S-LANG TRACE: $FeatureApps >> $ArkApplications !! $Server_Explorer_Miner_Wallet_Shell

MISSION: Read each application file and complete/polish it for demonstration quality.

TASKS:

1. apps/server.ark — HTTP Server
   - Should be a working HTTP server that serves static files and handles routes
   - Required routes:
     - GET / -> Returns "Welcome to Ark Server"
     - GET /health -> Returns JSON: {"status": "ok", "version": "0.1.0"}
     - GET /api/time -> Returns current timestamp
     - POST /api/echo -> Returns the request body back
   - Error handling: 404 for unknown routes, 500 for internal errors
   - Logging: print each request method, path, and response status
   - MUST use only intrinsics available in meta/ark_intrinsics.py (net.http_serve, net.http_respond, etc.)

2. apps/explorer.ark — Blockchain Explorer
   - Display blockchain state: chain height, latest blocks, transaction history
   - Functions:
     - `show_chain_info()` — print chain height, total transactions, difficulty
     - `show_block(index)` — print block details: hash, previous_hash, transactions, timestamp
     - `show_balance(address)` — print address balance
     - `search_tx(tx_hash)` — look up a transaction by hash
   - Format output as readable tables/structured text
   - Handle errors: invalid block index, unknown tx hash, etc.

3. apps/miner.ark — Cryptocurrency Miner
   - Implement a mining loop:
     - Collect pending transactions
     - Create a candidate block
     - Run proof-of-work (find valid nonce)
     - Submit the mined block
   - Mining statistics: blocks mined, total time, average block time, hash rate estimate
   - Difficulty display: show current difficulty and target
   - Graceful shutdown: stop mining on ctrl+c equivalent
   - MUST use chain intrinsics: chain.height, chain.submit_tx, chain.verify_tx, etc.

4. apps/wallet.ark — Cryptocurrency Wallet
   - Key management:
     - `wallet.create()` — generate a new keypair, save to file
     - `wallet.load(path)` — load existing wallet from file
     - `wallet.balance()` — check balance for the wallet's address
   - Transaction functions:
     - `wallet.send(to_address, amount)` — create, sign, and submit a transaction
     - `wallet.history()` — show transaction history for this wallet
   - Security: private key stored encrypted (use crypto intrinsics)
   - User-friendly output: format amounts, show confirmations

5. apps/sovereign_shell.ark — Interactive Shell
   - An Ark-based interactive shell/terminal
   - Built-in commands:
     - `help` — show available commands
     - `run <file.ark>` — execute an Ark file
     - `eval <expr>` — evaluate an Ark expression
     - `chain` — show blockchain status
     - `wallet` — show wallet info
     - `peers` — show connected P2P peers (if any)
     - `exit` — exit the shell
   - Command history (if the runtime supports it)
   - Colored prompt: `ark> ` in green

VERIFICATION:
- Each app should parse without errors: `python meta/ark.py parse apps/<file>.ark`
- Each app's logic should be internally consistent
- No undefined function calls — all functions called must be defined or from the standard library"""),

    ("AGENT-36: Docker Deployment", """You are completing the Ark Docker deployment infrastructure.

TARGET FILES: Dockerfile + docker-compose.yml (ONLY these two files at repo root)
DO NOT TOUCH: any source code, tests, or documentation files

S-LANG TRACE: $DockerDeploy >> $Containerization !! $MultiStage_Compose_Security

MISSION: Read the existing Dockerfile and docker-compose.yml, then upgrade them to production quality.

TASKS:

1. MULTI-STAGE DOCKERFILE
```dockerfile
# Stage 1: Rust builder
FROM rust:1.75-slim AS rust-builder
WORKDIR /build
COPY core/ ./core/
COPY Cargo.toml Cargo.lock ./
RUN cd core && cargo build --release

# Stage 2: Python runtime
FROM python:3.11-slim AS runtime
WORKDIR /app

# Install system deps
RUN apt-get update && apt-get install -y --no-install-recommends \\
    && rm -rf /var/lib/apt/lists/*

# Copy Python code
COPY meta/ ./meta/
COPY lib/ ./lib/
COPY apps/ ./apps/
COPY src/ ./src/
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

# Copy Rust binary
COPY --from=rust-builder /build/target/release/ark-core /usr/local/bin/ark-core

# Security: non-root user
RUN useradd -m -s /bin/bash ark
USER ark

# Health check
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \\
    CMD python -c "print('ok')" || exit 1

# Default command
CMD ["python", "meta/ark.py", "repl"]
EXPOSE 8080
```

2. docker-compose.yml — Full stack
```yaml
version: '3.8'
services:
  ark-runtime:
    build: .
    container_name: ark-runtime
    ports:
      - "8080:8080"
    volumes:
      - ./apps:/app/apps:ro
      - ./lib:/app/lib:ro
      - ark-data:/home/ark/.ark
    environment:
      - ARK_SANDBOX_TYPE=local
      - ARK_DEBUG=false
    restart: unless-stopped
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 512M
    networks:
      - ark-net

  ark-server:
    build: .
    container_name: ark-server
    command: ["python", "meta/ark.py", "run", "apps/server.ark"]
    ports:
      - "3000:3000"
    depends_on:
      - ark-runtime
    restart: unless-stopped
    networks:
      - ark-net

volumes:
  ark-data:

networks:
  ark-net:
    driver: bridge
```

3. .dockerignore (CREATE NEW FILE)
```
.git
__pycache__
*.pyc
target/debug
target/release/deps
target/release/build
*.log
*.tmp
node_modules
.vscode
.agent
.antigravity
*.enc
swarm_dispatch_log.json
```

4. SECURITY HARDENING
   - Non-root user (ark) for runtime
   - Read-only filesystem where possible (:ro mounts)
   - No network access by default (can be enabled via compose profile)
   - Resource limits: CPU and memory caps
   - No privileged mode
   - Minimal base image (slim variants)

5. BUILD OPTIMIZATION
   - Layer caching: COPY requirements.txt before source code
   - Multi-stage build: Rust compilation separate from runtime
   - .dockerignore to minimize context size
   - Pin specific base image versions for reproducibility

VERIFICATION:
- `docker build -t ark-compiler .` should complete without errors (or document any expected missing deps)
- `docker-compose config` should validate the compose file
- The Dockerfile should be valid syntax (no YAML errors)"""),

    ("AGENT-37: Contributing & Community Docs", """You are upgrading the Ark project's contributing documentation and community guidelines.

TARGET FILES: CONTRIBUTING.md + CLA.md + THIRD_PARTY_NOTICES.md + docs/USER_MANUAL.md
DO NOT TOUCH: README.md (another agent handles it), source code, or any .rs/.py/.ark files

S-LANG TRACE: $CommunityDocs >> $OpenSource !! $ContribGuide_CLA_Manual

MISSION: Read the existing docs and upgrade them to be welcoming, comprehensive, and professional.

TASKS:

1. CONTRIBUTING.md — Complete Contributor Guide
   Structure:
   - Welcome message and project overview
   - **Getting Started**
     - Prerequisites: Rust 1.75+, Python 3.11+, Git
     - Clone and build instructions
     - Running tests: `python meta/gauntlet.py`
     - Running the REPL: `python meta/repl.py`
   - **Development Workflow**
     - Fork -> Branch -> Code -> Test -> PR
     - Branch naming: `feature/description`, `fix/description`, `docs/description`
     - Commit message format: `[component] Short description` (e.g., `[core] Add list.pop intrinsic`)
   - **Code Style**
     - Rust: `cargo fmt` + `cargo clippy`
     - Python: PEP 8, type hints encouraged
     - Ark: 4-space indentation, `snake_case` for functions, `PascalCase` for structs
   - **Architecture Overview**
     - `meta/` — Python reference runtime (parser, interpreter, intrinsics)
     - `core/` — Rust production runtime (VM, compiler, intrinsics)
     - `lib/std/` — Ark standard library
     - `apps/` — Demo applications
     - `tests/` — Test suite (Gauntlet)
   - **Adding an Intrinsic**
     - Step-by-step guide: 1. Add to ark_intrinsics.py 2. Add to core/src/intrinsics.rs 3. Register in dispatch 4. Add test 5. Update INTRINSIC_PARITY.md
   - **Issue Labels**
     - `good-first-issue`, `help-wanted`, `intrinsic-parity`, `rust`, `python`, `documentation`
   - **Code of Conduct** — link to Contributor Covenant

2. CLA.md — Contributor License Agreement
   - Update with current project details
   - Use a standard Apache-style Individual CLA
   - Include: grant of copyright license, grant of patent license, representations
   - Simple sign-off process: "By submitting this PR, I agree to the CLA"

3. THIRD_PARTY_NOTICES.md
   - List ALL third-party dependencies with their licenses:
     - Python: lark (MIT), z3-solver (MIT), requests (Apache 2.0)
     - Rust: sha2 (MIT/Apache), hmac (MIT/Apache), aes-gcm (MIT/Apache), ed25519-dalek (BSD), etc.
   - Format: Name | Version | License | URL
   - Add a note about license compatibility

4. docs/USER_MANUAL.md — Updated User Manual
   - **Installation**: from source, from Docker, binary releases
   - **Quick Start**: Hello World, variables, functions, control flow
   - **Language Reference**: all syntax elements with examples
   - **Standard Library**: module overview with common functions
   - **Intrinsics Reference**: categorized list (sys, crypto, math, net, chain)
   - **Configuration**: env variables (ARK_EXEC_TIMEOUT, ARK_CAPABILITIES, etc.)
   - **FAQ**: common errors and solutions

VERIFICATION: All files should be well-formatted markdown with no broken links or missing sections."""),
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
    print(f"[{i}/7] {title}...", end=" ", flush=True)
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
print(f"WAVE 4 DEPLOYED: {ok}/7 agents")
with open("swarm_wave4_log.json", "w") as f:
    json.dump(results, f, indent=2)
print(f"Log: swarm_wave4_log.json")
