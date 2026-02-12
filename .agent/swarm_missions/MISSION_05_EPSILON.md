# SWARM MISSION: AGENT EPSILON (THE MATHEMATICIAN)
**TARGET:** `meta/z3_bridge.py`
**OBJECTIVE:** Bridge Ark to Microsoft Z3 Theorem Prover.

## CONTEXT
We need to verify code correctness mathematically.
We will create a bridge that allows Ark to send constraints to Z3.

## INSTRUCTIONS
1.  **Create `meta/z3_bridge.py`**:
    - Import `z3` (if available) or create a mock interface.
    - Implement `verify_contract(constraints: List[str]) -> bool`.
2.  **Modify `meta/ark.py`**:
    - Add `sys.z3.verify` intrinsic.
    - It should call the bridge.

## CONTEXT7 RESOURCE
- **Library ID:** `/z3prover/z3`
- **Use Case:** Verify Python bindings for Theorem Prover.

## CONSTRAINTS
- Handle the case where Z3 is not installed gracefully (print "Z3 Missing, Skipping Verification").
