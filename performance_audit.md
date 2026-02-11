# ARK OMEGA-POINT v112.0: Performance Audit

**CLASSIFICATION:** KINETIC AUDIT
**TARGET:** `core/src/vm.rs` and Memory Architecture (`core/src/gc.rs` - MISSING)
**STATUS:** CRITICAL BOTTLENECK IDENTIFIED

## §1 | CRITICAL FINDINGS: THE MEMORY WALL

### A. The "Fat Enum" Problem (Stack Bloat)
In `core/src/runtime.rs`, the `Value` enum is catastrophically large.
```rust
pub enum Value {
    Integer(i64),                 // 8 bytes
    String(String),               // 24 bytes (ptr + cap + len)
    List(Vec<Value>),             // 24 bytes (ptr + cap + len)
    Struct(HashMap<String, Value>), // ~48+ bytes (depends on implementation, potentially very large)
    ...
}
```
**Impact:**
*   The size of `Value` is determined by its largest variant (`Struct` or `String`).
*   This means *every* integer, boolean, or unit value consumes ~64 bytes of stack space (padding included).
*   Result: Excessive CPU cache pressure and memory bandwidth usage for simple operations.

### B. The O(N) Clone Catastrophe
The `VM` implementation in `core/src/vm.rs` relies on deep cloning for variable access.
*   **Mechanism:** `OpCode::Load` calls `scope.get()`, which calls `Value::clone()`.
*   **The Flaw:** `Value::List` and `Value::Struct` derive `Clone`, which performs a **Deep Copy** of the entire structure.
*   **Scenario:** Loading a list of 10,000 items triggers 10,000 allocations and copies.
*   **Complexity:** $O(N)$ where $N$ is the size of the data structure. This is unacceptable for a high-performance runtime.

### C. The Missing Garbage Collector
*   `core/src/gc.rs` does not exist.
*   The system currently relies on Rust's RAII (Drop) for memory management.
*   While RAII is deterministic, it forces deep copying for shared state (since we cannot easily share `Value` without `Rc` or `Gc`).
*   This leads to the "Clone Everywhere" anti-pattern observed in `vm.rs`.

---

## §2 | OPTIMIZATION PROTOCOLS (The Fix)

### OPTIMIZATION 1: ENUM SIZE REDUCTION (Box Large Variants)
**Strategy:** Reduce `Value` size to 16 bytes (or 2 words).
**Implementation:** Wrap large variants in `Box` or `Rc`.
```rust
pub enum Value {
    Integer(i64),
    // Box the large types to keep the enum small
    String(Box<String>),
    List(Box<Vec<Value>>),
    Struct(Box<HashMap<String, Value>>),
    ...
}
```
**Gain:**
*   Reduces stack usage by ~75%.
*   Improves CPU cache locality for the `VM` stack.
*   Makes moving `Value`s (e.g., in `Call` frames) much cheaper.

### OPTIMIZATION 2: REFERENCE COUNTING (O(1) Cloning)
**Strategy:** Replace deep copies with shallow copies using Reference Counting (`Rc`).
**Implementation:**
```rust
use std::rc::Rc;
use std::cell::RefCell;

pub enum Value {
    // ...
    // Use Rc<RefCell<...>> for shared mutable state
    List(Rc<RefCell<Vec<Value>>>),
    Struct(Rc<RefCell<HashMap<String, Value>>>),
    // ...
}
```
**Gain:**
*   `Clone` becomes $O(1)$ (increment ref count).
*   `OpCode::Load` becomes instant, regardless of data size.
*   Enables shared state between scopes/functions without copying.

### OPTIMIZATION 3: TICKING GC vs. STOP-THE-WORLD
**Strategy:** Implement an **Incremental ("Ticking") Garbage Collector**.
**Context:**
*   **Stop-The-World (STW):** Pauses execution to scan the entire heap. Simple to implement but causes frame drops/latency spikes.
*   **Ticking GC:** Interleaves GC steps with VM execution (e.g., mark 10 objects per opcode).
**Recommendation:**
*   Adopt a **Ticking GC** integrated into the `VM::run` loop.
*   Each `VM` step (or every N steps) performs a small slice of GC work.
*   This distributes the latency cost, ensuring the "Sovereign" runtime remains responsive (critical for the "Omega-Point" architecture).
*   Replace `Rc` with `Gc<Trace>` to handle reference cycles, which `Rc` cannot leak-check effectively.

---

**VERDICT:** The current `VM` is memory-bound by unnecessary copying and bloated stack values. Implementing **Boxing** and **Reference Counting** (or a Ticking GC) is mandatory for v112.0 performance targets.
