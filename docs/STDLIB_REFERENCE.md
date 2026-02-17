# Ark Standard Library Reference

The standard library (`lib/std/`) provides higher-level wrappers around system intrinsics. Import modules using `import <module>` in your Ark programs.

## Table of Contents
- [Ai](#ai)
- [Audio](#audio)
- [Chain](#chain)
- [Event](#event)
- [Fs](#fs)
- [Io](#io)
- [Math](#math)
- [Net](#net)
- [Result](#result)
- [String](#string)
- [Crypto](#crypto)
- [Time](#time)

---

## Ai

Neural bridge for AI model interaction. Requires a configured AI provider backend.

### `ask(prompt)`
Sends a prompt to the connected AI model and returns the response as a string.

```ark
answer := ai.ask("What is the capital of Canada?")
print(answer)  // "Ottawa"
```

### `chat(message)`
Sends a message in an ongoing conversational context (maintains history).

```ark
ai.chat("Remember my name is Alice")
response := ai.chat("What's my name?")  // "Alice"
```

### `new(persona)`
Creates a new AI agent with a custom persona/system prompt.

```ark
coder := ai.new("You are a Rust expert. Be concise.")
coder.ask("How do I read a file?")
```

---

## Audio

Audio synthesis, WAV file I/O, and MP3 metadata reading. All operations work on raw PCM sample buffers.

### `_synth_sine(freq, duration_ms, sample_rate, volume)`
Generates a sine wave tone. Returns a PCM sample buffer.

```ark
buf := audio._synth_sine(440, 1000, 44100, 0.8)  // A4 for 1 second
```

### `_synth_sawtooth(freq, duration_ms, sample_rate, volume)`
Generates a sawtooth wave tone. Returns a PCM sample buffer.

```ark
buf := audio._synth_sawtooth(220, 500, 44100, 0.5)
```

### `_synth_square(freq, duration_ms, sample_rate, volume)`
Generates a square wave tone. Returns a PCM sample buffer.

```ark
buf := audio._synth_square(330, 1000, 44100, 0.6)
```

### `_wav_read(path)`
Reads a WAV file and returns a struct with `data` (sample buffer), `sample_rate`, and `channels`.

```ark
wav := audio._wav_read("input.wav")
print("Sample rate:", wav.sample_rate)
```

### `_wav_write(path, buffer, sample_rate, channels)`
Writes PCM samples to a WAV file.

```ark
audio._wav_write("output.wav", buf, 44100, 1)
```

### `_mp3_read_metadata(path)`
Reads ID3 metadata from an MP3 file. Returns a struct with `title`, `artist`, `album`, etc.

```ark
meta := audio._mp3_read_metadata("song.mp3")
print(meta.title, "-", meta.artist)
```

### Buffer Utilities
- `_buf_to_str(buf, start, len)` — Extracts a string slice from a byte buffer.
- `_read_u16_le(buf, idx)` — Reads a little-endian 16-bit unsigned integer from a buffer.
- `_read_u32_le(buf, idx)` — Reads a little-endian 32-bit unsigned integer from a buffer.
- `_write_u16_le(buf, idx, val)` — Writes a little-endian 16-bit unsigned integer to a buffer.
- `_write_u32_le(buf, idx, val)` — Writes a little-endian 32-bit unsigned integer to a buffer.

---

## Chain

Blockchain client module. Wraps chain intrinsics and provides JSON-RPC functions for Ethereum testnet connectivity (Sepolia default).

### `chain.height()`
Returns the current block height from the built-in chain state.

```ark
h := chain.height()
print("Block:", h)
```

### `chain.get_balance(addr)`
Returns the balance for an address (in wei for Ethereum).

```ark
bal := chain.get_balance("0xabc...")
```

### `chain.submit_tx(payload)`
Submits a signed raw transaction. Returns the transaction hash.

```ark
hash := chain.submit_tx(signed_tx)
```

### `chain.verify_tx(tx_hash)`
Checks confirmation status of a transaction. Returns `true`/`false`.

```ark
if chain.verify_tx(hash) { print("Confirmed!") }
```

### `chain.get_block(n)`
Fetches full block data by number via `eth_getBlockByNumber` JSON-RPC.

```ark
block := chain.get_block(12345)
```

### `chain.get_tx(hash)`
Fetches transaction details via `eth_getTransactionByHash` JSON-RPC.

### `chain.get_tx_receipt(hash)`
Fetches the transaction receipt (gas used, status, logs).

### `chain.estimate_gas(tx)`
Estimates gas for a transaction via `eth_estimateGas`.

### `chain.gas_price()`
Returns current gas price via `eth_gasPrice`.

### `chain.rpc_block_number()`
Returns latest block number from the JSON-RPC endpoint (hex string).

---

## Event

Non-blocking event loop for asynchronous I/O and timers.

### `loop()`
Starts the event loop. Blocks until all registered handlers complete.

```ark
event.on("data", func(d) { print("Got:", d) })
event.loop()
```

### `poll()`
Polls for pending events without blocking. Returns immediately.

```ark
event.poll()
```

### `sleep(s)`
Async-compatible sleep. Yields control to the event loop for `s` seconds.

```ark
event.sleep(1)  // non-blocking pause
```

---

## Fs

File system operations. Requires `fs_read` and/or `fs_write` capabilities. All paths are sandboxed.

### `fs.read(path)`
Reads a file as a UTF-8 string. Requires `fs_read` capability.

```ark
content := fs.read("config.json")
```

### `fs.write(path, content)`
Writes a string to a file. Creates if not exists, overwrites if exists. Requires `fs_write`.

```ark
fs.write("output.txt", "Hello")
```

### `fs.append(path, content)`
Appends content to the end of a file. Requires `fs_read` + `fs_write`.

```ark
fs.append("log.txt", "Entry logged\n")
```

### `fs.read_bytes(path)`
Reads a file as raw bytes (list of 0–255). Requires `fs_read`.

```ark
bytes := fs.read_bytes("image.png")
```

### `fs.write_bytes(path, bytes)`
Writes raw bytes to a file. Requires `fs_write`.

### `fs.exists(path)`
Returns `true` if the file exists and is readable.

```ark
if fs.exists("config.json") { print("Found config") }
```

### `fs.size(path)`
Returns the size of a file in bytes.

---

## Io

Standard I/O and async wrappers for console interaction and file-based I/O.

### `print(msg)`
Prints a value to stdout with a newline.

```ark
io.print("Hello, Ark!")
```

### `println(msg)`
Alias for `print`. Prints with trailing newline.

### `read_file(path)`
Synchronous file read. Returns contents as a string.

```ark
data := io.read_file("data.txt")
```

### `write_file(path, content)`
Synchronous file write.

### `read_file_async(path, cb)`
Asynchronous file read. Calls `cb(content)` when complete.

```ark
io.read_file_async("big_file.txt", func(data) {
    print("Read", len(data), "chars")
})
```

### `net_request_async(url, cb)`
Asynchronous HTTP GET. Calls `cb(response_body)` when complete.

```ark
io.net_request_async("https://api.example.com", func(body) {
    print(body)
})
```

---

## Math

Scalar math, fixed-point trigonometry, and tensor (matrix) operations.

### `Tensor(data, shape)`
Creates a tensor from a flat data list and a shape list.

```ark
m := math.Tensor([1, 2, 3, 4], [2, 2])  // 2x2 matrix
```

### `add(a, b)`
Element-wise tensor addition. Both tensors must share the same shape.

```ark
c := math.add(a, b)
```

### `sub(a, b)`
Element-wise tensor subtraction.

### `mul_scalar(t, s)`
Multiplies every element in a tensor by scalar `s`.

```ark
scaled := math.mul_scalar(t, 10)
```

### `dot(a, b)`
Dot product of two 1D vectors.

```ark
result := math.dot([1, 2, 3], [4, 5, 6])  // 32
```

### `matmul(a, b)`
Matrix multiplication. A=[m,k] × B=[k,n] → C=[m,n].

```ark
C := math.matmul(A, B)
```

### `transpose(t)`
Matrix transpose. T=[m,n] → T'=[n,m].

---

## Net

Networking: HTTP client, TCP sockets, P2P connectivity, and Noise protocol encryption.

### `http_get(url)`
Sends an HTTP GET request. Returns the response body as a string.

```ark
body := net.http_get("https://api.example.com/data")
```

### `http_post(url, body)`
Sends an HTTP POST request with a body. Returns the response.

```ark
result := net.http_post("https://api.example.com", payload)
```

### `net_connect(ip, port)`
Opens a TCP connection to a remote host. Returns a socket handle.

```ark
sock := net.net_connect("127.0.0.1", 8080)
```

### `net_listen(port, handler)`
Starts a TCP listener. Calls `handler(connection)` for each incoming connection.

```ark
net.net_listen(8080, func(conn) {
    data := net.secure_recv(conn, 1024)
    net.secure_send(conn, "ACK")
})
```

### `net_broadcast(msg)`
Broadcasts a message to all connected peers in the P2P network.

### `noise_handshake(handle)`
Performs a Noise protocol handshake on a TCP connection. Establishes an encrypted channel.

```ark
secure_handle := net.noise_handshake(sock)
```

### `secure_send(handle, data)`
Sends encrypted data over a Noise-secured connection.

### `secure_recv(handle, size)`
Receives and decrypts data from a Noise-secured connection.

---

## Result

Rust-inspired `Result` type for explicit error handling without exceptions.

### `Result.Ok(val)`
Wraps a success value in a Result.

```ark
r := Result.Ok(42)
```

### `Result.Err(e)`
Wraps an error value in a Result.

```ark
r := Result.Err("file not found")
```

### `Result.is_ok(res)`
Returns `true` if the Result contains a success value.

```ark
if Result.is_ok(r) { print("Success!") }
```

### `Result.is_err(res)`
Returns `true` if the Result contains an error.

### `Result.unwrap(res)`
Extracts the success value. Panics if the Result is an error.

```ark
val := Result.unwrap(r)  // 42
```

### `Result.unwrap_err(res)`
Extracts the error value. Panics if the Result is Ok.

### `Result.unwrap_or(res, default)`
Extracts the success value, or returns `default` if the Result is an error.

```ark
val := Result.unwrap_or(r, 0)  // 42 if Ok, 0 if Err
```

### `Result.map(res, f)`
Applies function `f` to the success value if Ok, passes through Err unchanged.

```ark
doubled := Result.map(r, func(x) { x * 2 })
```

### `Result.map_err(res, f)`
Applies function `f` to the error value if Err, passes through Ok unchanged.

---

## String

String manipulation functions. Strings are immutable UTF-8 sequences.

### `string_len(s)`
Returns the length of a string in characters.

```ark
string_len("hello")  // 5
```

### `string_get(s, i)`
Returns the character at index `i`. Zero-indexed.

```ark
string_get("hello", 0)  // "h"
```

### `string_slice(s, start, end)`
Returns a substring from `start` (inclusive) to `end` (exclusive).

```ark
string_slice("hello world", 0, 5)  // "hello"
```

### `string_find(s, sub, start_index)`
Finds the first occurrence of `sub` in `s` starting at `start_index`. Returns the index, or -1 if not found.

```ark
string_find("hello world", "world", 0)  // 6
```

### `string_contains(s, sub)`
Returns `true` if `s` contains `sub`.

```ark
string_contains("hello world", "world")  // true
```

### `string_starts_with(s, prefix)`
Returns `true` if `s` starts with `prefix`.

### `string_ends_with(s, suffix)`
Returns `true` if `s` ends with `suffix`.

### `string_split(s, delim)`
Splits a string by delimiter. Returns a list of strings.

```ark
parts := string_split("a,b,c", ",")  // ["a", "b", "c"]
```

### `string_join(lst, sep)`
Joins a list of strings with a separator.

```ark
string_join(["a", "b", "c"], "-")  // "a-b-c"
```

### `string_replace(s, old, new)`
Replaces all occurrences of `old` with `new` in `s`.

```ark
string_replace("hello world", "world", "ark")  // "hello ark"
```

### `string_trim(s)`
Removes leading and trailing whitespace.

```ark
string_trim("  hello  ")  // "hello"
```

### `string_concat(a, b)`
Concatenates two strings. Equivalent to `a + b`.

### `string_is_space(c)`
Returns `true` if the character `c` is a whitespace character.

---

## Crypto

Sovereign cryptographic primitives. See also: [Crypto Intrinsics](API_REFERENCE.md#crypto).

### `crypto.hash(data)`
SHA-256 hash. Returns hex-encoded digest.

### `crypto.sha512(data)`
SHA-512 hash. Returns hex-encoded digest.

### `crypto.hmac_sha512(key, data)`
HMAC-SHA512 message authentication code.

### `crypto.pbkdf2(password, salt, rounds)`
PBKDF2-HMAC-SHA512 key derivation.

### `crypto.aes_gcm.encrypt(plaintext, key)`
AES-256-GCM authenticated encryption.

### `crypto.aes_gcm.decrypt(ciphertext, key)`
AES-256-GCM authenticated decryption.

### `crypto.random_bytes(n)`
`n` cryptographically secure random bytes.

### `crypto.merkle_root(leaves)`
Merkle root of a list of hex-encoded leaf hashes.

### `crypto.ed25519.gen()`
Generates an Ed25519 keypair.

### `crypto.ed25519.sign(msg, key)`
Signs a message with an Ed25519 secret key.

### `crypto.ed25519.verify(sig, msg, pubkey)`
Verifies an Ed25519 signature.

---

## Time

Wall-clock time operations.

### `time.now()`
Returns the current Unix timestamp in seconds.

```ark
ts := time.now()
```

### `time.sleep(s)`
Blocks execution for `s` seconds.

### `time.elapsed(start, end)`
Returns the difference between two timestamps in seconds.

### `time.format_ms(ms)`
Formats milliseconds as `"Xs Yms"` for human-readable display.

### `time.timestamp()`
Alias for `now()`.

### `time.to_iso(ts)`
Converts a Unix timestamp to a human-readable UTC string.
