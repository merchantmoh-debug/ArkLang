# Ark Language Specification v1.1.0

> **Status:** Living Document  
> **Author:** Mohamad Al-Zawahreh  
> **License:** DUAL-LICENSED (AGPLv3 or Commercial)  
> **Patent:** US Patent App #63/935,467

---

## 1. Lexical Structure

### 1.1 Source Encoding

Ark source files use UTF-8 encoding. The canonical file extension is `.ark`.

### 1.2 Comments

```ark
// Line comment (ignored by parser)
/// Doc comment (attached to next declaration)
/* Block comment -- may be nested */
```

Doc comments (`///`) are preserved in the AST and can be queried by tooling such as the LSP.

### 1.3 Keywords

The following identifiers are reserved:

| Category | Keywords |
|----------|----------|
| Declarations | `let` `mut` `func` `class` `struct` `import` |
| Control Flow | `if` `else` `while` `for` `in` `return` `break` `continue` |
| Pattern Matching | `match` |
| Error Handling | `try` `catch` |
| Literals | `true` `false` `nil` |
| Logic | `and` `or` |
| Concurrency | `async` `await` |

### 1.4 Identifiers

```
IDENTIFIER = [a-zA-Z_][a-zA-Z0-9_]*
```

Identifiers are case-sensitive. Convention: `snake_case` for variables and functions, `PascalCase` for classes and structs.

### 1.5 Literals

#### Integer
```
INTEGER = [0-9]+
```
Integers are signed 64-bit (`i64`). Examples: `0`, `42`, `1000000`.

#### Float
```
FLOAT = [0-9]+ "." [0-9]+
```
Floats are 64-bit IEEE 754 (`f64`). Examples: `3.14`, `0.001`.

> **Note:** Negative numeric literals are parsed as unary negation applied to a positive literal: `-42` is `neg(42)`.

#### String
```ark
"hello world"           // Regular string
f"Hello {name}!"        // F-string (interpolation)
"""multi
line
string"""               // Multi-line string
```

Escape sequences in regular strings: `\n` `\t` `\r` `\\` `\"`.

F-strings support arbitrary expressions inside `{}` braces.

#### Boolean
```ark
true
false
```

#### Nil
```ark
nil
```
Represents the absence of a value. Equivalent to `Unit` in the type system.

### 1.6 Operators

Listed from **lowest** to **highest** precedence:

| Precedence | Operator | Description | Associativity |
|-----------|----------|-------------|---------------|
| 1 (lowest) | `\|>` | Pipe | Left |
| 2 | `\|\|` / `or` | Logical OR | Left |
| 3 | `&&` / `and` | Logical AND | Left |
| 4 | `==` `!=` `<` `>` `<=` `>=` | Comparison | Left |
| 5 | `..` `..=` | Range (exclusive / inclusive) | None |
| 6 | `+` `-` | Addition, Subtraction | Left |
| 7 | `*` `/` `%` | Multiplication, Division, Modulo | Left |
| 8 | `!` `-` `~` | Unary NOT, Negate, Bitwise NOT | Right (prefix) |
| 9 (highest) | `.` `?.` `()` `[]` | Member access, Optional chain, Call, Index | Left (postfix) |

#### Assignment Operators
```ark
x := 10         // Assignment (declare + bind)
x += 1          // Compound: add-assign
x -= 1          // Compound: sub-assign
x *= 2          // Compound: mul-assign
x /= 2          // Compound: div-assign
```

> **Design Note:** Ark uses `:=` for assignment, not `=`. A bare `=` is a syntax error with a helpful message: *"Did you mean ':=' for assignment or '==' for comparison?"*

### 1.7 Delimiters

`(` `)` `{` `}` `[` `]` `,` `.` `:` `;`

---

## 2. Type System

### 2.1 Primitive Types

| Type | Description | Runtime Value |
|------|-------------|---------------|
| `Integer` | Signed 64-bit integer | `Value::Integer(i64)` |
| `Float` | 64-bit IEEE 754 float | Stored as `Value::String` (literal repr) |
| `String` | UTF-8 string | `Value::String(String)` |
| `Boolean` | `true` or `false` | `Value::Boolean(bool)` |
| `Unit` | Absence of value (`nil`) | `Value::Unit` |

### 2.2 Composite Types

| Type | Syntax | Runtime Value |
|------|--------|---------------|
| `List<T>` | `[1, 2, 3]` | `Value::List(Vec<Value>)` |
| `Map<K, V>` | (via intrinsics) | `Value::Struct(HashMap)` |
| `Struct` | `{ name: "Ark", version: 1 }` | `Value::Struct(HashMap<String, Value>)` |
| `Buffer` | (via intrinsics) | `Value::Buffer(Vec<u8>)` |
| `Function` | `func(x) { x + 1 }` | `Value::Function(Arc<Chunk>)` |
| `Optional<T>` | Type-level only | `T \| nil` |

### 2.3 Special Types

| Type | Description |
|------|-------------|
| `Any` | Accepts any type (dynamic typing fallback) |
| `Unknown` | Not yet inferred (internal to type checker) |
| `Linear(name)` | Linear resource -- must be consumed exactly once |
| `Affine(name)` | Affine resource -- consumed at most once |
| `Shared(name)` | Freely copyable reference |

### 2.4 Linear Type System

Ark enforces **linear type discipline** for resource safety. A linear value must be consumed (used) **exactly once**. The `LinearChecker` enforces these rules at compile time.

**Rules:**
1. A linear variable must be used exactly once before it goes out of scope
2. **Double-use** is an error: `Linear variable 'x' already consumed`
3. **Unused resource** is an error: `Linear variable 'x' dropped without consumption`
4. Linear values cannot be captured in closures by reference
5. Passing a linear value to a function **moves** it (consumes the binding)

**Example:**
```ark
// Linear resource -- file handle
handle := open_file("data.txt")    // handle is linear
content := read(handle)             // handle consumed here
// handle cannot be used again -- enforced at compile time
```

**Linearity at runtime:** `Value::is_linear()` returns `true` for `List`, `LinearObject`, `Buffer`, and `Struct`. Integers, Booleans, Strings, Functions, and Unit are **shared** (freely copyable).

### 2.5 Type Compatibility

Type checking uses structural compatibility with the following rules:
- `Any` is compatible with every type
- `Unknown` is compatible with every type (unifies during inference)
- `List<T>` is compatible with `List<U>` if `T` is compatible with `U`
- `Map<K1, V1>` is compatible with `Map<K2, V2>` if both key and value types are compatible
- `Function(P, R)` is compatible if parameter and return types are compatible
- `Optional<T>` is compatible with `T` and with `Unit`

---

## 3. Statements

### 3.1 Variable Declaration

```ark
x := 42                  // Immutable binding
name := "Ark"            // Type inferred
```

> **Note:** All bindings use `:=`. There is no separate `let` keyword required for simple assignments (the parser treats `IDENT := EXPR` as an assignment statement). The `let` keyword is used for destructuring.

### 3.2 Destructuring

```ark
let (a, b, c) := get_triple()
```

Destructures a list or tuple into multiple bindings. Each name is bound to the corresponding element.

### 3.3 Compound Assignment

```ark
counter += 1
counter -= 1
scale *= 2.0
ratio /= 10
```

### 3.4 Field Assignment

```ark
point.x := 10
config.timeout := 30
```

---

## 4. Control Flow

### 4.1 If / Else If / Else

```ark
if condition {
    // then block
} else if other_condition {
    // else-if block
} else {
    // else block
}
```

Conditions are expressions; no parentheses required around them. Braces are mandatory.

### 4.2 While Loop

```ark
while condition {
    // body
}
```

### 4.3 For Loop

```ark
for item in collection {
    // body -- item bound per iteration
}
```

### 4.4 Break / Continue

```ark
while true {
    if done {
        break
    }
    if skip {
        continue
    }
}
```

### 4.5 Match

```ark
match value {
    1 => print("one"),
    2 => {
        print("two")
        do_something()
    },
    _ => print("other")
}
```

Match arms use `=>`. Arms with a single expression use a trailing comma. Arms with multiple statements use a block `{ }`.

### 4.6 Try / Catch

```ark
try {
    result := risky_operation()
} catch err {
    print(f"Error: {err}")
}
```

The `catch` clause binds the error to a named variable.

### 4.7 Return

```ark
func double(x) {
    return x * 2
}
```

`return` immediately exits the current function with the given value.

---

## 5. Functions

### 5.1 Function Definition

```ark
func greet(name) {
    print(f"Hello, {name}!")
}

/// Computes factorial recursively
func factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
```

- Functions are declared with `func`
- Parameters are untyped by default (type `Any`)
- The function body is a block `{ ... }`
- Doc comments (`///`) attach to the following function

### 5.2 Lambda Expressions

```ark
// Closure syntax with pipes
square := |x| { x * x }

// Alternative: func keyword
add := func(a, b) { a + b }
```

Lambdas capture variables from the enclosing scope.

### 5.3 Pipe Operator

```ark
data |> transform |> validate |> save
```

The pipe operator `|>` passes the left-hand side as the first argument to the right-hand function:

```ark
// Equivalent to:
save(validate(transform(data)))
```

### 5.4 Intrinsics

Ark provides **109 built-in intrinsic functions** spanning I/O, math, string manipulation, networking, cryptography, AI, and system operations. Intrinsics are compiled directly to VM opcodes or native function calls.

Key intrinsics include:

| Category | Examples |
|----------|----------|
| I/O | `print`, `read_file`, `write_file`, `read_input` |
| Math | `abs`, `sqrt`, `pow`, `floor`, `ceil`, `random` |
| String | `len`, `split`, `join`, `replace`, `trim`, `upper`, `lower` |
| List | `push`, `pop`, `map`, `filter`, `reduce`, `sort`, `slice` |
| Type | `type_of`, `to_int`, `to_float`, `to_string` |
| Network | `http_get`, `http_post`, `socket_bind`, `socket_connect` |
| Crypto | `hash_sha256`, `hmac_sha256`, `base64_encode` |
| System | `env_var`, `exec`, `sleep`, `time_now`, `exit` |
| AI | `ai_prompt`, `ai_embed`, `ai_classify` |

See `docs/API_REFERENCE.md` for the complete intrinsic catalog.

---

## 6. Classes and Structs

### 6.1 Struct Initialization

```ark
point := { x: 10, y: 20 }
```

Structs are anonymous records -- key-value maps created with brace syntax.

### 6.2 Class Definition

```ark
class Counter {
    func new(initial) {
        self.count := initial
    }

    func increment() {
        self.count += 1
    }

    func value() {
        return self.count
    }
}
```

Classes are syntactic sugar for named structs with associated methods. At the AST level, a `class` is lowered to a `StructDecl` with method fields.

### 6.3 Member Access

```ark
point.x             // Field access
point.x := 30       // Field mutation
obj?.maybe_field    // Optional chaining (returns nil if obj is nil)
list[0]             // Index access
```

---

## 7. Modules

### 7.1 Import

```ark
import math
import std.collections
import crypto.hash
```

Import paths use dot notation. The module resolver searches:
1. Standard library (`std/`)
2. Local project modules (relative to project root)
3. Installed packages (`ark_modules/`)

### 7.2 Standard Library Namespaces

| Namespace | Contents |
|-----------|----------|
| `std.io` | File I/O, console I/O |
| `std.math` | Mathematical functions |
| `std.string` | String utilities |
| `std.collections` | List, Map, Set operations |
| `std.net` | HTTP, sockets, networking |
| `std.crypto` | Hashing, HMAC, encoding |
| `std.json` | JSON parse / stringify |
| `std.time` | Timestamps, duration, sleep |
| `std.ai` | AI inference, embeddings |

See `docs/STDLIB_REFERENCE.md` for the complete standard library.

---

## 8. Expressions

### 8.1 Arithmetic

All binary operators desugar to function calls at the AST level:

| Expression | Desugared Form |
|-----------|----------------|
| `a + b` | `Call("add", [a, b])` |
| `a - b` | `Call("sub", [a, b])` |
| `a * b` | `Call("mul", [a, b])` |
| `a / b` | `Call("div", [a, b])` |
| `a % b` | `Call("modulo", [a, b])` |

### 8.2 Comparison

| Expression | Desugared Form |
|-----------|----------------|
| `a == b` | `Call("eq", [a, b])` |
| `a != b` | `Call("neq", [a, b])` |
| `a < b` | `Call("lt", [a, b])` |
| `a > b` | `Call("gt", [a, b])` |
| `a <= b` | `Call("le", [a, b])` |
| `a >= b` | `Call("ge", [a, b])` |

### 8.3 Boolean

| Expression | Desugared Form |
|-----------|----------------|
| `a && b` / `a and b` | `Call("and", [a, b])` |
| `a \|\| b` / `a or b` | `Call("or", [a, b])` |
| `!a` | `Call("not", [a])` |

### 8.4 Range

```ark
0..10       // Exclusive range: [0, 10)
0..=10      // Inclusive range: [0, 10]
```

Ranges are used primarily in `for` loops and slice operations.

### 8.5 List Literal

```ark
empty := []
numbers := [1, 2, 3, 4, 5]
mixed := ["hello", 42, true]
```

### 8.6 Method Calls

```ark
list.push(item)
text.split(",")
obj.method(arg1, arg2)
```

Method calls desugar to `Call("method_name", [obj, ...args])`.

---

## 9. Security Model

### 9.1 Security Levels

The VM enforces a capability-based security model with configurable trust levels:

| Level | Name | Behavior |
|-------|------|----------|
| 0 | Unrestricted | All code executes (development mode) |
| 1 | Trusted Only | Only code with trusted hash may execute |
| 2+ | Sandboxed | Restricted intrinsic access (network, file I/O disabled) |

Set via environment variable:
```bash
ARK_SECURITY_LEVEL=1 ark run program.ark
```

### 9.2 Content-Addressed Code (MAST)

Every compiled function body is wrapped in a `MastNode` -- a Merkle-Authenticated Syntax Tree node. The `MastNode` contains:

- `content`: The `ArkNode` (AST)
- `hash`: SHA-256 hash of the serialized content

This enables:
- **Integrity verification** -- detect tampering
- **Trust chains** -- only execute code with known hashes
- **Deterministic compilation** -- same source always produces the same hash

### 9.3 Resource Limits

| Resource | Default Limit | Control |
|----------|---------------|---------|
| Stack depth | 10,000 frames | `MAX_STACK_DEPTH` |
| Execution steps | 10,000,000 ops | `MAX_STEPS` |
| Memory | 256 MB | `MAX_MEMORY_MB` |

Exceeding any limit triggers a controlled error, not a crash.

### 9.4 Resource Tracking

The runtime tracks all allocated resources (file handles, sockets, buffers) via `ResourceTracker`. On shutdown or Ctrl+C, all unclosed resources are cleaned up automatically with a warning:

```
Warning: Resource 3 (type file_handle) was not closed explicitly.
```

---

## 10. Runtime Semantics

### 10.1 Evaluation Order

- Expressions evaluate left-to-right
- Function arguments evaluate left-to-right before the call
- Short-circuit evaluation applies to `&&` and `||`
- The pipe operator `|>` evaluates left-to-right

### 10.2 Scope Rules

Ark uses **lexical scoping** with a chain of `Scope` frames:

1. Each function call creates a new scope
2. Variables are resolved by walking the scope chain upward
3. Inner scopes shadow outer bindings
4. Linear variables are **moved** on access (removed from scope)
5. Shared variables are **cloned** on access

### 10.3 Function Calls

When a function is called:
1. Arguments are pushed onto the operand stack
2. A new `CallFrame` is pushed onto the call stack
3. A new `Scope` is created for the function body
4. Parameters are bound to arguments via `Store` opcodes
5. The body executes
6. `Ret` pops the call frame and pushes the return value

### 10.4 Error Handling

Runtime errors are represented by the `RuntimeError` enum:

| Error | Cause |
|-------|-------|
| `VariableNotFound` | Accessing an undeclared or consumed variable |
| `TypeMismatch` | Operand types don't match the operation |
| `NotExecutable` | Calling a non-function value |
| `FunctionNotFound` | Named function not in scope |
| `StackUnderflow` | Pop from empty stack |
| `RecursionLimitExceeded` | Call depth exceeds `MAX_STACK_DEPTH` |
| `UntrustedCode` | Code hash not in trusted set (security level ≥ 1) |
| `AllocationError` | Memory limit exceeded |
| `ResourceError` | I/O or system resource failure |

### 10.5 Value Pool

For performance, small integers (−256 to +256) are cached in a thread-local pool. `ValuePool::pool_int()` returns cached instances, avoiding heap allocation for common values.

### 10.6 Graceful Shutdown

On `SIGINT` (Ctrl+C):
1. `SHUTTING_DOWN` flag is set atomically
2. `ResourceTracker::cleanup_all()` releases all tracked resources
3. Runtime statistics are printed if `ARK_RUNTIME_STATS` is set
4. Process exits after a 5-second grace period

---

## 11. Compilation Pipeline

```
┌──────────┐     ┌───────┐     ┌──────────┐     ┌──────┐     ┌────┐
│ .ark file │ ──▶ │ Lexer │ ──▶ │  Parser  │ ──▶ │ MAST │ ──▶ │ VM │
└──────────┘     └───────┘     └──────────┘     └──────┘     └────┘
                  Tokens        ArkNode AST      Bytecode      Execute
                                                 (Chunk)
```

1. **Lexer** -- Tokenizes source into `Token` stream (keywords, operators, literals, identifiers)
2. **Parser** -- Recursive descent parser produces `ArkNode` AST
3. **Compiler** -- Traverses AST, performs constant folding and dead code elimination, emits `OpCode` bytecodes into a `Chunk`
4. **VM** -- Stack-based virtual machine executes `Chunk` with scope chain, call frames, and intrinsic dispatch

### 11.1 Bytecode Instructions

| OpCode | Stack Effect | Description |
|--------|-------------|-------------|
| `Push(val)` | +1 | Push value onto stack |
| `Pop` | −1 | Discard top of stack |
| `Add` / `Sub` / `Mul` / `Div` / `Mod` | −1 | Binary arithmetic (pops 2, pushes 1) |
| `Eq` / `Neq` / `Gt` / `Lt` / `Ge` / `Le` | −1 | Binary comparison |
| `And` / `Or` / `Not` | −1 / 0 | Boolean logic |
| `Load(name)` | +1 | Load variable from scope chain |
| `Store(name)` | −1 | Store top-of-stack into current scope |
| `Jmp(addr)` | 0 | Unconditional jump |
| `JmpIfFalse(addr)` | −1 | Conditional jump (pops condition) |
| `Call(argc)` | −argc | Call function with argc arguments |
| `Ret` | varies | Return from function |
| `Print` | −1 | Print top-of-stack to stdout |
| `MakeList(n)` | −(n−1) | Collect n stack items into a list |
| `MakeStruct(n)` | −(2n−1) | Collect n key-value pairs into a struct |
| `GetField(name)` | 0 | Read field from struct on top of stack |
| `SetField(name)` | −1 | Write field to struct |
| `Destructure` | varies | Unpack list into named bindings |

---

## 12. EBNF Grammar Reference

The following is the complete context-free grammar, derived from `meta/ark.lark`:

```ebnf
program        = top_level_item+ ;
top_level_item = statement | function_def | class_def | doc_wrapper ;

statement      = assignment | flow_stmt | expression | import_stmt
               | match_stmt | try_stmt ;

(* Definitions *)
function_def   = [DOC_COMMENT] "func" IDENTIFIER "(" [param_list] ")" "{" block "}" ;
param_list     = IDENTIFIER ("," IDENTIFIER)* ;
class_def      = [DOC_COMMENT] "class" IDENTIFIER "{" function_def* "}" ;
doc_wrapper    = DOC_COMMENT ;

(* Control Flow *)
flow_stmt      = if_stmt | while_stmt | return_stmt ;
return_stmt    = "return" expression ;
if_stmt        = "if" expression "{" block "}"
                 ("else" "if" expression "{" block "}")*
                 ["else" "{" block "}"] ;
while_stmt     = "while" expression "{" block "}" ;
for_stmt       = "for" IDENTIFIER "in" expression "{" block "}" ;

(* Pattern Matching *)
match_stmt     = "match" expression "{" match_case* "}" ;
match_case     = pattern "=>" ("{" block "}" | expression [","]) ;
pattern        = expression ;

(* Error Handling *)
try_stmt       = "try" "{" block "}" "catch" IDENTIFIER "{" block "}" ;

block          = statement* ;

(* Assignments *)
assignment     = assign_var | assign_attr | assign_destructure | assign_op ;
assign_var     = IDENTIFIER ":=" expression ;
assign_destructure = "let" "(" IDENTIFIER ("," IDENTIFIER)* ")" ":=" expression ;
assign_attr    = atom "." IDENTIFIER ":=" expression ;
assign_op      = atom ASSIGN_OP expression ;

(* Expressions -- ordered by precedence, lowest first *)
expression     = pipe_expr ;
pipe_expr      = logical_or ("|>" logical_or)* ;
logical_or     = logical_and ("||" logical_and)* ;
logical_and    = comparison ("&&" comparison)* ;
comparison     = range_expr (COMP_OP range_expr)* ;
range_expr     = sum [(".." | "..=") sum] ;
sum            = product (("+" | "-") product)* ;
product        = unary (("*" | "/" | "%") unary)* ;
unary          = ("!" | "-" | "~") unary | atom ;
atom           = primary
               | atom "." IDENTIFIER
               | atom "?." IDENTIFIER
               | atom "(" [expr_list] ")"
               | atom "[" expression "]" ;

primary        = NUMBER | FSTRING | MULTI_STRING | STRING | IDENTIFIER
               | "(" expression ")"
               | "[" [expr_list] "]"
               | "{" [field_list] "}"
               | lambda_expr
               | "true" | "false" | "nil" ;

lambda_expr    = ("|" [param_list] "|" | "func" "(" [param_list] ")") "{" block "}" ;
field_list     = field_init ("," field_init)* ;
field_init     = IDENTIFIER ":" expression ;
expr_list      = expression ("," expression)* ;

(* Tokens *)
COMP_OP        = "==" | "!=" | "<" | ">" | "<=" | ">=" ;
ASSIGN_OP      = "+=" | "-=" | "*=" | "/=" ;
NUMBER         = [0-9]+ ("." [0-9]+)? ;
STRING         = '"' (escape_char | [^"\\])* '"' ;
FSTRING        = 'f"' (escape_char | [^"\\])* '"' ;
MULTI_STRING   = '"""' .* '"""' ;
IDENTIFIER     = [a-zA-Z_][a-zA-Z0-9_]* ;
DOC_COMMENT    = "///" [^\n]* ;
```

---

## Appendix A: CLI Reference

```bash
ark run <file.ark>         # Run source or MAST JSON
ark build <file.ark>       # Compile to native .wasm binary
ark run-wasm <file.wasm>   # Execute compiled WASM via wasmtime
ark check <file.ark>       # Static linear type checker
ark diagnose <file.ark>    # Diagnostic proof suite (cryptographic verification)
ark parse <file.ark>       # Dump AST as JSON
ark debug <file.ark>       # Interactive step-through debugger
ark repl                   # Interactive REPL
ark wit <file.ark>         # Generate WIT interface definition
ark adn <file.ark>         # Run and output in ADN format
```

## Appendix B: Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ARK_SECURITY_LEVEL` | `0` | VM security level (0=unrestricted, 1=trusted, 2+=sandboxed) |
| `ARK_RUNTIME_STATS` | unset | Print runtime statistics on exit |

## Appendix C: Source File Organization

```
core/src/
├── ast.rs          # AST node types (ArkNode, Statement, Expression)
├── bytecode.rs     # OpCode enum and Chunk struct
├── checker.rs      # Linear type checker
├── compiler.rs     # AST → bytecode compiler with optimizations
├── diagnostic.rs   # Diagnostic Proof Suite (Merkle + HMAC verification)
├── intrinsics.rs   # 109 built-in intrinsic functions
├── loader.rs       # JSON MAST loader with integrity verification
├── parser.rs       # Rust-native recursive descent parser
├── runtime.rs      # Value types, Scope, ResourceTracker, MemoryManager
├── types.rs        # ArkType enum and compatibility rules
├── vm.rs           # Stack-based virtual machine
└── bin/
    └── ark_loader.rs   # CLI binary (ark run/check/diagnose/parse/version)
```
