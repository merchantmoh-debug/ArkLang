# SWARM MISSION: AGENT GAMMA (THE ENFORCER)
**TARGET:** `compiler.ark` (or `meta/ark.py` checker logic)
**OBJECTIVE:** Enforce Strict Linear Types.

## CONTEXT
"The Iron Price." We must prove that Ark is safer than C++.
We need to ensure that `sys.mem.write(buf)` consumes `buf`, making it invalid for future use.

## INSTRUCTIONS
1.  **Analyze `meta/ark.py` (Interpreter Check)** OR **`compiler.ark` (Self-Hosted)**.
    - *Note: Focus on the Python Interpreter for v113.0.*
2.  **Implement `check_linearity`:**
    - When `sys.mem.write(buf, ...)` is called, mark the variable holding `buf` as "MOVED".
    - If `buf` is accessed again (e.g. `sys.mem.read(buf)`), raise a `LinearityViolation`.
3.  **Create Test Case:**
    - `tests/fail_linear_double_use.ark`.
    - It must attempt to use a buffer twice.
    - Assert that the interpreter crashes/raises an error.

## CONSTRAINTS
- **DO NOT** break existing valid code.
- Focus on `sys.mem.*` intrinsics only for this Sprint.
