# Ark Quick Start

## Installation

Ark is a Python-based language. To run Ark, you need Python 3.10+.

1. Clone the repository:
   ```bash
   git clone https://github.com/your-repo/ark.git
   cd ark
   ```

2. Run the Ark CLI:
   ```bash
   python meta/ark.py --help
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
