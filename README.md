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

**Ark** is a programming language where **resource safety is a compile-time guarantee**, not a runtime hope. It combines a high-performance **Rust core** with a **Linear Type System** that eliminates garbage collection, prevents double-spends, and catches resource leaks before your code ever runs.

**Use it for:** Financial systems, cryptographic protocols, AI-native applications, and anywhere resource correctness is non-negotiable.

> **Philosophical Manifesto:** For the project's vision and design philosophy, see [docs/MANIFESTO.md](docs/MANIFESTO.md).
> Play the Ark Snake Game (Coded in Ark - Demonstration of the Language's functionality) - https://merchantmoh-debug.github.io/ark-compiler/

---

## 🚀 Key Technical Features

### 1. 100% Rust Core (`core/`)

The runtime is built on a high-performance Rust foundation (`1.93-slim`).

* **Parity:** 107/107 Python intrinsics ported to Rust (109 total including Rust-only additions).
* **Performance:** `sys.network`, `sys.fs`, `sys.crypto` run at native speeds.
* **Safety:** Memory safety enforced by Rust's ownership model + Ark's Linear Checker.

### 2. Linear Type System (`core/src/checker.rs`)

Ark treats sensitive data (Money, Sockets, File Handles) as "Linear Resources".

* **No GC:** Resources must be used exactly once.
* **No Leaks:** Dropping a linear variable without consumption causes a **Compile-Time Error**.
* **No Double-Spend:** Passed variables are moved, not copied.

### 3. Neuro-Symbolic Intrinsics (`core/src/intrinsics.rs`)

AI calls are treated as standard compiler intrinsics (`sys.ai.ask`), allowing future optimizations like caching, batching, and formal verification of outputs.

---

## 💎 Advanced Capabilities

Beyond the basics, the Standard Library and featured apps demonstrate production-grade capabilities.

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

Ark ships with a multi-agent AI orchestration layer — spawn, coordinate, and sandbox AI agents from Ark code or the CLI.

```text
Task → RouterAgent → [CoderAgent | ResearcherAgent | ReviewerAgent] → Review → Result
                          ↕ execute_ark / compile_check
                     Ark Compiler (meta/ark.py, core/)
```

* **4 Specialist Agents:** Router, Coder (Ark-aware), Researcher, Reviewer
* **Swarm Orchestration:** `router`, `broadcast`, `consensus`, `pipeline` strategies
* **MCP Client:** JSON-RPC 2.0 over Stdio/HTTP/SSE
* **Security:** AST-level sandboxing + Docker isolation for untrusted workloads
* **Memory:** Fernet-encrypted storage + TF-IDF semantic recall
* **Backend-agnostic:** Gemini, OpenAI, Ollama — configure via env vars

```bash
# Run the agent orchestrator
python -m src.agent "Write a Python script that sorts a CSV by the second column"
```

> **Full guide:** [User Manual — Agent Framework](docs/USER_MANUAL.md#17-agent-framework)

### Ark-Native AI Intrinsics

Call AI directly from `.ark` code — built-in intrinsics, no external SDK required:

```ark
// Direct AI call
answer := sys.ai.ask("What is the capital of France?")
print(answer)

// Agent with persona + conversation history
sys.vm.source("lib/std/ai.ark")
coder := Agent.new("You are a Rust expert.")
response := coder.chat("Explain ownership.")

// Multi-agent swarm
swarm := Swarm.new([coder, Agent.new("You are a code reviewer.")])
results := swarm.run("Write a sort function")
```

> **Configuration:** Set `GOOGLE_API_KEY` or `ARK_LLM_ENDPOINT`. No key? AI degrades gracefully.

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

**1. Wallet CLI (Pure Ark Crypto):**
Secp256k1 + BIP39 — zero C bindings, 100% Ark.

```bash
python3 meta/ark.py run apps/wallet.ark create "mypassword"
```

**2. Market Maker (Linear Types in Action):**
HFT bot simulation — Linear Types enforce that positions are never double-counted.

```bash
python3 meta/ark.py run apps/market_maker.ark
```

**3. Snake Game (Live Web App):**
A fully functional game served over HTTP, written in Ark.

```bash
python3 meta/ark.py run examples/snake.ark
# Open http://localhost:8000 in your browser
```

---

## 📖 Learn Ark

New to Ark? Start here:

| Document | Description |
| :--- | :--- |
| **[User Manual](docs/USER_MANUAL.md)** | **Complete language guide** — variables, functions, control flow, imports, stdlib, crypto, blockchain, AI, and more. |
| **[Quick Start](docs/QUICK_START.md)** | 5-minute setup and Hello World. |
| **[API Reference](docs/API_REFERENCE.md)** | All 109 built-in intrinsics with signatures and examples. |
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
