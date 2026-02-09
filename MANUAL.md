# The Ark-1 Programmer's Field Manual (Infix Edition)
**Version:** 1.0 (True-Code Edition)
**Syntax:** Kinetic Infix (Algol-style)

---

## 1. Philosophy
Ark uses a **Kinetic Syntax** where energy flows from Left to Right.
`Target := Source`

## 2. Variables
Use `:=` to assign values.
```ark
power := 9000
name := "Sovereign"
```

## 3. Control Flow
Blocks are defined by `{}`.

### If-Statement
```ark
if power > 8000 {
    print("It's over 8000!")
}
```

### While-Loop
```ark
x := 5
while x > 0 {
    print(x)
    x := x - 1
}
```

## 4. Intrinsics (System Functions)
Call them like standard functions.

- `print(arg)`: Output text.
- `len(list)`: Get length.
- `get(list, index)`: Get item.
- `intrinsic_ask_ai(prompt)`: Query Gemini.
- `intrinsic_extract_code(text)`: Parse code blocks.
- `sys.exec(cmd)`: Run shell command.
- `sys.fs.write(path, content)`: Write file.

## 5. Iron Hand Pattern
Example of code generation:
```ark
prompt := "Write a script."
blueprint := intrinsic_ask_ai(prompt)
blocks := intrinsic_extract_code(blueprint)
// ... use blocks ...
```

---
**Verified against `ark.lark` and `ark.py`.**
