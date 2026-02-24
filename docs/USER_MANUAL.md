# The Ark Language — User Manual

**Version:** Phase 112 | **Updated:** 2026-02-19
**License:** AGPL-3.0 | **Author:** Mohamad Al-Zawahreh (Sovereign Systems)

> This manual teaches you everything you need to write real programs in Ark.
> For the full intrinsic API, see [API_REFERENCE.md](API_REFERENCE.md).
> For standard library module docs, see [STDLIB_REFERENCE.md](STDLIB_REFERENCE.md).

---

## Table of Contents

1. [Installation](#1-installation)
2. [Hello World](#2-hello-world)
3. [Variables](#3-variables)
4. [Data Types](#4-data-types)
5. [Operators](#5-operators)
6. [Control Flow](#6-control-flow)
7. [Functions](#7-functions)
8. [Lambdas](#8-lambdas)
9. [Structs](#9-structs)
10. [Enums](#10-enums)
11. [Pattern Matching](#11-pattern-matching)
12. [Traits & Impl Blocks](#12-traits--impl-blocks)
13. [Linear Types](#13-linear-types)
14. [Lists](#14-lists)
15. [Maps & Dictionaries](#15-maps--dictionaries)
16. [Strings](#16-strings)
17. [Imports & Modules](#17-imports--modules)
18. [Standard Library](#18-standard-library)
19. [Intrinsics (Built-ins)](#19-intrinsics-built-ins)
20. [File I/O](#20-file-io)
21. [Networking](#21-networking)
22. [Cryptography](#22-cryptography)
23. [Blockchain](#23-blockchain)
24. [AI Integration](#24-ai-integration)
25. [Agent Framework](#25-agent-framework)
26. [Persistent Data Structures](#26-persistent-data-structures)
27. [Error Handling](#27-error-handling)
28. [Macros](#28-macros)
29. [Configuration & Security](#29-configuration--security)
30. [Running Programs](#30-running-programs)
31. [REPL](#31-repl)
32. [Debugger](#32-debugger)
33. [WASM Compilation](#33-wasm-compilation)
34. [FAQ](#34-faq)

---

## 1. Installation

### Prerequisites

- **Rust 1.80+** — For the Rust VM core. [Install Rust](https://rustup.rs/)
- **Python 3.11+** — For the bootstrap compiler and tooling.
- **Git** — To clone the repo.

### From Source

```bash
# Clone
git clone https://github.com/merchantmoh-debug/ark-compiler.git
cd ark-compiler

# Build the Rust VM
cd core && cargo build --release && cd ..

# Install Python deps
pip install -r requirements.txt

# Verify
python meta/ark.py version
```

### Docker

```bash
docker build -t ark-compiler .
docker run -it --rm ark-compiler
```

---

## 2. Hello World

Create `hello.ark`:

```ark
print("Hello, World!")
```

Run it:

```bash
python meta/ark.py run hello.ark
```

Output:

```
Hello, World!
```

---

## 3. Variables

Ark uses `:=` for assignment. Variables are dynamically typed.

```ark
x := 42              // Integer
name := "Ark"        // String
pi := 3.14159        // Float
is_valid := true     // Boolean
nothing := null      // Null
```

Reassignment uses `:=` as well:

```ark
x := 42
x := x + 1           // x is now 43
```

---

## 4. Data Types

Ark has a 14-variant type system:

| Type | Example | Notes |
| --- | --- | --- |
| Integer | `42`, `-7`, `0` | Arbitrary precision |
| Float | `3.14`, `-0.5` | 64-bit floating point |
| String | `"hello"` | Double-quoted, UTF-8 |
| Boolean | `true`, `false` | Lowercase |
| Null | `null` | Absence of value |
| List | `[1, 2, 3]` | Heterogeneous, ordered |
| Map | `{"key": "value"}` | Key-value pairs |
| Struct | `{x: 1, y: 2}` | Named fields |
| Function | `func(x) { x + 1 }` | First-class values |
| Optional | `some(42)`, `none` | Nullable wrapper |
| Unit | — | Void return type |
| Any | — | Unconstrained type variable |
| Enum | `Shape.Circle(5.0)` | Algebraic data type |
| Trait | — | Interface/protocol type |

---

## 5. Operators

### Arithmetic

```ark
a := 10 + 3    // 13 (add)
b := 10 - 3    // 7  (subtract)
c := 10 * 3    // 30 (multiply)
d := 10 / 3    // 3  (integer division)
e := 10 % 3    // 1  (modulo)
```

### Comparison

```ark
x > y          // greater than
x < y          // less than
x >= y         // greater or equal
x <= y         // less or equal
x == y         // equal
x != y         // not equal
```

### Logical

```ark
a && b         // AND
a || b         // OR
!a             // NOT
```

### String Concatenation

```ark
greeting := "Hello, " + "World!"
```

---

## 6. Control Flow

### If / Else

```ark
if temperature > 100 {
    print("Too hot!")
} else if temperature < 0 {
    print("Freezing!")
} else {
    print("Just right.")
}
```

### While Loop

```ark
i := 0
while i < 10 {
    print(i)
    i := i + 1
}
```

### For Loop

Iterate over lists, ranges, and strings:

```ark
// Over a list
items := ["alpha", "beta", "gamma"]
for item in items {
    print(item)
}

// Over a range
for i in range(0, 10) {
    print(i)
}

// Over characters in a string
for ch in "hello" {
    print(ch)
}
```

### Break & Continue

`break` exits the loop. `continue` skips to the next iteration.

```ark
for i in range(0, 100) {
    if i == 5 {
        break   // stop at 5
    }
    if i % 2 == 0 {
        continue  // skip even numbers
    }
    print(i)  // prints 1, 3
}
```

---

## 7. Functions

Functions are first-class citizens. They are defined with `func` and can be passed around.

```ark
func greet(name) {
    return "Hello, " + name + "!"
}

message := greet("Alice")
print(message)    // Hello, Alice!
```

### Multiple Arguments

```ark
func add(a, b) {
    return a + b
}

print(add(3, 7))  // 10
```

### Recursion

```ark
func factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

print(factorial(10))  // 3628800
```

### Higher-Order Functions

Functions can be assigned to variables and passed as arguments:

```ark
func apply(f, x) {
    return f(x)
}

func double(n) {
    return n * 2
}

print(apply(double, 5))  // 10
```

### Return Type Annotations

```ark
func area(radius: Float) -> Float {
    return 3.14159 * radius * radius
}
```

---

## 8. Lambdas

Anonymous functions are written with `|args| { body }` syntax:

```ark
double := |x| { x * 2 }
print(double(21))  // 42

add := |a, b| { a + b }
print(add(3, 7))   // 10
```

### Lambdas with Higher-Order Functions

```ark
numbers := [1, 2, 3, 4, 5]

doubled := list.map(numbers, |x| { x * 2 })
// [2, 4, 6, 8, 10]

evens := list.filter(numbers, |x| { x % 2 == 0 })
// [2, 4]

sum := list.reduce(numbers, |acc, x| { acc + x }, 0)
// 15
```

Lambdas are fully compiled to WASM with lambda-lifting — they are not interpreted.

---

## 9. Structs

Named structures with typed fields:

```ark
struct Point {
    x: Float,
    y: Float
}

struct Person {
    name: String,
    age: Integer
}
```

### Creating Struct Instances

```ark
let p := {x: 1.0, y: 2.0}

let alice := {name: "Alice", age: 30}
```

### Field Access

```ark
print(p.x)        // 1.0
print(alice.name)  // "Alice"
```

### Field Mutation

```ark
p.x := 5.0
print(p.x)  // 5.0
```

---

## 10. Enums

Enums define algebraic data types — types with a fixed set of named variants, each of which can carry data.

### Defining Enums

```ark
enum Color {
    Red,
    Green,
    Blue
}

enum Shape {
    Circle(Float),
    Rectangle(Float, Float),
    Point
}

enum Option {
    Some(Any),
    None
}
```

### Creating Enum Values

```ark
let c := Color.Red
let s := Shape.Circle(5.0)
let rect := Shape.Rectangle(10.0, 20.0)
let found := Option.Some(42)
let missing := Option.None
```

### Using Enums with Match

See [Pattern Matching](#11-pattern-matching) for destructuring enums.

---

## 11. Pattern Matching

The `match` expression destructures values and selects a branch based on the pattern:

### Matching Enum Variants

```ark
enum Shape {
    Circle(Float),
    Rectangle(Float, Float),
    Point
}

func describe(s) {
    match s {
        Shape.Circle(r)       => print("Circle with radius: " + str(r))
        Shape.Rectangle(w, h) => print("Rectangle: " + str(w) + " x " + str(h))
        Shape.Point           => print("Just a point")
    }
}

describe(Shape.Circle(5.0))
// Circle with radius: 5.0

describe(Shape.Rectangle(3.0, 4.0))
// Rectangle: 3.0 x 4.0
```

### Match with Bindings

Pattern variables bind to the destructured fields, which you can use in the branch body:

```ark
func area(s) {
    match s {
        Shape.Circle(r)       => return 3.14159 * r * r
        Shape.Rectangle(w, h) => return w * h
        Shape.Point           => return 0.0
    }
}

print(area(Shape.Circle(5.0)))      // 78.53975
print(area(Shape.Rectangle(3.0, 4.0)))  // 12.0
```

---

## 12. Traits & Impl Blocks

Traits define shared interfaces. Impl blocks provide concrete implementations.

### Defining a Trait

```ark
trait Drawable {
    func draw(self) -> Unit
    func area(self) -> Float
}

trait Serializable {
    func to_string(self) -> String
}
```

### Implementing a Trait

```ark
impl Drawable for Circle {
    func draw(self) -> Unit {
        print("Drawing circle with radius " + str(self.radius))
    }

    func area(self) -> Float {
        return 3.14159 * self.radius * self.radius
    }
}

impl Serializable for Point {
    func to_string(self) -> String {
        return "(" + str(self.x) + ", " + str(self.y) + ")"
    }
}
```

### Impl Blocks Without Traits

You can also add methods to a type without a trait:

```ark
impl Vector {
    func magnitude(self) -> Float {
        return math.sqrt(self.x * self.x + self.y * self.y)
    }

    func normalize(self) -> Vector {
        let m := self.magnitude()
        return {x: self.x / m, y: self.y / m}
    }
}
```

---

## 13. Linear Types

Ark's **linear type system** enforces resource safety at compile time. Resources annotated as `Linear` must be consumed exactly once — they cannot be copied, and they cannot be silently dropped.

### Type Annotations

| Annotation | Meaning |
| --- | --- |
| `Linear<T>` | Must be used exactly once. Cannot be copied or dropped. |
| `Affine<T>` | Used at most once. Can be dropped, but cannot be copied. |
| `Shared<T>` | Freely copyable and shareable. Default for most values. |

### How It Works

```ark
func transfer(coin: Linear<Coin>, recipient: Address) {
    // 'coin' is MOVED into this function.
    // The caller can NEVER touch 'coin' again.
    send(recipient, coin)
}
```

If you try to use a moved value:

```ark
transfer(coin, alice)
transfer(coin, bob)    // COMPILE ERROR: Use of moved value 'coin'
```

If you forget to use a linear value:

```ark
func broken(coin: Linear<Coin>) {
    // COMPILE ERROR: Linear value 'coin' was never consumed
}
```

### Why This Matters

Linear types prevent entire classes of bugs:

| Bug Class | How Linear Types Prevent It |
| --- | --- |
| Double-spend | Moved value cannot be used again |
| Resource leak | Unconsumed linear value is a compile error |
| Use-after-free | Moved out of scope = gone forever |
| Data race | Single-owner semantics = no shared mutable state |

The checker that enforces these rules is in `checker.rs` (1,413 LOC).

---

## 14. Lists

Lists are ordered, heterogeneous collections.

```ark
items := [1, "two", 3.0, true]
```

### Accessing Elements

```ark
first := list.get(items, 0)       // 1
last  := list.get(items, 3)       // true
```

### Modifying Lists

```ark
items := list.append(items, "new")  // [1, "two", 3.0, true, "new"]
length := list.length(items)        // 5
```

### List Intrinsics

| Intrinsic | Description |
| --- | --- |
| `list.get(list, index)` | Get element at index |
| `list.set(list, index, value)` | Set element at index |
| `list.append(list, value)` | Append to end |
| `list.length(list)` | Get list length |
| `list.slice(list, start, end)` | Extract sublist |
| `list.contains(list, value)` | Check membership |
| `list.map(list, func)` | Apply function to each element |
| `list.filter(list, func)` | Filter by predicate |
| `list.reduce(list, func, init)` | Reduce to single value |
| `list.sort(list)` | Sort the list |
| `list.reverse(list)` | Reverse the list |

---

## 15. Maps & Dictionaries

Maps are key-value collections:

```ark
config := {"host": "localhost", "port": 8080}

// Access
host := map.get(config, "host")

// Mutation
config := map.set(config, "port", 3000)

// Check key
if map.has(config, "host") {
    print("Host is configured")
}
```

### Map Intrinsics

| Intrinsic | Description |
| --- | --- |
| `map.get(map, key)` | Get value by key |
| `map.set(map, key, value)` | Set key-value pair |
| `map.has(map, key)` | Check if key exists |
| `map.keys(map)` | List all keys |
| `map.values(map)` | List all values |
| `map.remove(map, key)` | Remove a key |

---

## 16. Strings

Strings are double-quoted and support concatenation with `+`.

```ark
name := "Ark"
greeting := "Hello, " + name + "!"
print(greeting)    // Hello, Ark!
```

### String Intrinsics

| Intrinsic | Description |
| --- | --- |
| `str.length(s)` | String length |
| `str.upper(s)` | Uppercase |
| `str.lower(s)` | Lowercase |
| `str.split(s, delimiter)` | Split into list |
| `str.join(list, delimiter)` | Join list into string |
| `str.contains(s, sub)` | Check substring |
| `str.replace(s, old, new)` | Replace occurrences |
| `str.trim(s)` | Trim whitespace |
| `str.starts_with(s, prefix)` | Check prefix |
| `str.ends_with(s, suffix)` | Check suffix |
| `str.substring(s, start, end)` | Extract substring |

### Type Conversion

```ark
x := 42
s := str(x)        // "42"
n := int("123")    // 123
f := float("3.14") // 3.14
```

---

## 17. Imports & Modules

Use `import` to bring in standard library modules or other Ark files.

```ark
import lib.std.math
import lib.std.crypto
import lib.std.net
import lib.wallet_lib
```

After importing, you can call the functions defined in that module:

```ark
import lib.std.math

result := math.sqrt(144)
print(result)  // 12
```

### Module Path Convention

| Import Path | Location |
| --- | --- |
| `lib.std.math` | `lib/std/math.ark` |
| `lib.std.crypto` | `lib/std/crypto.ark` |
| `lib.wallet_lib` | `lib/wallet_lib.ark` |
| `apps.server` | `apps/server.ark` |

### Security

- **Path traversal** (`../` or absolute paths) is blocked → `RuntimeError::UntrustedCode`
- **Circular imports** are detected automatically via a `imported_files` HashSet
- All imported code runs within the same sandbox and capability tokens

---

## 18. Standard Library

Ark ships with **13 standard library modules**. Import them with `import lib.std.<module>`.

| Module | Purpose | Key Functions |
| --- | --- | --- |
| `math` | Math operations | `sqrt`, `sin`, `cos`, `pow`, `abs`, `random` |
| `string` | String utilities | `length`, `upper`, `lower`, `split`, `join` |
| `io` | Console I/O | `read_line`, `write` |
| `fs` | File system | `read`, `write`, `exists`, `size`, `read_bytes` |
| `net` | HTTP networking | `http_get`, `http_post` |
| `crypto` | Cryptography | `sha256`, `sha512`, `hmac`, `aes_encrypt`, `uuid` |
| `chain` | Blockchain | `height`, `balance`, `submit_tx`, `get_block` |
| `time` | Date/time | `now`, `sleep`, `format`, `elapsed` |
| `event` | Event system | `poll`, `push` |
| `result` | Error handling | `ok`, `err`, `is_ok`, `unwrap` |
| `audio` | Audio playback | `play`, `stop` |
| `ai` | AI/LLM access | `ask`, `Agent.new`, `Agent.chat`, `Swarm.run`, `pipeline` |
| `persistent` | Immutable data | `PVec`, `PMap` (trie + HAMT, structural sharing) |

> **Full module documentation:** [STDLIB_REFERENCE.md](STDLIB_REFERENCE.md)

---

## 19. Intrinsics (Built-ins)

Intrinsics are functions compiled directly into the runtime — no imports needed.

### Core Intrinsics (Always Available)

```ark
print("Hello!")             // Print to stdout
len([1,2,3])                // 3
type(42)                    // "Integer"
str(42)                     // "42"
int("42")                   // 42
float("3.14")               // 3.14
range(0, 10)                // [0,1,2,...,9]
assert(1 + 1 == 2)          // Pass (or crash with error)
```

### System Intrinsics

```ark
sys.log("Debug message")    // Log to stdout
sys.exit(0)                 // Exit with code
sys.time.now()              // Unix timestamp
sys.time.sleep(1000)        // Sleep for 1000ms
sys.json.parse(json_str)    // Parse JSON string
sys.json.stringify(value)   // Convert to JSON string
sys.exec("ls")              // Execute shell command (requires ALLOW_DANGEROUS_LOCAL_EXECUTION)
```

> **Full list of all 109 intrinsics:** [API_REFERENCE.md](API_REFERENCE.md)

---

## 20. File I/O

```ark
// Read a file
content := sys.fs.read("data.txt")
print(content)

// Write a file
sys.fs.write("output.txt", "Hello from Ark!")

// Check if file exists
if sys.fs.exists("config.json") {
    config := sys.fs.read("config.json")
}

// Get file size
size := sys.fs.size("data.txt")
print("File size: " + str(size) + " bytes")
```

> **Note:** File system access requires the `fs_read` or `fs_write` capability. See [Configuration](#29-configuration--security).

---

## 21. Networking

```ark
// HTTP GET
response := net.http.get("https://api.example.com/data")
print(response)

// HTTP POST
result := net.http.post("https://api.example.com/submit", "{\"key\": \"value\"}")
```

> **Note:** Network access requires `ARK_CAPABILITIES=net`. The runtime is air-gapped by default.

---

## 22. Cryptography

Ark has 14 built-in cryptographic intrinsics — no external dependencies needed.

```ark
// Hashing
hash := sys.crypto.hash("sha256", "hello")
print(hash)  // 2cf24dba5fb0a30e26e83b2ac5b9e29e...

// UUID generation
id := sys.crypto.uuid()
print(id)  // e.g. "550e8400-e29b-41d4-a716-446655440000"

// HMAC
mac := sys.crypto.hmac("sha256", "key", "message")

// Random bytes
bytes := sys.crypto.random_bytes(32)
```

### Full Crypto Primitives

| Primitive | Status |
| --- | --- |
| SHA-256 / SHA-512 | ✅ Hand-rolled in Rust |
| Double SHA-256 | ✅ |
| HMAC-SHA256 / HMAC-SHA512 | ✅ |
| BIP-32 HD Key Derivation | ✅ `derive_key("m/44/0/0")` |
| Ed25519 Sign/Verify | ✅ |
| Wallet Address Generation | ✅ (`ark:` prefix, checksum) |
| Constant-Time Comparison | ✅ |
| Merkle Root Computation | ✅ |
| Secure Random | ✅ (`/dev/urandom`) |

> See also: [STDLIB_REFERENCE.md](STDLIB_REFERENCE.md#crypto)

---

## 23. Blockchain

Ark can interact with Ethereum-compatible chains via JSON-RPC.

```ark
// Get current block height
height := sys.chain.height()
print("Block: " + str(height))

// Check balance
bal := sys.chain.get_balance("0x742d35Cc...")
print("Balance: " + str(bal))

// Submit a transaction
tx_hash := sys.chain.submit_tx(signed_payload)
```

> **Configuration:** Set `ARK_RPC_URL` to your JSON-RPC endpoint (e.g., Infura, Alchemy). Without it, chain intrinsics return stubbed test data.

---

## 24. AI Integration

Ark has built-in LLM integration via the `sys.ai.*` namespace:

```ark
// Direct AI call
answer := sys.ai.ask("What is the capital of France?")
print(answer)  // "Paris"

code := intrinsic_extract_code(answer)  // Extract code blocks from AI response
```

### Agent Class

Create persistent agents with personas and conversation history:

```ark
sys.vm.source("lib/std/ai.ark")

coder := Agent.new("You are a Rust expert. Be concise.")
response := coder.chat("How do I read a file?")
print(response)

// Reset conversation history
coder.reset()
```

### Swarm Class

Run tasks across multiple agents:

```ark
sys.vm.source("lib/std/ai.ark")

architect := Agent.new("You are a software architect.")
reviewer := Agent.new("You are a code reviewer.")

swarm := Swarm.new([architect, reviewer])

// Broadcast: all agents respond independently
results := swarm.run("Design a cache system")

// Pipeline: each agent feeds into the next
final := swarm.run_chain("Build a REST API")
```

### Pipeline Function

Sequential prompt chaining without agents:

```ark
sys.vm.source("lib/std/ai.ark")

result := pipeline([
    "List 5 sorting algorithms",
    "Pick the fastest one and explain why",
    "Write it in Ark"
])
print(result)
```

> **Configuration:** Set `GOOGLE_API_KEY` for Gemini or `ARK_LLM_ENDPOINT` for local models (e.g. Ollama at `http://localhost:11434/v1/chat/completions`).
> Without either, AI calls return a graceful fallback message instead of crashing.

---

## 25. Agent Framework

Ark ships with a **built-in multi-agent AI framework** (`src/`). This is not an add-on — it is a core part of the language: programs that can reason, write code, review their own output, and learn from execution.

### Overview

* **Task Orchestration** — Route tasks to specialist AI agents automatically
* **Multi-Agent Swarm** — Coordinate agents with router, broadcast, consensus, and pipeline strategies
* **MCP Protocol** — Connect to any Model Context Protocol server for tool access
* **Sandboxed Execution** — Run generated code in secure, isolated environments
* **Encrypted Memory** — Persistent agent memory with Fernet encryption and vector search

### Running the Orchestrator

The `AgentOrchestrator` executes a pipeline: **Route → Specialist → Review → Result**.

```bash
python -m src.agent "Write a Python script that reads a CSV and outputs JSON"
python -m src.agent "Analyze the security of apps/server.ark"
```

### Specialist Agents

| Agent | Role | When It's Called |
| --- | --- | --- |
| `RouterAgent` | Classifies the task and picks the right specialist | Always (first step) |
| `CoderAgent` | Writes, modifies, and refactors code — **Ark-aware** with full language reference, `execute_ark()` and `compile_check()` tools | Code generation tasks |
| `ResearcherAgent` | Analyzes codebases, gathers context | Research/analysis tasks |
| `ReviewerAgent` | Audits code for bugs, security, and style | Post-execution review |

### Swarm Mode

The `SwarmOrchestrator` coordinates multiple agents:

```python
from src.swarm import SwarmOrchestrator

swarm = SwarmOrchestrator()

# Router: RouterAgent picks the best specialist
result = await swarm.execute("Optimize the sort algorithm", strategy="router")

# Broadcast: send to ALL agents, collect all responses
result = await swarm.execute("Review this function", strategy="broadcast")

# Consensus: multiple agents answer independently
result = await swarm.execute("Is this code secure?", strategy="consensus")

# Pipeline: chain agents sequentially
result = await swarm.execute_pipeline("Build a REST API", ["coder", "reviewer"])

# Parallel: run multiple tasks concurrently
results = await swarm.execute_parallel([
    "Write unit tests", "Refactor queries", "Update docs"
])
```

### Configuring LLM Backends

The framework is backend-agnostic. It tries backends in order: Gemini → OpenAI → Ollama.

**Google Gemini:**

```bash
export ARK_GOOGLE_API_KEY="your-api-key"
```

**OpenAI / Compatible:**

```bash
export ARK_OPENAI_BASE_URL="https://api.openai.com/v1"
export ARK_OPENAI_API_KEY="sk-..."
export ARK_OPENAI_MODEL="gpt-4o"
```

**Ollama (Local, Free):**

```bash
export ARK_OPENAI_BASE_URL="http://localhost:11434/v1"
export ARK_OPENAI_API_KEY="ollama"
export ARK_OPENAI_MODEL="llama3"
```

### MCP Server Integration

The framework includes a full [Model Context Protocol](https://modelcontextprotocol.io/) client supporting Stdio, HTTP, and SSE transports.

Create `mcp_servers.json`:

```json
{
  "servers": [
    {
      "name": "filesystem",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"],
      "transport": "stdio",
      "enabled": true
    }
  ]
}
```

Enable MCP:

```bash
export ARK_MCP_ENABLED=true
export ARK_MCP_SERVERS_CONFIG="mcp_servers.json"
python -m src.agent "List all Python files in the project"
```

### Sandbox Security

All agent-generated code runs in a sandbox:

**Local Sandbox** (default):

* AST-level static analysis before execution
* Blocks dangerous imports (`os`, `sys`, `subprocess`, `socket`, etc.)
* Blocks dangerous functions (`exec`, `eval`, `compile`, `__import__`)
* Blocks dangerous attributes (`__subclasses__`, `__globals__`, `__code__`)
* Supports Python, Ark, JavaScript, and Rust

**Docker Sandbox** (for untrusted code):

* Full container isolation, network-disabled by default
* CPU/memory/disk resource limits

```bash
export ARK_SANDBOX_TYPE="auto"    # auto, local, or docker
```

### Encrypted Agent Memory

```python
from src.memory import MemoryManager, VectorMemory

mem = MemoryManager()  # Uses ARK_MEMORY_KEY for encryption
mem.store("api_response", {"status": 200})
result = mem.recall("api_response")
matches = mem.search("api")  # Fuzzy search

# Vector similarity search (TF-IDF)
vmem = VectorMemory()
vmem.store_embedding("doc1", "How to configure the sandbox")
results = vmem.search_similar("sandbox setup", top_k=3)
```

### Agent Framework Environment Variables

| Variable | Default | Description |
| --- | --- | --- |
| `ARK_MODEL` | `gpt-4` | Default LLM model |
| `ARK_TEMPERATURE` | `0.7` | LLM temperature |
| `ARK_MAX_TOKENS` | `4096` | Max output tokens |
| `ARK_SANDBOX_TYPE` | `auto` | Sandbox: `auto`, `docker`, `local` |
| `ARK_MEMORY_KEY` | (none) | Master encryption key for agent memory |
| `ARK_MCP_ENABLED` | `false` | Enable MCP integration |
| `ARK_MCP_SERVERS_CONFIG` | `mcp_servers.json` | Path to MCP server config |
| `ARK_DEBUG` | `false` | Enable debug logging |

---

## 26. Persistent Data Structures

Ark includes **persistent immutable data structures** — collections that preserve all previous versions when modified (structural sharing).

```ark
import lib.std.persistent
```

### PVec (Persistent Vector)

An immutable vector implemented as a 32-way trie:

```ark
let v := PVec.new()
let v1 := v.push(1)
let v2 := v1.push(2)
let v3 := v2.push(3)

print(v3.get(0))  // 1
print(v3.len())   // 3

// v, v1, v2 are all still valid and unchanged
print(v.len())    // 0
print(v1.len())   // 1
```

### PMap (Persistent Map)

A hash-array mapped trie (HAMT):

```ark
let m := PMap.new()
let m1 := m.set("name", "Ark")
let m2 := m1.set("version", "112")

print(m2.get("name"))     // "Ark"
print(m2.has("version"))  // true

// Original map is unchanged
print(m.has("name"))      // false
```

### Why Persistent Data Structures?

| Property | Benefit |
| --- | --- |
| Immutability | No shared mutable state = no data races |
| Structural Sharing | Memory-efficient: O(log N) per update |
| Versioning | Every modification creates a new version |
| Thread Safety | Safe to share across concurrent contexts |

The implementation is 832 LOC in `persistent.rs`.

---

## 27. Error Handling

Use the `result` standard library module for structured error handling:

```ark
import lib.std.result

// Functions can return result values
func divide(a, b) {
    if b == 0 {
        return result.err("Division by zero")
    }
    return result.ok(a / b)
}

r := divide(10, 0)
if result.is_ok(r) {
    print("Result: " + str(result.unwrap(r)))
} else {
    print("Error: " + result.unwrap_err(r))
}
```

For simple validation, use `assert`:

```ark
assert(x > 0)  // Crashes with error if false
```

---

## 28. Macros

Ark supports a hygienic macro system with `gensym`-based symbol generation (522 LOC in `macros.rs`).

Macros allow you to extend the language's syntax and generate code at compile time. They are pattern-matched on the AST and expand before type checking.

---

## 29. Configuration & Security

Ark uses environment variables for security controls. **By default, the runtime is sandboxed** — no network, no file writes, no shell access.

### Environment Variables

| Variable | Default | Description |
| --- | --- | --- |
| `ARK_EXEC_TIMEOUT` | `5` | Max execution time in seconds |
| `ARK_MAX_STEPS` | `1000000` | Max VM instructions |
| `ARK_CAPABILITIES` | (none) | Comma-separated: `net`, `fs_read`, `fs_write`, `*` |
| `ALLOW_DANGEROUS_LOCAL_EXECUTION` | `false` | Enable `sys.exec()` |
| `ARK_API_KEY` | (none) | API key for `sys.ai.ask` |
| `ARK_LLM_ENDPOINT` | (none) | Custom LLM endpoint (e.g., Ollama) |
| `ARK_RPC_URL` | (none) | Ethereum JSON-RPC URL for chain intrinsics |

### Example: Enable Networking

```bash
ARK_CAPABILITIES=net python meta/ark.py run my_app.ark
```

### Example: Full Permissions (Dangerous)

```bash
ARK_CAPABILITIES=* ALLOW_DANGEROUS_LOCAL_EXECUTION=true python meta/ark.py run my_app.ark
```

---

## 30. Running Programs

### Execute a Script

```bash
python meta/ark.py run <file.ark>
```

### Compile to Bytecode

```bash
python meta/ark.py compile <file.ark>
```

### Run on the Rust VM

```bash
# Compile first
python meta/ark.py compile hello.ark

# Then execute the bytecode
./core/target/release/ark_loader hello.arkb
```

### Run the Test Suite

```bash
python meta/gauntlet.py
```

---

## 31. REPL

Launch the interactive Read-Eval-Print Loop:

```bash
python meta/ark.py repl
```

```
Ark REPL v1.0 — Type 'exit' to quit
>>> x := 42
42
>>> x + 8
50
>>> print("Hello from REPL!")
Hello from REPL!
```

---

## 32. Debugger

Ark includes an interactive step-through debugger (248 LOC):

```bash
python meta/ark.py debug <file.ark>
```

Features:

- **Breakpoints** — Set on any line
- **Step In / Step Over / Step Out** — Navigate through execution
- **Variable Inspection** — See all variables in scope at any point
- **Call Stack** — View the full function call chain

---

## 33. WASM Compilation

Ark compiles directly to native WebAssembly (3,865 LOC in `wasm_codegen.rs`):

```bash
# Compile Ark source to .wasm
python meta/ark.py build <file.ark>

# Run the compiled .wasm via wasmtime
python meta/ark.py run-wasm <file.wasm>
```

### WASM Features

- **WASI-compatible** — runs on any WASI runtime (wasmtime, wasmer, etc.)
- **Lambda lifting** — closures are compiled to static functions with captured variables
- **Full type system** — all 14 Ark types are represented in WASM
- **Browser support** — `wasm_bindgen` API for in-browser execution
- **WIT generation** — Generate WebAssembly Interface Types from Ark code

```bash
# Generate WIT interface definition
python meta/ark.py wit <file.ark>
```

---

## 34. FAQ

**Q: Why does Ark use both Rust and Python?**
Python provides a flexible bootstrap compiler ("The Brain"), while Rust provides a secure, high-performance execution engine ("The Engine"). This dual-runtime lets us iterate fast without sacrificing production safety.

**Q: Is Ark production-ready?**
The Core VM is stable. The Standard Library is active and growing. Everything is tested via the Gauntlet test suite. 286 tests pass across 10 CI jobs on 3 operating systems.

**Q: How is Ark different from Python/JavaScript?**
Ark is designed for *sovereign computing* — sandboxed by default, with built-in cryptography, blockchain access, and AI integration. It uses a capability-based security model instead of trusting all code unconditionally. It has a linear type system that prevents resource leaks at compile time, and enums/traits/pattern matching for type-safe domain modeling.

**Q: What happens if my code loops forever?**
The `ARK_EXEC_TIMEOUT` watchdog terminates the process after 5 seconds (configurable).

**Q: Can I write web servers in Ark?**
Yes. See `apps/server.ark` for a working HTTP server example.

**Q: Can I build smart contracts?**
Ark has chain intrinsics for interacting with Ethereum-compatible blockchains. See [Blockchain](#23-blockchain).

**Q: What are the compilation targets?**
Three: (1) Bytecode VM (fastest iteration), (2) Native WASM (production deployment), (3) Tree-walker interpreter (testing).

**Q: Does Ark have enums and traits like Rust?**
Yes. Enums with variant fields, traits with method signatures, and `impl Trait for Type` blocks are all implemented across all compiler backends. See [Enums](#10-enums), [Traits](#12-traits--impl-blocks).

---

**© 2026 Sovereign Systems. All rights reserved.**
