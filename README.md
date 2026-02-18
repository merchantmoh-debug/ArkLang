<div align="center">

<pre>
    █████╗ ██████╗ ██╗  ██╗    ██████╗ ██████╗ ██╗███╗   ███╗███████╗
   ██╔══██╗██╔══██╗██║ ██╔╝    ██╔══██╗██╔══██╗██║████╗ ████║██╔════╝
   ███████║██████╔╝█████╔╝     ██████╔╝██████╔╝██║██╔████╔██║█████╗  
   ██╔══██║██╔══██╗██╔═██╗     ██╔═══╝ ██╔══██╗██║██║╚██╔╝██║██╔══╝  
   ██║  ██║██║  ██║██║  ██╗    ██║     ██║  ██║██║██║ ╚═╝ ██║███████╗
   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝    ╚═╝     ╚═╝  ╚═╝╚═╝╚═╝     ╚═╝╚══════╝

           THE ARK COMPILER v112.0 (PRIME)
           -------------------------------
           System: Linear Type System & Neuro-Symbolic Intrinsic Engine
</pre>

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![License: Commercial](https://img.shields.io/badge/License-Commercial-blue.svg)](LICENSE_COMMERCIAL)
![Status](https://img.shields.io/badge/Status-BETA-yellow?style=for-the-badge)
![Security](https://img.shields.io/badge/Security-LINEAR_TYPES-blue?style=for-the-badge)
![Core](https://img.shields.io/badge/Core-RUST-red?style=for-the-badge)
![Parity](https://img.shields.io/badge/Rust_Parity-100%25-green?style=for-the-badge)

</div>

---

# Ark Compiler (Technical Preview)

**Ark** is a neuro-symbolic programming language designed for **Sovereign Computation**. It combines a strictly typed **Rust Core** with a **Linear Type System** to enforce resource safety without Garbage Collection.

> **Philosophical Manifesto:** For the project's vision, philosophy, and "Omega Point" doctrine, see [docs/MANIFESTO.md](docs/MANIFESTO.md).

---

## 🚀 Key Technical Features

### 1. 100% Rust Core (`core/`)

The runtime is built on a high-performance Rust foundation (`1.93-slim`).

* **Parity:** 106/106 Python intrinsics ported to Rust (108 total including Rust-only additions).
* **Performance:** `sys.network`, `sys.fs`, `sys.crypto` run at native speeds.
* **Safety:** Memory safety enforced by Rust's ownership model + Ark's Linear Checker.

### 2. Linear Type System (`core/src/checker.rs`)

Ark treats sensitive data (Money, Sockets, File Handles) as "Linear Resources".

* **No GC:** Resources must be used exactly once.
* **No Leaks:** Dropping a linear variable without consumption causes a **Compile-Time Error**.
* **No Double-Spend:** Passed variables are moved, not copied.

### 3. Neuro-Symbolic Intrinsics (`core/src/intrinsics.rs`)

AI calls are treated as standard compiler intrinsics (`sys.ask_ai`), allowing future optimizations like caching, batching, and formal verification of outputs.

---

## 💎 The Hidden Gems (Advanced Implementation)

Beyond the basic examples, the Standard Library and specific Apps demonstrate advanced capabilities.

### 1. Pure Ark Cryptography (`lib/wallet_lib.ark`)

**Status:** `🟢 PRODUCTION`
A full implementation of the **Secp256k1** Elliptic Curve and **BIP39** Mnemonic generation written entirely in Ark.

* **Features:** Point Addition, Point Doubling, Scalar Multiplication, PBKDF2-HMAC-SHA512.
* **Significance:** Proves Ark can handle complex mathematical operations without relying on C bindings.

### 2. Self-Hosting Parser (`apps/lsp.ark`)

**Status:** `🟡 BETA`
A 1000+ line Recursive Descent Parser and Lexer for the Ark language, written in Ark.

* **Features:** Tokenizes source code, builds AST nodes, reports range-based errors.
* **Significance:** Demonstrates language self-sufficiency and complex data structure handling (recursive structs/lists).

---

## 🤖 Ark Agent Framework (`src/`)

**No other programming language ships with a built-in multi-agent AI system.** Ark does.

The Agent Framework is a full-stack orchestration layer that lets Ark programs spawn, coordinate, and sandbox AI agents — with zero external dependencies beyond an LLM backend.

### Architecture

```text
Task → RouterAgent → [CoderAgent | ResearcherAgent | ReviewerAgent] → Review → Result
                          ↕ execute_ark / compile_check
                     Ark Compiler (meta/ark.py, core/)
```

| Component | What It Does |
| :--- | :--- |
| **AgentOrchestrator** | Pipeline executor: route tasks to specialists, auto-review code changes |
| **SwarmOrchestrator** | Multi-agent coordination with 4 strategies: `router`, `broadcast`, `consensus`, `pipeline` |
| **MCP Client** | [Model Context Protocol](https://modelcontextprotocol.io/) — JSON-RPC 2.0 over Stdio/HTTP/SSE. Connect any MCP-compatible tool server. |
| **Encrypted Memory** | Fernet-encrypted namespaced storage + TF-IDF vector search for semantic recall |
| **Local Sandbox** | AST-level security analysis — bans dangerous imports, functions, and attributes before execution |
| **Docker Sandbox** | Container-isolated code execution for untrusted workloads |

### Specialist Agents

| Agent | Role |
| :--- | :--- |
| `RouterAgent` | Classifies tasks and delegates to the right specialist |
| `CoderAgent` | Generates, modifies, and refactors code — **Ark-aware** with `execute_ark()` and `compile_check()` tools |
| `ResearcherAgent` | Analyzes codebases and gathers context |
| `ReviewerAgent` | Audits code changes for bugs, security issues, and style |

### LLM Backend Support

The framework is backend-agnostic. Configure via environment variables:

* **Google Gemini:** `ARK_GOOGLE_API_KEY`
* **OpenAI / Compatible:** `ARK_OPENAI_BASE_URL` + `ARK_OPENAI_API_KEY`
* **Ollama (Local):** Point `ARK_OPENAI_BASE_URL` to `http://localhost:11434`

### Quick Start

```bash
# Run the agent orchestrator
python -m src.agent "Write a Python script that sorts a CSV by the second column"

# Run the swarm (multi-agent)
python -m src.swarm_demo
```

> **Full Agent Framework guide:** [User Manual — Agent Framework](docs/USER_MANUAL.md#17-agent-framework)

### Ark-Native AI (The Neural Bridge)

Ark is the **only programming language with built-in AI intrinsics**. Call AI directly from `.ark` code:

```ark
// Direct AI call
answer := sys.ai.ask("What is the capital of France?")
print(answer)

// Agent with persona
sys.vm.source("lib/std/ai.ark")
coder := Agent.new("You are a Rust expert.")
response := coder.chat("Explain ownership.")

// Multi-agent swarm
swarm := Swarm.new([coder, Agent.new("You are a code reviewer.")])
results := swarm.run("Write a sort function")
```

> **Configuration:** Set `GOOGLE_API_KEY` or `ARK_LLM_ENDPOINT` (e.g., `http://localhost:11434/v1/chat/completions` for Ollama).

---

## 🛠️ Quick Start

### Installation

```bash
# Clone the Repository
git clone https://github.com/merchantmoh-debug/ark-compiler.git
cd ark-compiler

# Build Docker Container (Recommended)
docker build -t ark-compiler .

# Run Interactive Shell
docker run -it --rm ark-compiler
```

### Local Development (Without Docker)

```bash
# Install Python dependencies
pip install -r requirements.txt

# Install Rust toolchain (if building from source)
cargo build --release
```

### Running Examples

**1. The Snake Game (Live App):**
A fully functional Snake game written in Ark.

```bash
# Start the Snake Server
python3 meta/ark.py run examples/snake.ark
# Open http://localhost:8000 in your browser
```

**2. Market Maker (Heavyweight Financial Logic):**
A High-Frequency Trading bot simulation demonstrating Linear Types and Event Loops.

```bash
python3 meta/ark.py run apps/market_maker.ark
```

**3. Wallet CLI (Pure Ark Crypto):**

```bash
python3 meta/ark.py run apps/wallet.ark create "mypassword"
```

---

## 📖 Learn Ark

New to Ark? Start here:

| Document | Description |
| :--- | :--- |
| **[User Manual](docs/USER_MANUAL.md)** | **Complete language guide** — variables, functions, control flow, imports, stdlib, crypto, blockchain, AI, and more. |
| **[Quick Start](docs/QUICK_START.md)** | 5-minute setup and Hello World. |
| **[API Reference](docs/API_REFERENCE.md)** | All 106 built-in intrinsics with signatures and examples. |
| **[Stdlib Reference](docs/STDLIB_REFERENCE.md)** | Documentation for all 12 standard library modules. |
| **[Manifesto](docs/MANIFESTO.md)** | Why Ark exists — the design philosophy. |

---

## 📂 Project Structure

| Directory | Description | Maturity |
| :--- | :--- | :--- |
| `core/` | **Rust Runtime & Intrinsics.** The engine. | 🟢 STABLE |
| `lib/std/` | **Standard Library.** 12 modules (`math`, `net`, `io`, `crypto`, `chain`, etc.). | 🟢 STABLE |
| `lib/wallet_lib.ark` | **Crypto Library.** Secp256k1/BIP39 implementation. | 🟢 STABLE |
| `apps/lsp.ark` | **Language Server.** Self-hosted Parser/Lexer. | 🟡 BETA |
| `apps/server.ark` | **HTTP Server.** Functional web server. | 🟡 BETA |
| `src/` | **Agent Framework.** Multi-agent orchestration, MCP client, sandboxed execution, encrypted memory. | 🟡 BETA |
| `meta/` | **Tooling.** Python-based JIT, Security Scanner, Gauntlet runner. | 🟡 BETA |
| `docs/` | **Documentation.** API Reference, Stdlib Reference, Manifesto. | 🟢 STABLE |
| `site/` | **Web Assets.** Landing page, WASM test harness. | 🟡 BETA |
| `tests/` | **Test Suite.** 100+ feature tests (Gauntlet runner). | 🟢 PASSING |

---

## 🛡️ Security Model

Ark uses a **Capability-Token System** (`ARK_CAPABILITIES`) to sandbox execution.

* **Default:** Safe Mode (No IO/Net).
* **Dev Mode:** `export ARK_CAPABILITIES="exec,net,fs_write,fs_read,thread,ai"`

**Security Scanner:**
The JIT engine includes a static analysis pass (`meta/ark_security.py`) that scans for:

* SQL/Command Injection patterns.
* Path Traversal vulnerabilities.
* Hardcoded Secrets.

---

## 📜 License

Dual Licensed: AGPL v3 (Open Source) or Commercial (Sovereign Systems).
