# SWARM MISSION: AGENT THETA (THE SENTRY)
**TARGET:** `tests/security/`
**OBJECTIVE:** Security Regression Suite.

## CONTEXT
We need to prove the sandbox is secure.

## INSTRUCTIONS
1.  **Create `tests/security/path_traversal.ark`**:
    - Try to read `/etc/passwd` or `../IMPLEMENTATION_PLAN.md`.
    - Assert failure.
2.  **Create `tests/security/exec_violation.ark`**:
    - Try to run `sys.exec("rm -rf /")`.
    - Assert failure (when sandbox is ON).

## CONSTRAINTS
- These tests MUST fail.
