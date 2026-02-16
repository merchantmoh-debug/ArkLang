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

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
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
*   **Parity:** 105/105 Intrinsics ported to native Rust.
*   **Performance:** `sys.network`, `sys.fs`, `sys.crypto` run at native speeds.
*   **Safety:** Memory safety enforced by Rust's ownership model + Ark's Linear Checker.

### 2. Linear Type System (`core/src/checker.rs`)
Ark treats sensitive data (Money, Sockets, File Handles) as "Linear Resources".
*   **No GC:** Resources must be used exactly once.
*   **No Leaks:** Dropping a linear variable without consumption causes a **Compile-Time Error**.
*   **No Double-Spend:** Passed variables are moved, not copied.

### 3. Neuro-Symbolic Intrinsics (`core/src/intrinsics.rs`)
AI calls are treated as standard compiler intrinsics (`sys.ask_ai`), allowing future optimizations like caching, batching, and formal verification of outputs.

---

## 💎 The Hidden Gems (Advanced Implementation)

Beyond the basic examples, the Standard Library and specific Apps demonstrate advanced capabilities.

### 1. Pure Ark Cryptography (`lib/wallet_lib.ark`)
**Status:** `🟢 PRODUCTION`
A full implementation of the **Secp256k1** Elliptic Curve and **BIP39** Mnemonic generation written entirely in Ark.
*   **Features:** Point Addition, Point Doubling, Scalar Multiplication, PBKDF2-HMAC-SHA512.
*   **Significance:** Proves Ark can handle complex mathematical operations without relying on C bindings.

### 2. Self-Hosting Parser (`apps/lsp.ark`)
**Status:** `🟡 BETA`
A 1000+ line Recursive Descent Parser and Lexer for the Ark language, written in Ark.
*   **Features:** Tokenizes source code, builds AST nodes, reports range-based errors.
*   **Significance:** Demonstrates language self-sufficiency and complex data structure handling (recursive structs/lists).

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

### Running Examples

**1. The Snake Game (Live App):**
A fully functional Snake game written in Ark.
```bash
# Start the Snake Server
python3 apps/sovereign.py run examples/snake.ark
# Open http://localhost:8000 in your browser
```

**2. Market Maker (Heavyweight Financial Logic):**
A High-Frequency Trading bot simulation demonstrating Linear Types and Event Loops.
```bash
python3 apps/sovereign.py run apps/market_maker.ark
```

**3. Wallet CLI (Pure Ark Crypto):**
```bash
python3 apps/sovereign.py run apps/wallet.ark create "mypassword"
```

---

## 📂 Project Structure

| Directory | Description | Maturity |
| :--- | :--- | :--- |
| `core/` | **Rust Runtime & Intrinsics.** The engine. | 🟢 STABLE |
| `lib/std/` | **Standard Library.** (`math.ark`, `net.ark`, `io.ark`). | 🟢 STABLE |
| `lib/wallet_lib.ark` | **Crypto Library.** Secp256k1/BIP39 implementation. | 🟢 STABLE |
| `apps/lsp.ark` | **Language Server.** Self-hosted Parser/Lexer. | 🟡 BETA |
| `apps/server.ark` | **HTTP Server.** Functional web server. | 🟡 BETA |
| `meta/` | **Tooling.** Python-based JIT, Security Scanner, Swarm Bridge. | 🟡 BETA |
| `tests/` | **Test Suite.** Feature tests. | 🟢 PASSING |

---

## 🛡️ Security Model

Ark uses a **Capability-Token System** (`ARK_CAPABILITIES`) to sandbox execution.
*   **Default:** Safe Mode (No IO/Net).
*   **Dev Mode:** `export ARK_CAPABILITIES="exec,net,fs_write,fs_read,thread,ai"`

**Security Scanner:**
The JIT engine includes a static analysis pass (`meta/ark_security.py`) that scans for:
*   SQL/Command Injection patterns.
*   Path Traversal vulnerabilities.
*   Hardcoded Secrets.

---

## 📜 License
MIT License. Open Verification. Sovereign Code.
