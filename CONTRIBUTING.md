# Contributing to Ark Compiler Prime

**Welcome to the Resistance.**

We are building the **Sovereign Stack**—language, compiler, and runtime—to free ourselves from the dependency on hyper-scale corporate AI and bloated legacy ecosystems.

## Getting Started

### Prerequisites

To build and contribute to Ark, you need the following tools installed:

1.  **Rust 1.80+**: Required for the Core VM. [Install Rust](https://rustup.rs/)
2.  **Python 3.11+**: Required for the Bootstrap Compiler and Swarm tools.
3.  **Git**: For version control.

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/merchantmoh-debug/ark-compiler.git
cd ark-compiler

# Build the Rust Core
cd core
cargo build --release
cd ..
```

### Running Tests

We use a custom test runner called **The Gauntlet**. It runs the full suite of Ark tests.

```bash
python meta/gauntlet.py
```

**Note:** All tests must pass (Green) before you submit a PR.

### Running the REPL

To interact with the Ark language directly:

```bash
python meta/ark.py repl
```

This launches the interactive Read-Eval-Print Loop where you can type Ark code and see immediate results.

## Development Workflow

1.  **Fork** the repository to your own GitHub account.
2.  **Create a Branch** for your changes.
    -   `feature/description`: For new features.
    -   `fix/description`: For bug fixes.
    -   `docs/description`: For documentation updates.
3.  **Code** your changes.
4.  **Test** your changes using `meta/gauntlet.py`. Add new tests in `tests/` if applicable.
5.  **Submit a Pull Request (PR)** to the `main` branch.

### Commit Message Format

We follow a structured commit message format:

```
[component] Short description of the change
```

Examples:
-   `[core] Add list.pop intrinsic`
-   `[meta] Fix parser bug in while loops`
-   `[docs] Update installation instructions`

## Code Style

### Rust (`core/`)
-   Run `cargo fmt` to format your code.
-   Run `cargo clippy` to catch common mistakes.

### Python (`meta/`)
-   Follow **PEP 8** guidelines.
-   Use **Type Hints** for function arguments and return values.

### Ark (`apps/`, `lib/std/`)
-   Use **4-space indentation**.
-   Use `snake_case` for function names and variables.
-   Use `PascalCase` for struct names.

## Architecture Overview

-   **`meta/`**: The Python reference runtime. Includes the parser, interpreter, and intrinsics implementation. Acts as the "Brain".
-   **`core/`**: The Rust production runtime. Includes the Virtual Machine (AVM), compiler, and intrinsics. Acts as the "Engine".
-   **`lib/std/`**: The Ark Standard Library written in Ark.
-   **`apps/`**: Demo applications and examples.
-   **`tests/`**: The test suite, run by The Gauntlet.

## Adding an Intrinsic

To add a new intrinsic function (e.g., `math.gcd`), follow these steps:

1.  **Define in Python**: Add the implementation to `meta/ark_intrinsics.py`.
2.  **Define in Rust**: Add the implementation to `core/src/intrinsics.rs`.
3.  **Register**: Add the function to the dispatch table in both runtimes.
4.  **Test**: Add a test case in `tests/test_math.ark` (or create a new test file).
5.  **Update Parity**: Update `INTRINSIC_PARITY.md` to reflect the new addition.

## Issue Labels

-   `good-first-issue`: Good for newcomers.
-   `help-wanted`: We need extra hands on this.
-   `intrinsic-parity`: Tasks related to matching Python and Rust intrinsic implementations.
-   `rust`: Issues specific to the Core VM.
-   `python`: Issues specific to the Meta tools.
-   `documentation`: Docs improvements needed.

## Code of Conduct

We are committed to providing a friendly, safe and welcoming environment for all.
Please read and follow the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
