# Phase 79: The Sovereign Upgrade (Post-Parity)

> **Status:** Phase 78 (Rust Parity) is **COMPLETE (100%)**.
> **Objective:** With the Core Runtime now feature-complete (107/107 parity, 109 total intrinsics), the focus shifts to **WASM Distribution** and **Self-Documentation**.

## ✅ COMPLETED: Phase 78 (The Parity Sprint)
- **100% Rust Intrinsic Parity Achieved.**
- **Swarm Success:** Jules Swarm ported 31 intrinsics.
- **Local Success:** Implemented final 12 intrinsics (JSON, Tensor Math, Z3 stub, VM Source).
- **Runtime Pipeline:** Merged `ark_loader` and `ark_to_json` pipeline.
- **Verification:** `cargo check` passing cleanly.

---

## 🚀 PHASE 79 PLAN: The Distribution (WASM & Packaging)

### 1. WASM Hardening (The Edge)
Now that `intrinsics.rs` is full, we must ensure it compiles to `wasm32-unknown-unknown` for the browser.
- [ ] **Gate I/O:** Wrap `sys.fs.*` and `sys.net.*` in `#[cfg(not(target_arch = "wasm32"))]`.
- [ ] **WASM Polyfills:** Implement browser-side equivalents for `sys.log` (console.log) and `sys.time` (Date.now).
- [ ] **Build Pipeline:** Add `cargo build --target wasm32-unknown-unknown` to CI.

### 2. The Holographic Spec (Self-Documentation)
- [ ] **Generate Core Reference:** Auto-generate specificaiton from `intrinsics.rs` docstrings.
- [ ] **LSP Upgrade:** Update the Language Server to use the new `ark_to_json` AST for better error reporting.

### 3. Package Manager (The Swarm)
- [ ] **Registry:** Finalize the file-based registry in `meta/pkg/`.
- [ ] **Dependency Resolution:** Implement simple version pinning.

---

## TIER 1: IMMEDIATE TASKS (Cleanup)
- [ ] **Prune Legacy Python:** Now that Rust is authoritative, `ark_interpreter.py` should be formally deprecated/archived.
- [ ] **Benchmark:** Compare Rust Runtime speed vs Legacy Python Interpreter on `fib.ark`.

---

## Execution Strategy
1. **Gate Intrinsics:** Modify `core/src/intrinsics.rs` to allow WASM compilation.
2. **Build WASM:** Verify `apps/server.ark` can run in the browser with the new runtime.
3. **Docs:** Publish the `ARK_STD_LIB.md` reference.
