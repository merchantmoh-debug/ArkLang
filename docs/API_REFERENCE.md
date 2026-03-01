# Ark API Reference (Intrinsics)

All intrinsics are compiled into the Ark runtime and available without imports.
Dotted intrinsics (e.g., `sys.crypto.hash`) are accessed through the system namespace.
Bare intrinsics (e.g., `print`, `len`) are available at top level.

## Table of Contents
- [Chain](#chain)
- [Core](#core)
- [Crypto](#crypto)
- [Fs](#fs)
- [Io](#io)
- [Json](#json)
- [List](#list)
- [Math](#math)
- [Mem](#mem)
- [Net](#net)
- [Str](#str)
- [Struct](#struct)
- [Sys](#sys)
- [Time](#time)
- [Z3](#z3)

---

## Chain

Blockchain interaction intrinsics. Operate on the internal Ark chain state or a configured JSON-RPC endpoint.

### `sys.chain.get_balance`
Returns the balance (in smallest unit) for the given address.

```ark
balance := sys.chain.get_balance("0xabc123...")
print("Balance:", balance)
```

### `sys.chain.height`
Returns the current block height of the connected chain.

```ark
h := sys.chain.height()
print("Current block:", h)
```

### `sys.chain.submit_tx`
Submits a signed raw transaction payload to the chain. Returns the transaction hash.

```ark
tx_hash := sys.chain.submit_tx(signed_payload)
print("Submitted:", tx_hash)
```

### `sys.chain.verify_tx`
Checks whether a transaction has been confirmed on-chain. Returns `true` or `false`.

```ark
confirmed := sys.chain.verify_tx("0xdeadbeef...")
if confirmed { print("Transaction confirmed") }
```

---

## Core

Built-in functions available at the top level without any namespace prefix.

### `exit`
Terminates the program immediately with exit code 0.

```ark
exit()
```

### `get`
Retrieves a value from a struct by key name. Returns `nil` if the key doesn't exist.

```ark
person := { name: "Alice", age: 30 }
name := get(person, "name")  // "Alice"
```

### `intrinsic_and`
Logical AND. Returns `true` if both arguments are truthy.

```ark
result := intrinsic_and(true, false)  // false
```

### `intrinsic_ask_ai`
Sends a prompt to a connected AI provider and returns the response string. Requires a live AI backend.

```ark
answer := intrinsic_ask_ai("Explain P vs NP in one sentence")
print(answer)
```

### `intrinsic_buffer_alloc`
Allocates a fixed-size byte buffer of `n` bytes, initialized to zero. Returns a buffer handle.

```ark
buf := intrinsic_buffer_alloc(1024)  // 1KB buffer
```

### `intrinsic_buffer_inspect`
Returns a debug representation of a buffer's contents and metadata.

```ark
info := intrinsic_buffer_inspect(buf)
print(info)
```

### `intrinsic_buffer_read`
Reads a byte value at the given offset from a buffer. Returns an integer (0–255).

```ark
byte_val := intrinsic_buffer_read(buf, 0)
```

### `intrinsic_buffer_write`
Writes a byte value (0–255) at the given offset in a buffer.

```ark
intrinsic_buffer_write(buf, 0, 255)
```

### `intrinsic_crypto_hash`
SHA-256 hash of a string. Returns hex-encoded 64-character digest. Alias for `sys.crypto.hash`.

```ark
h := intrinsic_crypto_hash("hello")
// "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
```

### `intrinsic_extract_code`
Extracts code blocks from a markdown-formatted AI response string. Useful for processing AI-generated code.

```ark
code := intrinsic_extract_code(ai_response)
```

### `intrinsic_ge`
Greater-than-or-equal comparison. Returns `true` if `a >= b`.

```ark
intrinsic_ge(5, 3)  // true
```

### `intrinsic_gt`
Greater-than comparison. Returns `true` if `a > b`.

```ark
intrinsic_gt(5, 3)  // true
```

### `intrinsic_le`
Less-than-or-equal comparison. Returns `true` if `a <= b`.

```ark
intrinsic_le(3, 5)  // true
```

### `intrinsic_len`
Returns the length of a string, list, or buffer. Alias for `len`.

```ark
intrinsic_len([1, 2, 3])  // 3
```

### `intrinsic_list_append`
Appends a value to the end of a list. Mutates the list in place.

```ark
items := [1, 2]
intrinsic_list_append(items, 3)  // items is now [1, 2, 3]
```

### `intrinsic_list_get`
Returns the element at index `i` from a list. Zero-indexed.

```ark
intrinsic_list_get([10, 20, 30], 1)  // 20
```

### `intrinsic_lt`
Less-than comparison. Returns `true` if `a < b`.

```ark
intrinsic_lt(3, 5)  // true
```

### `intrinsic_merkle_root`
Computes the Merkle root hash of a list of hex-encoded leaf hashes. Returns hex string.

```ark
root := intrinsic_merkle_root(["aabb...", "ccdd..."])
```

### `intrinsic_not`
Logical NOT. Returns `true` if the argument is falsy, `false` if truthy.

```ark
intrinsic_not(false)  // true
```

### `intrinsic_or`
Logical OR. Returns `true` if either argument is truthy.

```ark
intrinsic_or(false, true)  // true
```

### `intrinsic_time_now`
Returns the current Unix timestamp in seconds. Alias for `sys.time.now`.

```ark
ts := intrinsic_time_now()
```

### `len`
Returns the length of a string, list, or buffer.

```ark
len("hello")       // 5
len([1, 2, 3])     // 3
```

### `print`
Prints one or more values to stdout, separated by spaces, followed by a newline.

```ark
print("Hello", "World")  // Hello World
print(42)                 // 42
```

### `quit`
Alias for `exit()`. Terminates the program immediately.

```ark
quit()
```

---

## Crypto

Cryptographic operations implemented in the Ark runtime (Rust core + Python fallback). No external dependencies for core primitives.

### `sys.crypto.aes_gcm_decrypt`
AES-256-GCM authenticated decryption. Takes hex-encoded ciphertext+nonce+tag and a 32-byte hex key. Returns the plaintext string.

```ark
plain := sys.crypto.aes_gcm_decrypt(ciphertext_hex, key_hex)
```

### `sys.crypto.aes_gcm_encrypt`
AES-256-GCM authenticated encryption. Takes a plaintext string and a 32-byte hex key. Returns hex-encoded ciphertext with nonce and auth tag prepended.

```ark
ct := sys.crypto.aes_gcm_encrypt("secret message", key_hex)
```

### `sys.crypto.ed25519.gen`
Generates an Ed25519 keypair. Returns a struct with `public` and `secret` hex-encoded keys.

```ark
kp := sys.crypto.ed25519.gen()
print("Public:", kp.public)
print("Secret:", kp.secret)
```

### `sys.crypto.ed25519.sign`
Signs a message with an Ed25519 secret key. Returns hex-encoded 64-byte signature.

```ark
sig := sys.crypto.ed25519.sign("message", kp.secret)
```

### `sys.crypto.ed25519.verify`
Verifies an Ed25519 signature. Returns `true` if the signature is valid.

```ark
valid := sys.crypto.ed25519.verify(sig, "message", kp.public)
```

### `sys.crypto.hash`
SHA-256 hash of a string. Returns hex-encoded 64-character digest.

```ark
h := sys.crypto.hash("hello world")
// "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
```

### `sys.crypto.hmac_sha512`
HMAC-SHA512 message authentication code. Takes a key and message string. Returns hex-encoded MAC.

```ark
mac := sys.crypto.hmac_sha512("key", "data")
```

### `sys.crypto.merkle_root`
Computes the Merkle root of a list of hex-encoded leaf hashes using SHA-256.

```ark
root := sys.crypto.merkle_root(["leaf1_hex", "leaf2_hex", "leaf3_hex"])
```

### `sys.crypto.pbkdf2_hmac_sha512`
PBKDF2 key derivation using HMAC-SHA512. Takes password, salt, and iteration count.

```ark
key := sys.crypto.pbkdf2_hmac_sha512("password", "salt", 2048)
```

### `sys.crypto.random_bytes`
Generates `n` cryptographically secure random bytes. Returns a list of integers (0–255).

```ark
bytes := sys.crypto.random_bytes(32)  // 32 random bytes
```

### `sys.crypto.sha512`
SHA-512 hash of a string. Returns hex-encoded 128-character digest.

```ark
h := sys.crypto.sha512("data")
```

---

## Fs

File system operations. Requires `fs_read` and/or `fs_write` capability tokens. All paths are sandboxed -- path traversal is blocked at the runtime level.

### `sys.fs.read`
Reads the entire contents of a file as a UTF-8 string. Requires `fs_read` capability.

```ark
content := sys.fs.read("config.json")
print(content)
```

### `sys.fs.read_buffer`
Reads a file as raw bytes. Returns a list of byte values (0–255). Requires `fs_read` capability.

```ark
bytes := sys.fs.read_buffer("image.png")
print("Size:", len(bytes), "bytes")
```

### `sys.fs.write`
Writes a UTF-8 string to a file, creating it if it doesn't exist. Overwrites existing content. Requires `fs_write` capability.

```ark
sys.fs.write("output.txt", "Hello, Ark!")
```

### `sys.fs.write_buffer`
Writes raw bytes (list of 0–255 integers) to a file. Requires `fs_write` capability.

```ark
sys.fs.write_buffer("output.bin", [0x48, 0x65, 0x6C, 0x6C, 0x6F])
```

---

## Io

Standard I/O stream operations for interactive programs.

### `sys.io.read_bytes`
Reads up to `n` bytes from stdin. Returns a list of byte values. Blocks until data is available.

```ark
data := sys.io.read_bytes(256)
```

### `sys.io.read_line`
Reads a single line from stdin (blocking). Returns the line as a string without trailing newline.

```ark
name := sys.io.read_line()
print("Hello,", name)
```

### `sys.io.write`
Writes a string to stdout without a trailing newline. Use for raw output control.

```ark
sys.io.write("Loading...")
```

---

## Json

JSON serialization and deserialization.

### `sys.json.parse`
Parses a JSON string into an Ark value (struct, list, string, number, bool, or nil).

```ark
data := sys.json.parse("{\"name\": \"Alice\", \"age\": 30}")
print(data.name)  // "Alice"
```

### `sys.json.stringify`
Serializes an Ark value (struct, list, string, number, bool) into a JSON string.

```ark
json_str := sys.json.stringify({ name: "Alice", age: 30 })
// "{\"name\": \"Alice\", \"age\": 30}"
```

---

## List

Mutable list operations. Lists are zero-indexed.

### `sys.list.append`
Appends a value to the end of a list. Mutates the list in place.

```ark
items := [1, 2, 3]
sys.list.append(items, 4)  // items is now [1, 2, 3, 4]
```

### `sys.list.delete`
Removes the element at the given index from a list. Shifts subsequent elements left.

```ark
items := [10, 20, 30]
sys.list.delete(items, 1)  // items is now [10, 30]
```

### `sys.list.get`
Returns the element at the given index. Zero-indexed.

```ark
val := sys.list.get([10, 20, 30], 2)  // 30
```

### `sys.list.pop`
Removes and returns the last element of a list.

```ark
items := [1, 2, 3]
last := sys.list.pop(items)  // last = 3, items = [1, 2]
```

### `sys.list.set`
Sets the element at the given index to a new value. Mutates the list in place.

```ark
items := [10, 20, 30]
sys.list.set(items, 1, 99)  // items is now [10, 99, 30]
```

---

## Math

Mathematical functions. Scalar trig functions operate on floating-point values. Tensor operations work on `math.Tensor` structs.

### `math.Tensor`
Creates a Tensor from a flat data list and a shape list.

```ark
t := math.Tensor([1, 2, 3, 4], [2, 2])  // 2x2 matrix
```

### `math.acos`
Arc cosine. Returns the angle in radians whose cosine is `x`.

```ark
angle := math.acos(0.5)  // ~1.0472 (π/3)
```

### `math.add`
Element-wise tensor addition. Both tensors must have the same shape.

```ark
a := math.Tensor([1, 2], [2])
b := math.Tensor([3, 4], [2])
c := math.add(a, b)  // Tensor([4, 6], [2])
```

### `math.asin`
Arc sine. Returns the angle in radians whose sine is `x`.

```ark
angle := math.asin(0.5)  // ~0.5236 (π/6)
```

### `math.atan`
Arc tangent. Returns the angle in radians whose tangent is `x`.

```ark
angle := math.atan(1.0)  // ~0.7854 (π/4)
```

### `math.atan2`
Two-argument arc tangent. Returns the angle in radians of the point `(x, y)`.

```ark
angle := math.atan2(1.0, 1.0)  // ~0.7854 (π/4)
```

### `math.cos`
Cosine of an angle in radians.

```ark
val := math.cos(0)  // 1.0
```

### `math.cos_scaled`
Fixed-point cosine. Returns `cos(x) * 10000` as an integer (avoids floating-point in integer-only contexts).

```ark
val := math.cos_scaled(0)  // 10000
```

### `math.dot`
Dot product of two 1D vectors. Element-wise multiply and sum.

```ark
result := math.dot([1, 2, 3], [4, 5, 6])  // 32
```

### `math.matmul`
Matrix multiplication. A=[m,k], B=[k,n] → C=[m,n].

```ark
A := math.Tensor([1, 2, 3, 4], [2, 2])
B := math.Tensor([5, 6, 7, 8], [2, 2])
C := math.matmul(A, B)  // [[19, 22], [43, 50]]
```

### `math.mul_scalar`
Multiplies every element of a tensor by a scalar value.

```ark
t := math.Tensor([1, 2, 3], [3])
result := math.mul_scalar(t, 10)  // Tensor([10, 20, 30])
```

### `math.pi_scaled`
Returns π * 10000 as an integer (31416). Useful for fixed-point trig without floats.

```ark
pi := math.pi_scaled()  // 31416
```

### `math.pow`
Raises `base` to the power of `exp`.

```ark
val := math.pow(2, 10)  // 1024
```

### `math.sin`
Sine of an angle in radians.

```ark
val := math.sin(3.14159 / 2)  // ~1.0
```

### `math.sin_scaled`
Fixed-point sine. Returns `sin(x) * 10000` as an integer.

```ark
val := math.sin_scaled(15708)  // ~10000 (sin(π/2) scaled)
```

### `math.sqrt`
Square root of a number.

```ark
val := math.sqrt(144)  // 12
```

### `math.sub`
Element-wise tensor subtraction.

```ark
c := math.sub(a, b)
```

### `math.tan`
Tangent of an angle in radians.

```ark
val := math.tan(0.7854)  // ~1.0
```

### `math.transpose`
Matrix transpose. T=[m,n] → T'=[n,m].

```ark
t := math.Tensor([1, 2, 3, 4, 5, 6], [2, 3])
t2 := math.transpose(t)  // shape [3, 2]
```

### `sys.math.pow_mod`
Modular exponentiation: `(base^exp) mod modulus`. Essential for cryptographic operations.

```ark
result := sys.math.pow_mod(2, 10, 1000)  // 24
```

---

## Mem

Low-level memory buffer operations for binary data processing.

### `sys.mem.alloc`
Allocates a byte buffer of `n` bytes, initialized to zero. Returns a buffer handle.

```ark
buf := sys.mem.alloc(4096)
```

### `sys.mem.inspect`
Returns a debug representation of a buffer's contents, size, and allocation metadata.

```ark
info := sys.mem.inspect(buf)
print(info)
```

### `sys.mem.read`
Reads a byte (0–255) at the given offset from a buffer.

```ark
val := sys.mem.read(buf, 0)
```

### `sys.mem.write`
Writes a byte (0–255) at the given offset in a buffer.

```ark
sys.mem.write(buf, 0, 0xFF)
```

---

## Net

Network I/O: HTTP client and raw TCP sockets.

### `sys.net.http.request`
Sends an HTTP request. Takes method (`"GET"`, `"POST"`, etc.), URL, and optional body. Returns the response body as a string.

```ark
body := sys.net.http.request("GET", "https://api.example.com/data", "")
data := sys.json.parse(body)
```

### `sys.net.socket.accept`
Accepts an incoming connection on a bound socket. Returns a new socket handle for the client.

```ark
client := sys.net.socket.accept(server_socket)
```

### `sys.net.socket.bind`
Binds a TCP socket to an address and port. Returns a socket handle.

```ark
sock := sys.net.socket.bind("0.0.0.0", 8080)
```

### `sys.net.socket.close`
Closes a socket connection and releases the file descriptor.

```ark
sys.net.socket.close(sock)
```

### `sys.net.socket.connect`
Connects to a remote TCP server. Returns a socket handle.

```ark
sock := sys.net.socket.connect("example.com", 80)
```

### `sys.net.socket.recv`
Receives data from a connected socket. Returns a string. Blocks until data arrives.

```ark
data := sys.net.socket.recv(sock)
```

### `sys.net.socket.send`
Sends a string over a connected socket.

```ark
sys.net.socket.send(sock, "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
```

### `sys.net.socket.set_timeout`
Sets the read/write timeout in seconds for a socket.

```ark
sys.net.socket.set_timeout(sock, 5)  // 5 second timeout
```

---

## Str

String operations.

### `sys.str.from_code`
Converts a Unicode code point (integer) to a single-character string.

```ark
ch := sys.str.from_code(65)  // "A"
```

### `sys.str.get`
Returns the character at the given index of a string. Zero-indexed.

```ark
ch := sys.str.get("hello", 0)  // "h"
```

---

## Struct

Struct (object/dictionary) operations.

### `sys.struct.get`
Gets a field value from a struct by key. Returns `nil` if the key doesn't exist.

```ark
person := { name: "Alice", age: 30 }
val := sys.struct.get(person, "name")  // "Alice"
```

### `sys.struct.has`
Returns `true` if the struct has the given key.

```ark
has_name := sys.struct.has(person, "name")  // true
```

### `sys.struct.set`
Sets a field on a struct. Mutates the struct in place.

```ark
sys.struct.set(person, "age", 31)
```

---

## Sys

System-level intrinsics for process control and shell execution.

### `sys.exec`
Executes a shell command and returns the output as a string. Requires `exec` capability.

```ark
output := sys.exec("ls -la")
print(output)
```

### `sys.exit`
Terminates the program with the given exit code (default 0).

```ark
sys.exit(1)  // exit with error
```

### `sys.len`
Returns the length of a string, list, or buffer. Same as `len()`.

```ark
sys.len([1, 2, 3])  // 3
```

### `sys.log`
Writes a debug log message to stderr. Useful for diagnostics without polluting stdout.

```ark
sys.log("Processing item", i)
```

---

## Time

Wall-clock time operations.

### `sys.time.now`
Returns the current Unix timestamp in seconds since the epoch (1970-01-01 00:00:00 UTC).

```ark
ts := sys.time.now()
print("Current time:", ts)
```

### `sys.time.sleep`
Blocks execution for the given number of seconds.

```ark
sys.time.sleep(2)  // sleep 2 seconds
```

---

## Z3

Formal verification via Z3 SMT solver integration.

### `sys.z3.verify`
Verifies a set of SMT-LIB2 constraints using the Z3 solver. Takes a list of constraint strings. Returns `true` if satisfiable (consistent), `false` if unsatisfiable (contradiction).

```ark
// Consistent constraints → true
c1 := ["(declare-const x Int)", "(assert (> x 10))"]
result := sys.z3.verify(c1)  // true

// Contradictory constraints → false
c2 := ["(declare-const x Int)", "(assert (> x 10))", "(assert (< x 5))"]
result := sys.z3.verify(c2)  // false
```
