<div align="center">

<pre>
    █████╗ ██████╗ ██╗  ██╗
   ██╔══██╗██╔══██╗██║ ██╔╝
   ███████║██████╔╝█████╔╝
   ██╔══██║██╔══██╗██╔═██╗
   ██║  ██║██║  ██║██║  ██╗
   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝
</pre>

**A general-purpose language with linear types, formal verification, and built-in AI.**

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

</div>

## Install

```bash
git clone https://github.com/merchantmoh-debug/ArkLang.git
cd ArkLang
pip install -r requirements.txt
```

## Hello World

```ark
print("Hello, World!")
```

```bash
python3 meta/ark.py run hello.ark
```

> **Also works with Docker:** `docker build -t ark . && docker run -it --rm ark`
>
> **Also compiles to WASM:** `cd core && cargo build --release` for the Rust compiler, then `ark build hello.ark` for native `.wasm` output.

**[Docs](docs/USER_MANUAL.md)** | **[Quick Start](docs/QUICK_START.md)** | **[API Reference](docs/API_REFERENCE.md)** | **[Playground](https://merchantmoh-debug.github.io/ArkLang/site/leviathan/)** | **[Manifesto](docs/MANIFESTO.md)**

---

## The Language

Ark is a general-purpose, dynamically typed language with first-class functions, closures, algebraic types, and a linear type system. It compiles to bytecode (VM), native WASM, or runs via tree-walking interpreter.

### Variables & Functions

```ark
name := "Ark"
pi := 3.14159

func factorial(n) {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambdas
double := |x| { x * 2 }
print(double(21))  // 42
```

### Enums & Pattern Matching

```ark
enum Shape {
    Circle(Float),
    Rectangle(Float, Float),
    Point
}

func area(s) {
    match s {
        Shape.Circle(r)       => return 3.14159 * r * r
        Shape.Rectangle(w, h) => return w * h
        Shape.Point           => return 0.0
    }
}

print(area(Shape.Circle(5.0)))  // 78.53975
```

### Traits & Impl Blocks

```ark
trait Drawable {
    func draw(self) -> Unit
    func area(self) -> Float
}

impl Drawable for Circle {
    func draw(self) -> Unit {
        print("Drawing circle with radius " + str(self.radius))
    }
    func area(self) -> Float {
        return 3.14159 * self.radius * self.radius
    }
}
```

### Linear Types

Resources that behave like physical matter -- they cannot be copied, cannot be leaked, and must be consumed exactly once:

```ark
func transfer(coin: Linear<Coin>, recipient: Address) {
    // 'coin' is MOVED here. The caller can never touch it again.
    // Double-spend? COMPILE ERROR.
    // Forgot to use it? COMPILE ERROR.
}
```

The linear type checker (`checker.rs`, 1,413 LOC) enforces this at compile time. No double-spend, no use-after-free, no resource leaks -- by construction, not convention.

### Built-in AI

```ark
// AI is a first-class intrinsic -- no SDK, no import
answer := sys.ai.ask("Explain linear types in 3 sentences.")
print(answer)
```

Set `GOOGLE_API_KEY` for Gemini, or point at a local Ollama instance. Without a key, AI calls return a graceful fallback.

---

## Standard Library

| Module | Purpose | Key Functions |
| --- | --- | --- |
| `math` | Mathematics | `sqrt`, `sin`, `cos`, `pow`, `abs`, `ln`, `exp`, `random` |
| `string` | String utilities | `length`, `upper`, `lower`, `split`, `join`, `replace` |
| `io` | Console I/O | `read_line`, `write` |
| `fs` | File system | `read`, `write`, `exists`, `size`, `read_bytes` |
| `net` | HTTP networking | `http_get`, `http_post` |
| `crypto` | Cryptography | `sha256`, `sha512`, `hmac`, `aes_encrypt`, `uuid` |
| `chain` | Blockchain | `height`, `balance`, `submit_tx`, `get_block` |
| `time` | Date/time | `now`, `sleep`, `format`, `elapsed` |
| `event` | Event system | `poll`, `push` |
| `result` | Error handling | `ok`, `err`, `is_ok`, `unwrap` |
| `audio` | Audio playback | `play`, `stop` |
| `ai` | AI/LLM agents | `ask`, `Agent.new`, `Agent.chat`, `Swarm.run` |
| `persistent` | Immutable data | `PVec`, `PMap` (trie + HAMT) |
| `gcd` | Data integrity | `evaluate`, `audit_dataset`, `decorrelate` |

---

## CLI

```bash
ark run <file.ark>         # Run source or MAST JSON
ark build <file.ark>       # Compile to native .wasm binary
ark run-wasm <file.wasm>   # Execute compiled WASM via wasmtime
ark check <file.ark>       # Static linear type checker
ark diagnose <file.ark>    # Diagnostic proof suite (cryptographic verification)
ark parse <file.ark>       # Dump AST as JSON
ark debug <file.ark>       # Interactive step-through debugger
ark repl                   # Interactive REPL
ark wit <file.ark>         # Generate WIT interface definition
ark adn <file.ark>         # Run and output in ADN format
```

---

## Compiler Architecture

Ark has three backends, all fully functional:

| Backend | Purpose |
| --- | --- |
| **Bytecode VM** | `compiler.rs` + `vm.rs` -- fast bytecode compilation and execution |
| **Native WASM** | `wasm_codegen.rs` (3,865 LOC) -- compiles Ark to standalone `.wasm` binaries |
| **Tree-walker** | `eval.rs` -- interpreter (used for testing and REPL) |

The Rust compiler (`core/`) contains 62 source files. The Python meta-interpreter (`meta/`) is the reference implementation and includes Z3 integration.

---

## Z3 Formal Verification

Ark integrates with [Microsoft's Z3 SMT solver](https://github.com/Z3Prover/z3) for constraint verification. The runtime calls `sys.z3.verify()` which invokes Z3 via [`z3_bridge.py`](meta/z3_bridge.py).

The [**Leviathan Portal**](https://merchantmoh-debug.github.io/ArkLang/site/leviathan/) demonstrates this: it Z3-verifies thermodynamic constraints, then CSG-compiles a titanium metamaterial heat sink using `manifold-3d` WASM -- entirely in the browser.

> The browser demo simulates the Z3 step client-side (Z3 is a native C++ library that cannot compile to WASM). The full Z3 solver runs via the native Ark runtime: `python3 meta/ark.py run apps/leviathan_compiler.ark`

**[Try the demo](https://merchantmoh-debug.github.io/ArkLang/site/leviathan/)** | **[Read the source](apps/leviathan_compiler.ark)** (210 lines of Ark)

---

## Cryptography

Core primitives implemented in Rust without OpenSSL:

| Primitive | Status |
| --- | --- |
| SHA-256, SHA-512, Double SHA-256 | Done |
| HMAC-SHA256, HMAC-SHA512 | Done |
| BIP-32 HD Key Derivation | Done |
| Ed25519 Sign/Verify | Done (via `ed25519-dalek`) |
| Wallet Address Generation | Done (`ark:` prefix, checksum) |
| Merkle Root Computation | Done |
| Secure Random | Done (`/dev/urandom`) |

---

## Blockchain & Governance

**Blockchain** (338 LOC): Full Proof-of-Work chain with transactions, blocks, Merkle roots, chain validation, balance tracking, difficulty adjustment, and code submission.

**Governance Engine** (839 LOC): 5-phase pipeline (Sense -> Assess -> Decide -> Action -> Verify) with HMAC-signed audit receipts and Monotone Confidence Constraint enforcement.

---

## AI Agent Framework

Ark ships with a multi-agent system as a core feature, not a plugin:

```text
Task -> RouterAgent -> [CoderAgent | ResearcherAgent | ReviewerAgent] -> Result
```

- **4 Specialist Agents** with swarm strategies (broadcast, consensus, pipeline)
- **MCP Client** (JSON-RPC 2.0 over Stdio/HTTP/SSE) for tool integration
- **Encrypted Memory** with TF-IDF semantic recall
- **LLM Backends**: Gemini, OpenAI, Ollama (auto-fallback)
- **Sandbox-first execution**: AST analysis + Docker isolation for untrusted code

The agent framework is backed by a **26-module Rust-native substrate** (~13,350 LOC) providing taint tracking, capability tokens, shell injection detection, Ed25519 manifest signing, 130+ model catalog, A2A protocol, semantic memory with confidence decay, and lifecycle hooks.

```ark
// Multi-agent swarm from Ark code
sys.vm.source("lib/std/ai.ark")
coder := Agent.new("You are a Rust expert.")
reviewer := Agent.new("You are a security auditor.")
swarm := Swarm.new([coder, reviewer])
results := swarm.run("Build a key-value store")
```

---

## Diagnostic Proof Suite

Ark can produce cryptographic evidence that the compiler verified your code correctly:

```bash
ark diagnose app.ark                  # Summary + pass/fail
ark diagnose app.ark --tier pro       # Full Merkle-rooted, HMAC-signed proof bundle
ark diagnose app.ark --json           # JSON output for CI/CD
```

The proof bundle includes source hashes, MAST roots, 15 quality gate scores, a Merkle root, and an HMAC signature. Suitable for SOC 2 compliance, smart contract verification, and supply chain attestation.

---

## Data Integrity (GCD Kernel)

The `gcd` standard library module implements the Tier-1 Kernel from Clement Paulus's [Generative Collapse Dynamics](https://doi.org/10.5281/zenodo.18819238) framework. It uses the AM-GM inequality to detect weak data channels hidden by healthy-looking averages:

```ark
import lib.std.gcd

// Halt if the gap between arithmetic and geometric mean exceeds threshold
gcd.audit_dataset(training_features, weights, 2000)
```

The `Censored` type is enforced at the language level -- any arithmetic on missing data raises `CensoredAccessError`. Missing data cannot be silently averaged away.

> **Credit:** GCD/UMCP theory by [Clement Paulus](https://orcid.org/0009-0000-6069-8234) (CC BY 4.0).

---

## Security Model

| Feature | Details |
| --- | --- |
| **Default** | Air-gapped -- no network, no filesystem writes, no shell |
| **Capability Tokens** | `ARK_CAPABILITIES="net,fs_read,fs_write,ai"` |
| **Static Analysis** | Security scanner catches injection, path traversal, hardcoded secrets |
| **Import Security** | Path traversal blocked with `RuntimeError::UntrustedCode` |
| **Agent Sandbox** | AST analysis + Docker isolation for untrusted workloads |
| **Epistemic Firewall** | `Censored` sentinel blocks arithmetic on missing data |

---

## Documentation

| Document | Description |
| --- | --- |
| **[User Manual](docs/USER_MANUAL.md)** | Complete language guide (1,500+ lines) |
| **[Quick Start](docs/QUICK_START.md)** | 5-minute setup |
| **[API Reference](docs/API_REFERENCE.md)** | All 109 intrinsics with signatures and examples |
| **[Stdlib Reference](docs/STDLIB_REFERENCE.md)** | All 16 standard library modules |
| **[Language Spec](docs/ARK_LANGUAGE_SPEC.md)** | Formal specification |
| **[Manifesto](docs/MANIFESTO.md)** | The philosophy behind Ark |

---

## License

Dual Licensed: **AGPL v3** (Open Source) or **Commercial**.

**GCD/UMCP Attribution:** The `lib/std/gcd.ark` module implements theory from Clement Paulus, *"GCD: Enabling Cross-Domain Comparability via Contract-Frozen Kernel Invariants and Typed Return"* (v2.1.3, February 2026). [DOI: 10.5281/zenodo.18819238](https://doi.org/10.5281/zenodo.18819238). Licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
