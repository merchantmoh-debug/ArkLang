<div align="center">

<pre>
    █████╗ ██████╗ ██╗  ██╗    ██████╗ ██████╗ ██╗███╗   ███╗███████╗
   ██╔══██╗██╔══██╗██║ ██╔╝    ██╔══██╗██╔══██╗██║████╗ ████║██╔════╝
   ███████║██████╔╝█████╔╝     ██████╔╝██████╔╝██║██╔████╔██║█████╗  
   ██╔══██║██╔══██╗██╔═██╗     ██╔═══╝ ██╔══██╗██║██║╚██╔╝██║██╔══╝  
   ██║  ██║██║  ██║██║  ██╗    ██║     ██║  ██║██║██║ ╚═╝ ██║███████╗
   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝    ╚═╝     ╚═╝  ╚═╝╚═╝╚═╝     ╚═╝╚══════╝

            THE ARK COMPILER
            ─────────────────────────────────
            Resource-safe. Formally verified. Compiles to WASM.
</pre>

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![License: Commercial](https://img.shields.io/badge/License-Commercial-blue.svg)](LICENSE_COMMERCIAL)
![CI](https://img.shields.io/badge/CI-10/10_PASSING-brightgreen?style=for-the-badge)
![Tests](https://img.shields.io/badge/Tests-744_Passing-brightgreen?style=for-the-badge)
![Core](https://img.shields.io/badge/Core-RUST-red?style=for-the-badge)
![Parity](https://img.shields.io/badge/Intrinsic_Parity-100%25-green?style=for-the-badge)

</div>

---

## What Is Ark?

**Ark** is a programming language where **resource safety is a compile-time guarantee**, not a runtime hope. It features **enums, traits, impl blocks, pattern matching, lambdas**, a dual-backend compiler (VM + native WASM), a linear type system, a **built-in diagnostic proof suite** with cryptographic verification, 109 built-in intrinsics, a blockchain layer, a governance engine, an AI agent framework with a 26-module Rust-native agent substrate, a parametric manufacturing compiler, and a browser-based playground.

**Use it for:** Financial systems, cryptographic protocols, AI-native applications, smart contracts, compliance-audited systems, and anywhere resource correctness is non-negotiable.

> **[Manifesto](docs/MANIFESTO.md)** -- Why Ark exists.
> **[User Manual](docs/USER_MANUAL.md)** -- Complete language guide.
> **[Leviathan Portal](https://merchantmoh-debug.github.io/ArkLang/site/leviathan/)** -- Z3-verify and CSG-compile a titanium metamaterial heat sink in your browser.

---

## At a Glance

| Metric | Count |
|---|---|
| Rust source files | 59 |
| Total Rust LOC | 40,000+ |
| Total Python LOC | 92,000+ |
| Built-in intrinsics | 109 (100% Python↔Rust parity) |
| CLI subcommands | 10 |
| Standard library modules | 16 |
| Core Rust modules | 58 (compiler, VM, WASM, security, LLM, agent substrate, memory) |
| Agent substrate modules | 26 (taint, capability, metering, triggers, semantic memory, A2A, lifecycle hooks, etc.) |
| Unit tests (all passing) | 744 |
| CI jobs (all green) | 10/10 |
| Compilation backends | 3 (Bytecode VM, Native WASM, Tree-walker) |
| User manual | 1,500+ lines |
| Leviathan Portal | Live in-browser CSG via manifold-3d WASM |

---

## Leviathan: Parametric Manufacturing Compiler

Ark includes a parametric manufacturing compiler that outputs printer-ready geometry from constraint specifications.

The [**Leviathan Portal**](https://merchantmoh-debug.github.io/ArkLang/site/leviathan/) is a zero-installation browser demo of Ark's parametric manufacturing compiler. Click one button and watch Ark:

1. **Z3-verify** 11 thermodynamic constraints -- wall thickness, porosity, thermal conductivity, structural integrity -- rejecting any design that violates physics *before* a single vertex is generated.
2. **CSG-compile** a titanium metamaterial heat sink via `manifold-3d` WASM -- real constructive solid geometry: a 100mm cube minus up to 972 intersecting cylindrical channels, computed as boolean algebra.
3. **Export a printer-ready GLB** -- a watertight, 2-manifold mesh that loads directly into SLS titanium slicer software. Not a mockup. Not a render. An actual manufacturing specification.
4. **Seal it with a cryptographic proof-of-matter receipt** -- SHA-256 hash of the mesh topology, proving the geometry was produced by a verified compilation.

Runtime: ~12ms in a browser tab, zero installation required.

### What This Replaces

The traditional workflow for producing the same output:
- **SolidWorks/Fusion 360** ($5k–$50k/seat/year) -- engineer manually models geometry
- **ANSYS/Abaqus** ($50k–$200k/year) -- run thermal FEA to validate constraints
- **Iterate** 5–15 times over days to weeks when constraints fail
- Export STL, send to print bureau, hope it works

Ark collapses this entire pipeline into a single compilation step. The constraint specification IS the design. The compiler proves correctness and outputs the only geometry that satisfies it -- in one pass, in milliseconds.

**→ [Try it now](https://merchantmoh-debug.github.io/ArkLang/site/leviathan/)** | **[Read the source](apps/leviathan_compiler.ark)** (210 lines of Ark)

---

## Language Features

### Core Language

Ark is a general-purpose language with first-class functions, closures, algebraic types, and a linear type system.

```ark
// Variables
name := "Ark"
pi := 3.14159
items := [1, "two", true, null]

// Functions (first-class, recursive, higher-order)
func factorial(n) {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambdas
double := |x| { x * 2 }
print(double(21))  // 42

// For loops, while loops, break, continue
for item in items {
    print(item)
}
```

### Enums & Pattern Matching

Full algebraic data types with destructuring pattern matching:

```ark
enum Shape {
    Circle(Float),
    Rectangle(Float, Float),
    Point
}

let s := Shape.Circle(5.0)

match s {
    Shape.Circle(r)       => print("Circle with radius: " + str(r))
    Shape.Rectangle(w, h) => print("Rectangle: " + str(w) + "x" + str(h))
    Shape.Point           => print("Just a point")
}
```

### Traits & Impl Blocks

Interface-based polymorphism:

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

### Structs

Named, typed structures with field access:

```ark
struct Point {
    x: Float,
    y: Float
}

let p := {x: 1.0, y: 2.0}
p.x := 3.0
```

### Linear Type System

Resources that behave like **physical matter** -- they cannot be copied, cannot be leaked, and must be consumed exactly once:

```ark
// 'coin' is a Linear resource -- the compiler enforces Conservation of Value
func transfer(coin: Linear<Coin>, recipient: Address) {
    // 'coin' is MOVED here. The caller can NEVER touch it again.
    // Double-spend? COMPILE ERROR.
    // Forgot to use it? COMPILE ERROR.
}
```

---

## Compiler Architecture

Ark has **three backends**, all fully functional:

| Backend | File | LOC | Purpose |
|---|---|---|---|
| **Bytecode Compiler** | `compiler.rs` | 906 | Ark → fast bytecode |
| **Stack VM** | `vm.rs` | 737 | Execute bytecode with intrinsic dispatch |
| **WASM Codegen** | `wasm_codegen.rs` | 3,865 | Ark → native `.wasm` binary (WASI-compatible) |
| **WASM Runner** | `wasm_runner.rs` | 700 | Execute `.wasm` via wasmtime |
| **Browser Bridge** | `wasm.rs` | 358 | `wasm_bindgen` API for in-browser execution |
| **Tree-walker** | `eval.rs` | 733 | Interpreter (deprecated, test-only) |

### CLI -- 10 Commands

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

## Cryptography

Core primitives implemented in Rust without OpenSSL or ring:

| Primitive | Status |
|---|---|
| SHA-256 / SHA-512 | ✅ |
| Double SHA-256 | ✅ |
| HMAC-SHA256 / HMAC-SHA512 | ✅ |
| BIP-32 HD Key Derivation | ✅ `derive_key("m/44/0/0")` |
| Ed25519 Sign/Verify | ✅ (via `ed25519-dalek`) |
| Wallet Address Generation | ✅ (`ark:` prefix, checksum) |
| Constant-Time Comparison | ✅ |
| Merkle Root Computation | ✅ |
| Secure Random | ✅ (`/dev/urandom`) |

---

## Diagnostic Proof Suite

Ark includes a diagnostic tool that produces Merkle-rooted, HMAC-signed evidence bundles proving the compiler did its job correctly. The output is machine-verifiable and tamper-evident.

```bash
ark diagnose app.ark                          # Developer tier (detailed metrics)
ark diagnose app.ark --tier pro               # Pro tier (full cryptographic proof)
ark diagnose app.ark --json                   # JSON output for CI/CD integration
ark diagnose app.ark --tier pro --key secret  # Custom HMAC key for signing
```

### What It Proves

Every `ark diagnose` run executes a **5-phase pipeline** (Parse → Check → Pipeline → Gates → Seal) that evaluates your code against **15 quality gates** across 3 diagnostic probes:

| Gate | What It Verifies |
|------|------------------|
| `OVERLAY_DELTA` | Compiler phases actually changed state (catches no-op passes) |
| `LINEAR_SAFETY` | All linear resources consumed correctly, zero leaks |
| `MCC_COMPLIANCE` | Confidence is monotonically non-decreasing (catches regression) |
| `LATENCY` | Each phase completed within its time budget |
| `TOKEN_RATIO` | Output/input size ratio within bounds (catches bloat) |

### Tiered Output (Monetization-Ready)

| Tier | What You Get |
|------|-------------|
| **Free** | Summary + pass/fail + probe count |
| **Developer** | + Per-gate scores, evidence strings, linear audit, pipeline health |
| **Pro** | + Merkle root, HMAC signature, per-probe hashes, full crypto verification chain |

### Sample Output

```
╔══════════════════════════════════════════════════════════╗
║       ARK DIAGNOSTIC PROOF SUITE v1.0                    ║
╚══════════════════════════════════════════════════════════╝

▸ Source: bench_fibonacci.ark
▸ Tier:   DEVELOPER

✓ Parsed (196 bytes, MAST root: 9926f799...)
✓ Linear check passed (score: 1.0000)
✓ Pipeline health: 0.6800 (confidence: 0.6000)

─── DIAGNOSTIC REPORT ───
Gates: 15 passed, 0 failed (avg score: 1.0000)
Overlay: 100.0% improvement
Linear Safety: CLEAN
Pipeline: VERIFIED

✓ ALL QUALITY GATES PASSED

▸ Merkle Root: 81f7a640...
▸ Elapsed:     1ms
```

The Pro tier produces auditable evidence suitable for SOC 2 compliance, smart contract verification, CI/CD quality gates, and supply chain attestation.

---

## Blockchain & Governance

### Blockchain (338 LOC)
Full Proof-of-Work chain: transactions, blocks, Merkle roots, chain validation, balance tracking, difficulty adjustment, code submission. Global singleton via `OnceLock<Mutex<Blockchain>>`.

### Governance Engine (839 LOC)
5-phase governed pipeline (Sense→Assess→Decide→Action→Verify) with HMAC-signed `StepTrace` receipts, Monotone Confidence Constraint enforcement, Dual-Band orientation scoring, and Merkle audit trails.

---

## Multi-Agent AI Framework

Ark includes a built-in agent system:

```text
Task → RouterAgent → [CoderAgent | ResearcherAgent | ReviewerAgent] → Review → Result
```

| Feature | Details |
|---|---|
| **4 Specialist Agents** | Router, Coder (Ark-aware), Researcher, Reviewer |
| **Swarm Strategies** | `router`, `broadcast`, `consensus`, `pipeline` |
| **MCP Client** | JSON-RPC 2.0 over Stdio/HTTP/SSE |
| **Security** | AST-level sandboxing + Docker isolation |
| **Memory** | Fernet-encrypted + TF-IDF semantic recall |
| **LLM Backends** | Gemini → OpenAI → Ollama (auto-fallback) |

```ark
// AI is a first-class intrinsic -- no SDK, no import
answer := sys.ai.ask("Explain linear types in 3 sentences.")
print(answer)

// Multi-agent swarm from Ark code
sys.vm.source("lib/std/ai.ark")
coder := Agent.new("You are a Rust expert.")
reviewer := Agent.new("You are a security auditor.")
swarm := Swarm.new([coder, reviewer])
results := swarm.run("Build a key-value store")
```

### Agent Substrate (26 Modules, ~13,350 LOC)

The Python-level agent framework is backed by a Rust-native substrate -- 26 modules providing security, routing, memory, and lifecycle primitives:

| Layer | Modules | What It Provides |
|---|---|---|
| **Security** | `taint`, `capability`, `shell_bleed`, `manifest_signing`, `tool_policy`, `approval` | Lattice-based taint tracking, capability tokens, shell injection detection (5 languages), Ed25519 manifest signing, deny-wins ACLs, human-in-the-loop approval gates |
| **Agent Safety** | `loop_guard`, `audit`, `context_budget`, `context_overflow`, `graceful_shutdown`, `retry` | SHA-256 dedup loop detection, Merkle hash-chain audit trails, token budget management, overflow strategies, signal-safe shutdown with state preservation, exponential backoff |
| **Channel Framework** | `channel_types`, `channel_formatter`, `channel_router` | 40+ adapter types (Telegram, Slack, Discord, email, SMS, etc.), per-channel formatting, priority-based routing |
| **LLM Layer** | `llm_driver`, `model_catalog`, `routing`, `provider_health` | 130+ model registry across 28 providers with pricing/context data, complexity-based model routing, provider health probing |
| **Agent Lifecycle** | `a2a`, `embedding`, `hooks`, `kernel_handle` | Google A2A protocol (Agent Cards + task store), vector embedding driver (8 models), 4-event lifecycle hooks, 26-method kernel handle trait |
| **Memory** | `semantic_memory` | Semantic memory fragments with confidence decay, knowledge graph (entities + relations), in-memory consolidation engine, remember/recall/forget lifecycle |

> **Zero new dependencies.** The substrate is Ark-native Rust, with its architecture informed by [OpenFang](https://github.com/ArcadeLabsInc/openfang) (MIT/Apache-2.0). All 744 tests pass.

---

## Standard Library (16 Modules)

| Module | Purpose | Key Functions |
|---|---|---|
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
| `gcd` | Data integrity | `evaluate`, `audit_dataset`, `decorrelate`, `create_contract`, `normalize` |

---

## Additional Subsystems

| Subsystem | LOC | What It Does |
|---|---|---|
| **Agent Substrate** | 13,350+ | 24-module Rust-native agent runtime (security, LLM, lifecycle, memory) -- see above |
| **Diagnostic Proof Suite** | 780+ | Cryptographic compilation verification (Merkle + HMAC) |
| **Hygienic Macros** | 522 | `gensym`-based macro expansion |
| **Interactive Debugger** | 248 | Breakpoints, step-in/out, variable inspection |
| **Content-Addressed AST (MAST)** | 218 | SHA-256 hashed AST nodes for integrity |
| **WIT Generator** | 477 | Ark types → WebAssembly Interface Types |
| **WASM Host Imports** | 361 | Bridge intrinsics into WASM modules |
| **Persistent Data Structures** | 832 | PVec (trie) + PMap (HAMT) with structural sharing |
| **ADN (Ark Data Notation)** | 526 | Bidirectional serialization (like Clojure's EDN) |
| **FFI** | 120 | C ABI: `extern "C" fn ark_eval_string()` |
| **WASM Interop** | 428 | Load/call/inspect external `.wasm` modules |
| **VSCode Extension** | -- | TextMate grammar, language config (v1.3.0) |
| **Browser Playground** | -- | `site/wasm/index.html` test harness |
| **Leviathan WASM Portal** | 1,086 | Live CSG compilation via manifold-3d WASM + Z3 verification + GLB export |
| **GitHub CI** | -- | 10 jobs across 3 OS + Docker + WASM + Audit |

---

## Quick Start

### Docker (Recommended)

```bash
git clone https://github.com/merchantmoh-debug/ArkLang.git
cd ArkLang
docker build -t ark .
docker run -it --rm ark
```

### From Source (Recommended)

```bash
git clone https://github.com/merchantmoh-debug/ArkLang.git
cd ArkLang

# Build the Rust compiler
cd core && cargo build --release && cd ..

# Install Python tooling (pick one)
uv sync                         # Recommended (fast, deterministic)
pip install -r requirements.txt  # Also works

# Run your first program
echo 'print("Hello from Ark!")' > hello.ark
python meta/ark.py run hello.ark
```

> **Don't have uv?** Install it in one line: `curl -LsSf https://astral.sh/uv/install.sh | sh`
> Or on Windows: `powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"`

### Try the Examples

```bash
# Wallet CLI -- Secp256k1 + BIP39 in pure Ark
python3 meta/ark.py run apps/wallet.ark create "mypassword"

# Market Maker -- Linear types enforcing no double-counting
python3 meta/ark.py run apps/market_maker.ark

# Snake Game -- playable in the browser
python3 meta/ark.py run examples/snake.ark
# Open http://localhost:8000

# Leviathan Portal -- Z3 + CSG compilation in the browser
# Visit: https://merchantmoh-debug.github.io/ArkLang/site/leviathan/
```

---

## Documentation

| Document | Description |
|---|---|
| **[User Manual](docs/USER_MANUAL.md)** | Complete language guide -- enums, traits, functions, imports, crypto, blockchain, AI |
| **[Quick Start](docs/QUICK_START.md)** | 5-minute setup |
| **[API Reference](docs/API_REFERENCE.md)** | All 109 intrinsics with signatures and examples |
| **[Stdlib Reference](docs/STDLIB_REFERENCE.md)** | All 16 standard library modules |
| **[Language Spec](docs/ARK_LANGUAGE_SPEC.md)** | Formal specification -- types, grammar, runtime semantics |
| **[Manifesto](docs/MANIFESTO.md)** | The philosophy -- why Ark exists |
| **[Roadmap](docs/ROADMAP.md)** | What's next |
| **[Leviathan Portal](https://merchantmoh-debug.github.io/ArkLang/site/leviathan/)** | Live demo -- Z3 + CSG compilation in the browser |

---

## Security Model

| Feature | Details |
|---|---|
| **Default** | Air-gapped -- no network, no filesystem writes, no shell |
| **Capability Tokens** | `ARK_CAPABILITIES="net,fs_read,fs_write,ai"` |
| **Static Analysis** | Security scanner catches injection, path traversal, hardcoded secrets |
| **Import Security** | Path traversal → `RuntimeError::UntrustedCode` |
| **Circular Import Protection** | `imported_files` HashSet |
| **Agent Sandbox** | AST analysis + Docker isolation for untrusted workloads |
| **Epistemic Firewall** | `Censored` (∞_rec) sentinel blocks arithmetic on missing data -- compiler-enforced |
| **Data Integrity (GCD)** | AM-GM bound detects weak channels hidden by averages -- `audit_dataset()` halts on fraud |

---

## Data Integrity (GCD Kernel)

The `gcd` standard library module provides contract-frozen measurement discipline that detects fraudulent data at runtime.

The `gcd` standard library module implements the **Tier-1 Kernel** from Clement Paulus's [Generative Collapse Dynamics](https://doi.org/10.5281/zenodo.18819238) (GCD/UMCP v2.1.3) framework. It provides a contract-frozen measurement discipline that uses the **AM-GM inequality** -- one of the most fundamental bounds in mathematics -- to detect weak data channels hidden by healthy-looking averages.

```ark
import lib.std.gcd

// Run your data through the kernel
ledger := gcd.evaluate(trace, weights)
// ledger.F = arithmetic mean (what looks fine)
// ledger.IC = geometric mean (what actually survives)
// ledger.delta = F - IC (the gap = hidden fragility)

// Or use the kill switch -- halt if the gap is too big
gcd.audit_dataset(trace, weights, 2000)  // VETO if Δ > 0.20
```

| Function | What It Does |
|---|---|
| `gcd.evaluate(trace, weights)` | Full kernel: F, IC, Δ, ρ, ω, κ, S, C |
| `gcd.audit_dataset(trace, weights, max_delta)` | Evaluate + **halt** if Δ exceeds threshold |
| `gcd.decorrelate(trace, weights, threshold)` | Remove correlated channels (Covariance Trap fix) |
| `gcd.create_contract(adapter, epsilon, weights, metric, tolerance)` | Freeze measurement params into SHA-256 RunID |
| `gcd.normalize(trace, epsilon)` | Clip raw values to `[ε, 1-ε]` |

The `Censored` type (`∞_rec`) is enforced at the interpreter level -- any arithmetic on a Censored value raises `CensoredAccessError`. Missing data cannot be silently averaged away. It must be explicitly handled.

> **Credit:** GCD/UMCP theory by [Clement Paulus](https://orcid.org/0009-0000-6069-8234) (CC BY 4.0). Ark implementation adapts the Tier-1 kernel to fixed-point integer arithmetic with contract-freezing and runtime veto.

---

## License

Dual Licensed: **AGPL v3** (Open Source) or **Commercial** (Sovereign Systems).

**Patent Notice:** Protected by US Patent Application 

**GCD/UMCP Attribution:** The `lib/std/gcd.ark` module implements theory from Clement Paulus, *"GCD: Enabling Cross-Domain Comparability via Contract-Frozen Kernel Invariants and Typed Return"* (v2.1.3, February 2026). [DOI: 10.5281/zenodo.18819238](https://doi.org/10.5281/zenodo.18819238). Licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). ORCID: [0009-0000-6069-8234](https://orcid.org/0009-0000-6069-8234).

---

<div align="center">

40,000+ lines of Rust. 92,000+ lines of Python. 744 tests. 58 core modules. 109 intrinsics. 3 backends. 10/10 CI.

</div>
