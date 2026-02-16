# The Ark-1 Programmer's Field Manual

**Version:** Omega-Point v113.5
**Architect:** Mohamad Al-Zawahreh (Sovereign Systems)
**Status:** ACTIVE / SOVEREIGN

---

## 1. Installation

Ark is distributed as a source-available sovereign stack. It is designed to be built from source to ensure full auditability.

### Prerequisites

*   **Rust 1.75+**: For the high-performance VM (`core/`).
*   **Python 3.11+**: For the bootstrap compiler and tooling (`meta/`).
*   **Git**: To clone the repository.

### Quick Install

```bash
# 1. Clone the repository
git clone https://github.com/merchantmoh-debug/ark-compiler.git
cd ark-compiler

# 2. Build the Rust VM (The Engine)
cd core
cargo build --release
# The binary will be at core/target/release/ark_loader

# 3. Setup Python Environment (The Brain)
cd ..
pip install -r requirements.txt
```

### Docker Install (Sanctuary)

For a fully isolated environment:

```bash
docker build -t ark-compiler .
docker run -it --rm ark-compiler
```

---

## 2. Quick Start

### Hello World

Create a file named `hello.ark`:

```ark
print("Hello, Sovereign World!")
```

Run it using the Python reference interpreter:

```bash
python meta/ark.py run hello.ark
```

Or compile and run on the Rust VM:

```bash
# Compile to bytecode (hello.arkb)
python meta/compile.py hello.ark hello.arkb

# Run on VM
./core/target/release/ark_loader hello.arkb
```

### Basic Script

```ark
func fib(n) {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

print("Fibonacci of 10 is: " + str(fib(10)))
```

---

## 3. Language Reference

### Variables

Ark uses the `:=` operator for assignment. Variables are dynamically typed but strongly checked at runtime.

```ark
x := 42              // Integer
name := "Ark"        // String
is_valid := true     // Boolean
```

### Control Flow

#### If / Else

```ark
if x > 10 {
    print("Large")
} else {
    print("Small")
}
```

#### Loops

```ark
i := 0
while i < 5 {
    print(i)
    i := i + 1
}
```

### Functions

Functions are defined using the `func` keyword.

```ark
func add(a, b) {
    return a + b
}
```

### Lists

Lists are heterogeneous.

```ark
items := [1, "two", 3.0]
first := list.get(items, 0)
items := list.append(items, 4)
```

---

## 4. Standard Library & Intrinsics

Ark's functionality is exposed through "Intrinsics" – built-in functions that bridge the gap between the high-level language and the low-level runtime.

### System (`sys`)

*   `sys.log(message)`: Logs a message to stdout.
*   `sys.exit(code)`: Terminates the program with the given exit code.
*   `sys.exec(command)`: Executes a shell command. **Requires `ALLOW_DANGEROUS_LOCAL_EXECUTION=true`**.
*   `sys.fs.read(path)`: Reads a file as a string.
*   `sys.fs.write(path, content)`: Writes a string to a file.

### Math (`math`)

*   `math.sin(x)`: Sine function (scaled integer).
*   `math.cos(x)`: Cosine function (scaled integer).
*   `math.sqrt(x)`: Square root.
*   `math.random()`: Random integer between 0 and 100.

### Cryptography (`crypto`)

*   `crypto.sha256(data)`: Returns the SHA-256 hash of the input string.
*   `crypto.uuid()`: Generates a V4 UUID.

### Networking (`net`)

*   `net.http.get(url)`: Performs a GET request.
*   `net.http.post(url, body)`: Performs a POST request.

### Chain (`chain`)

*   `chain.block.height()`: Returns the current block height of the host chain.
*   `chain.tx.send(to, amount)`: Sends a transaction on the host chain.

---

## 5. Configuration

Ark is controlled by several environment variables to ensure security and resource management.

### Execution Limits

*   `ARK_EXEC_TIMEOUT`: Maximum execution time in seconds (Default: 5). Prevents infinite loops.
*   `ARK_MAX_STEPS`: Maximum number of VM instructions to execute (Default: 1,000,000).

### Security

*   `ARK_CAPABILITIES`: A comma-separated list of allowed capabilities.
    *   `*`: Allow all (Dangerous!).
    *   `net`: Allow networking.
    *   `fs_read`: Allow file reading.
    *   `fs_write`: Allow file writing.
*   `ALLOW_DANGEROUS_LOCAL_EXECUTION`: Must be set to `true` to enable `sys.exec` and file system writes outside of sandboxed directories.

### AI Integration

*   `ARK_API_KEY`: API Key for LLM services (OpenAI, Anthropic, etc.) used by `intrinsic_ask_ai`.
*   `ARK_LLM_ENDPOINT`: Custom endpoint for local LLMs (e.g., Ollama).

---

## 6. FAQ

**Q: Why Rust and Python?**
A: Python provides a flexible, rapid-prototyping frontend ("The Brain"), while Rust provides a secure, high-performance execution engine ("The Engine"). This Dual-Runtime architecture allows us to iterate fast without sacrificing production stability.

**Q: Is Ark production-ready?**
A: The Core VM is stable. The Standard Library is evolving. We follow the "Verify First" doctrine—everything is tested.

**Q: How do I enable networking?**
A: You must set `ARK_CAPABILITIES=net` in your environment. By default, the runtime is air-gapped.

**Q: What happens if my code loops forever?**
A: The `ARK_EXEC_TIMEOUT` watchdog will terminate the process after 5 seconds (configurable).

---

**© 2026 Sovereign Systems**
