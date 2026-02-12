# SWARM MISSION: AGENT ETA (THE LIBRARIAN)
**TARGET:** `lib/std/math.ark`
**OBJECTIVE:** Advanced Tensor Math.

## CONTEXT
AI requires Tensors (Matrices).

## INSTRUCTIONS
1.  **Modify `lib/std/math.ark`**:
    - Implement `struct Tensor { data: List, shape: List }`.
    - Implement `func matmul(a: Tensor, b: Tensor) -> Tensor`.
2.  **Optimize**:
    - (Optional) Use `intrinsic_math_*` if available.

## CONSTRAINTS
- Keep it pure Ark for now (slow but correct).
