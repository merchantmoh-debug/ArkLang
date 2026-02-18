# Ark Quick Start

## Installation

Ark is a Python-based language. To run Ark, you need Python 3.11+.

1. Clone the repository:

   ```bash
   git clone https://github.com/merchantmoh-debug/ark-compiler.git
   cd ark-compiler
   ```

2. Install Python dependencies:

   ```bash
   pip install -r requirements.txt
   ```

3. Verify your setup:

   ```bash
   python meta/ark.py version
   ```

## Hello World

Create a file named `hello.ark`:

```ark
print("Hello, World!")
```

Run it:

```bash
python meta/ark.py run hello.ark
```

## Basic Syntax

### Variables

Ark uses `:=` for assignment (reassignment is allowed with `:=`).

```ark
x := 10
y := "Ark"
```

### Functions

```ark
func add(a, b) {
    return a + b
}

result := add(3, 7)
print(result)  // 10
```

### Control Flow

```ark
if x > 5 {
    print("Big")
} else {
    print("Small")
}

i := 0
while i < 10 {
    print(i)
    i := i + 1
}
```

## AI Integration

Ark has built-in AI capabilities. Set up your API key:

```bash
# Google Gemini
set GOOGLE_API_KEY=your-key-here

# Or local Ollama
set ARK_LLM_ENDPOINT=http://localhost:11434/v1/chat/completions
```

Create a file named `hello_ai.ark`:

```ark
// Direct AI call
answer := sys.ai.ask("What is 2 + 2?")
print(answer)

// Agent with persona
sys.vm.source("lib/std/ai.ark")
math_tutor := Agent.new("You are a math tutor. Explain step by step.")
response := math_tutor.chat("Solve x^2 - 5x + 6 = 0")
print(response)
```

Run it:

```bash
python meta/ark.py run hello_ai.ark
```

> Without an API key, AI calls return a graceful fallback message instead of crashing.

## Available Commands

| Command | Description |
| --- | --- |
| `python meta/ark.py run <file.ark>` | Execute an Ark program |
| `python meta/ark.py repl` | Start the interactive REPL |
| `python meta/ark.py version` | Print the compiler version |
| `python meta/ark.py compile <file.ark>` | Compile to bytecode |

## Docker (Recommended)

```bash
docker build -t ark-compiler .
docker run -it --rm ark-compiler
```
