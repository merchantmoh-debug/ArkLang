# SWARM MISSION: AGENT LAMBDA (THE KERNEL)
**TARGET:** `core/src/vm.rs`
**OBJECTIVE:** HyperGraph Memory Model.

## CONTEXT
Replace `Vec<u8>` with `GraphArena`.

## INSTRUCTIONS
1.  **Modify `core/src/vm.rs`**:
    - Change `heap: Vec<u8>` to a more complex struct (e.g. `Slab` or `Arena`).
    - Implement "Zero-Copy" reference counting for frames.

## CONSTRAINTS
- This is Hardcore Rust. Be careful.
