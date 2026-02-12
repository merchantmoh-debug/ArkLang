# SWARM MISSION: AGENT ZETA (THE BUILDER)
**TARGET:** `apps/build.ark`
**OBJECTIVE:** Create a Self-Hosting Build System.

## CONTEXT
We use Python to build Ark. We should use Ark to build Ark.

## INSTRUCTIONS
1.  **Create `apps/build.ark`**:
    - A script that iterates over `src/` directory.
    - Calls `sys.exec("cargo build")` or similar.
    - Manages dependencies (if any).
2.  **Features**:
    - `ark build clean`
    - `ark build test`

## CONSTRAINTS
- Use `sys.fs.*` intrinsics.
