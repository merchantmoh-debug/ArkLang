# Ark Roadmap

> **Current:** Phase 79 (Post-Parity) — 100% Rust intrinsic parity achieved.

---

## Completed

### Phase 78: Rust Parity Sprint
- 100% Python↔Rust intrinsic parity (107/107 parity + 2 Rust-only = 109 total)
- Diagnostic Proof Suite (780+ LOC, 3-tier monetization, CLI integration)
- wasmtime upgraded to v41.0.3

---

## Current: Phase 79 — Distribution & WASM

### WASM Hardening
- [ ] Gate I/O intrinsics for `wasm32-unknown-unknown` (`#[cfg(not(target_arch = "wasm32"))]`)
- [ ] Browser polyfills for `sys.log` (console.log) and `sys.time` (Date.now)
- [ ] Add `cargo build --target wasm32-unknown-unknown` to CI

### Self-Documentation
- [ ] Auto-generate intrinsic reference from `intrinsics.rs` docstrings
- [ ] Upgrade LSP to use `ark_to_json` AST for improved diagnostics

### Package Manager
- [ ] Finalize file-based registry (`meta/pkg/`)
- [ ] Implement version pinning and dependency resolution

---

## Phase 80 — Diagnostic Suite Enhancements

### Coverage Expansion
- [ ] Custom quality gates (user-defined)
- [ ] Gate severity levels (warning/error/critical)
- [ ] Historical ProofBundle tracking for trend analysis

### CI/CD Integration
- [ ] GitHub Actions Action (`ark-diagnose-action`)
- [ ] SARIF output for IDE integration
- [ ] Auto-generated repo badges from diagnostic results

### Supply Chain Security
- [ ] SBOM generation in ProofBundle
- [ ] Sigstore integration for public verifiability
- [ ] Public attestation registry

---

## Phase 81+ — Long-Term

### Sovereign Computing
- [ ] Ollama-native AI (zero-config, no API keys required)
- [ ] Noise_XX handshake for encrypted P2P communication
- [ ] DAG-based memory model (replace linear heap)

### Formal Verification
- [ ] Native Z3 integration in Rust core (currently Python bridge)
- [ ] Custom quality gates for domain-specific verification

### Cleanup
- [ ] Formally deprecate `ark_interpreter.py` (Python tree-walker)
- [ ] Benchmark Rust runtime vs Python interpreter

---

**© 2026 Sovereign Systems**
