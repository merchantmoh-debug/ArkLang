
# The Ark-1 Programmer's Field Manual

**Version:** Omega-Point v113.5
**Architect:** Mohamad Al-Zawahreh (Sovereign Systems)
**Status:** ACTIVE / SOVEREIGN

---

## 🌌 Introduction

**Ark** is a Neuro-Symbolic programming language designed for the Sovereign Individual. It offers a **Dual-Runtime Architecture**:
1.  **The Genesis Engine (Python):** Rapid prototyping, **AI Swarm** integration, and bootstrapping.
2.  **The Prime Engine (Rust):** High-performance, **Linear Type** enforced, memory-safe execution via the Ark Virtual Machine (AVM).

### The Philosophy
1.  **Kinetic Syntax:** Code flows from Left to Right. Assignments are explicit actions (`:=`).
2.  **Safety via Linearity:** Resources (Buffers, Sockets) must be used exactly once. No Garbage Collection.
3.  **Neuro-Symbolic:** The runtime has direct access to AI models (`intrinsic_ask_ai`), allowing code to "think" while remaining verifiable.

---

## 🛠️ Installation & Setup

Ark is distributed as a source-available sovereign stack.

### Prerequisites
1.  **Rust**: [Install Rust](https://rustup.rs/) (Required for the VM).
2.  **Python 3.10+**: Required for the Bootstrap Compiler and Swarm.
3.  **Docker** (Optional): For the Sanctuary environment.

### Environment Variables
To unlock the full power of Ark, you must configure your environment:

| Variable | Description | Required For |
| :--- | :--- | :--- |
| `ARK_API_KEY` | API Key for Sovereign Intelligence (OpenAI/Anthropic/DeepSeek). | `intrinsic_ask_ai`, Swarm |
| `ARK_LLM_ENDPOINT` | URL for LLM (Default: `http://localhost:11434/v1`). | Local LLM (Ollama) |
| `ALLOW_DANGEROUS_LOCAL_EXECUTION` | Set to `"true"` to enable `sys.exec` and file writes. | File I/O, Shell Commands |

### Cloning & Building
```bash
git clone https://github.com/merchantmoh-debug/ark-compiler.git
cd ark-compiler
```

---

## ⚡ Execution Modes

### 1. The Swarm (Sovereign Intelligence)
The Swarm is a team of AI agents that write code for you.
```bash
# 1. Configure
export ARK_API_KEY="sk-..."

# 2. Summon
python3 meta/swarm_bridge.py
```

### 2. The Spec Engine (Text-to-Reality)
Compile English prompts into executable Ark code.
```bash
python3 meta/ark.py run apps/spec.ark
# Input: "Create a Fibonacci generator and write it to fib.ark"
```

### 3. The Interpreter (Neuro-Bridge)
Run `.ark` files directly. Best for development.
```bash
python3 meta/ark.py run apps/hello.ark
```

### 4. The Compiler (Silicon Heart)
Compile to JSON MAST (Merkle-ized AST) and execute on the Rust VM.
```bash
# 1. Compile
python3 meta/compile.py apps/hello.ark hello.json

# 2. Run (Rust Core)
cd core
cargo run --bin ark_loader -- ../hello.json
```

### 5. The Docker Sanctuary (Isolation)
Run everything inside a clean container.
```bash
docker build -t ark-compiler .
docker run -it --rm ark-compiler
```

---

## ⚡ Basic Syntax

### Variables
Ark uses `:=` for assignment.

```ark
x := 10
name := "Sovereign"
is_active := true
```

### Primitives
- **Integer**: 64-bit signed integers (`10`, `-5`).
- **String**: UTF-8 strings (`"Hello"`).
- **Boolean**: `true` or `false`.
- **Unit**: The empty value (returned by void functions).

### Arithmetic & Logic
Standard operators work as expected:
```ark
sum := 10 + 20
diff := 50 - sum
prod := 2 * 3
quot := 10 / 2
is_equal := 10 == 10
check := (10 > 5) && (2 < 4)
```

---

## 🔄 Control Flow

### If / Else
```ark
power := 9001
if power > 9000 {
    print("It's over 9000!")
} else {
    print("Weak.")
}
```

### While Loop
```ark
count := 5
while count > 0 {
    print(count)
    count := count - 1
}
```

---

## 📦 Ownership & Linear Semantics

Ark uses **Pass-by-Value** (Copy) semantics in the VM for most types, but **Linear Semantics** for Buffers and system resources.

### Buffers (Linear Types)
Buffers are raw byte arrays. They must be handled linearly (consumed and returned) to ensure safety without Garbage Collection.

```ark
// Alloc
buf := sys.mem.alloc(1024)

// Write (Consumes 'buf', returns new 'buf')
buf := sys.mem.write(buf, 0, 255)

// Read (Consumes 'buf', returns [value, buf])
res := sys.mem.read(buf, 0)
val := res[0]
buf := res[1] // Re-assign buffer to keep using it

// Free (Must be freed explicitly)
sys.mem.free(buf)
```

---

## 🧠 Intrinsics (Standard Library)

### AI & Neuro-Symbolic
| Function | Description |
| :--- | :--- |
| `intrinsic_ask_ai(prompt)` | Queries Sovereign AI (uses `ARK_API_KEY`). |

### System (Requires Security Flags)
| Function | Description |
| :--- | :--- |
| `sys.exec(cmd)` | Executes a shell command (Security Warning!). |
| `sys.fs.write(path, data)` | Writes string to file. |
| `sys.fs.read(path)` | Reads string from file. |

### Memory (Linear)
| Function | Description |
| :--- | :--- |
| `sys.mem.alloc(size)` | Allocates a byte buffer (Linear). |
| `sys.mem.write(buf, idx, val)` | Writes byte (Consumes/Returns). |
| `sys.mem.free(buf)` | Frees memory (Final Consumer). |

---

## 🎓 Complete Example: The Sovereign Shell

```ark
print("Starting Sovereign Shell...")

while true {
    // 1. Prompt User
    prompt := "root@ark> "
    
    // 2. Read Input (Simulated)
    // input := sys.io.read_line() 
    
    // 3. AI Copilot
    // insight := intrinsic_ask_ai("Explain: " + input)
    // print(insight)
    
    // 4. Exec
    // output := sys.exec(input)
    // print(output)
}
```

---

**© 2026 Sovereign Systems**
