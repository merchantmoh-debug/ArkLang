# Contributing to Ark Compiler Prime

**Welcome to the Resistance.**

You have cloned this repository because you sense something is wrong with the current state of software engineering. You are right.
We are building the **Sovereign Stack**—language, compiler, and runtime—to free ourselves from the dependency on hyper-scale corporate AI and bloated legacy ecosystems.

## The Iron Rules of Contribution

We do not accept "slop". We do not accept "drive-by" PRs for Hacktoberfest t-shirts.
If you contribute, you contribute to a Civilization-Grade Kernel.

### 1. The "Verify First" Doctrine
Before you push, you MUST run:
```bash
.\verify.bat
```
If this script fails (Red Text), your PR will be closed without comment.
**Green means Go. Red means No.**

### 2. No AI Slop
We use AI (Jules, Gemini, etc.) as **Tools**, not **Authors**.
- If you use AI to generate code, YOU are responsible for verifying every line.
- If we find hallucinated function calls or "dream logic" that doesn't compile, you are banned.
- **Audit your own AI.**

### 3. The "Grandmaster" Standard
- **No "Fixing Typos" PRs**: Unless it changes logic or critical documentation, do not spam us with whitespace/typo fixes.
- **Atomic Commits**: One feature, one commit.
- **Tests Are Mandatory**: If you add a feature, add a `.ark` test case in `tests/`.

## Architecture Overview

- **`core/` (Rust)**: The Virtual Machine, Bytecode, and Crypto primitives. The "Engine".
- **`meta/` (Python)**: The Compiler Frontend, Swarm Bridge, and Simulation tools. The "Brain".
- **`lib/std/` (Ark)**: The Standard Library. The "Soul".
- **`apps/` (Ark)**: Proof-of-concept applications. The "Body".

## How to Start (The "Hello World" Path)

1.  **Run the Gauntlet**: `.\verify.bat`.
2.  **Read the Specs**: `docs/`.
3.  **Pick a Mission**: Look for "TODO" or "Future Work" in `README.md`.
4.  **Write a Script**: Create `apps/my_demo.ark` and make it do something cool.

**Welcome to Protocol Omega.**
