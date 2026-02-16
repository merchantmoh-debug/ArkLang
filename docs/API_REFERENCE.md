# Ark API Reference (Intrinsics)

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

## Chain
### `sys.chain.get_balance`
No description available.

```ark
// Example for sys.chain.get_balance
sys.chain.get_balance(...)
```

### `sys.chain.height`
No description available.

```ark
// Example for sys.chain.height
sys.chain.height(...)
```

### `sys.chain.submit_tx`
No description available.

```ark
// Example for sys.chain.submit_tx
sys.chain.submit_tx(...)
```

### `sys.chain.verify_tx`
No description available.

```ark
// Example for sys.chain.verify_tx
sys.chain.verify_tx(...)
```

## Core
### `exit`
No description available.

```ark
// Example for exit
exit(...)
```

### `get`
No description available.

```ark
// Example for get
get(...)
```

### `intrinsic_and`
No description available.

```ark
// Example for intrinsic_and
intrinsic_and(...)
```

### `intrinsic_ask_ai`
No description available.

```ark
// Example for intrinsic_ask_ai
intrinsic_ask_ai(...)
```

### `intrinsic_buffer_alloc`
No description available.

```ark
// Example for intrinsic_buffer_alloc
intrinsic_buffer_alloc(...)
```

### `intrinsic_buffer_inspect`
No description available.

```ark
// Example for intrinsic_buffer_inspect
intrinsic_buffer_inspect(...)
```

### `intrinsic_buffer_read`
No description available.

```ark
// Example for intrinsic_buffer_read
intrinsic_buffer_read(...)
```

### `intrinsic_buffer_write`
No description available.

```ark
// Example for intrinsic_buffer_write
intrinsic_buffer_write(...)
```

### `intrinsic_crypto_hash`
No description available.

```ark
// Example for intrinsic_crypto_hash
intrinsic_crypto_hash(...)
```

### `intrinsic_extract_code`
No description available.

```ark
// Example for intrinsic_extract_code
intrinsic_extract_code(...)
```

### `intrinsic_ge`
No description available.

```ark
// Example for intrinsic_ge
intrinsic_ge(...)
```

### `intrinsic_gt`
No description available.

```ark
// Example for intrinsic_gt
intrinsic_gt(...)
```

### `intrinsic_le`
No description available.

```ark
// Example for intrinsic_le
intrinsic_le(...)
```

### `intrinsic_len`
No description available.

```ark
// Example for intrinsic_len
intrinsic_len(...)
```

### `intrinsic_list_append`
No description available.

```ark
// Example for intrinsic_list_append
intrinsic_list_append(...)
```

### `intrinsic_list_get`
No description available.

```ark
// Example for intrinsic_list_get
intrinsic_list_get(...)
```

### `intrinsic_lt`
No description available.

```ark
// Example for intrinsic_lt
intrinsic_lt(...)
```

### `intrinsic_merkle_root`
No description available.

```ark
// Example for intrinsic_merkle_root
intrinsic_merkle_root(...)
```

### `intrinsic_not`
No description available.

```ark
// Example for intrinsic_not
intrinsic_not(...)
```

### `intrinsic_or`
No description available.

```ark
// Example for intrinsic_or
intrinsic_or(...)
```

### `intrinsic_time_now`
No description available.

```ark
// Example for intrinsic_time_now
intrinsic_time_now(...)
```

### `len`
No description available.

```ark
// Example for len
len(...)
```

### `print`
No description available.

```ark
// Example for print
print(...)
```

### `quit`
No description available.

```ark
// Example for quit
quit(...)
```

## Crypto
### `sys.crypto.aes_gcm_decrypt`
No description available.

```ark
// Example for sys.crypto.aes_gcm_decrypt
sys.crypto.aes_gcm_decrypt(...)
```

### `sys.crypto.aes_gcm_encrypt`
No description available.

```ark
// Example for sys.crypto.aes_gcm_encrypt
sys.crypto.aes_gcm_encrypt(...)
```

### `sys.crypto.ed25519.gen`
No description available.

```ark
// Example for sys.crypto.ed25519.gen
sys.crypto.ed25519.gen(...)
```

### `sys.crypto.ed25519.sign`
No description available.

```ark
// Example for sys.crypto.ed25519.sign
sys.crypto.ed25519.sign(...)
```

### `sys.crypto.ed25519.verify`
No description available.

```ark
// Example for sys.crypto.ed25519.verify
sys.crypto.ed25519.verify(...)
```

### `sys.crypto.hash`
No description available.

```ark
// Example for sys.crypto.hash
sys.crypto.hash(...)
```

### `sys.crypto.hmac_sha512`
No description available.

```ark
// Example for sys.crypto.hmac_sha512
sys.crypto.hmac_sha512(...)
```

### `sys.crypto.merkle_root`
No description available.

```ark
// Example for sys.crypto.merkle_root
sys.crypto.merkle_root(...)
```

### `sys.crypto.pbkdf2_hmac_sha512`
No description available.

```ark
// Example for sys.crypto.pbkdf2_hmac_sha512
sys.crypto.pbkdf2_hmac_sha512(...)
```

### `sys.crypto.random_bytes`
No description available.

```ark
// Example for sys.crypto.random_bytes
sys.crypto.random_bytes(...)
```

### `sys.crypto.sha512`
No description available.

```ark
// Example for sys.crypto.sha512
sys.crypto.sha512(...)
```

## Fs
### `sys.fs.read`
No description available.

```ark
// Example for sys.fs.read
sys.fs.read(...)
```

### `sys.fs.read_buffer`
No description available.

```ark
// Example for sys.fs.read_buffer
sys.fs.read_buffer(...)
```

### `sys.fs.write`
No description available.

```ark
// Example for sys.fs.write
sys.fs.write(...)
```

### `sys.fs.write_buffer`
No description available.

```ark
// Example for sys.fs.write_buffer
sys.fs.write_buffer(...)
```

## Io
### `sys.io.read_bytes`
No description available.

```ark
// Example for sys.io.read_bytes
sys.io.read_bytes(...)
```

### `sys.io.read_line`
No description available.

```ark
// Example for sys.io.read_line
sys.io.read_line(...)
```

### `sys.io.write`
No description available.

```ark
// Example for sys.io.write
sys.io.write(...)
```

## Json
### `sys.json.parse`
No description available.

```ark
// Example for sys.json.parse
sys.json.parse(...)
```

### `sys.json.stringify`
No description available.

```ark
// Example for sys.json.stringify
sys.json.stringify(...)
```

## List
### `sys.list.append`
No description available.

```ark
// Example for sys.list.append
sys.list.append(...)
```

### `sys.list.delete`
No description available.

```ark
// Example for sys.list.delete
sys.list.delete(...)
```

### `sys.list.get`
No description available.

```ark
// Example for sys.list.get
sys.list.get(...)
```

### `sys.list.pop`
No description available.

```ark
// Example for sys.list.pop
sys.list.pop(...)
```

### `sys.list.set`
No description available.

```ark
// Example for sys.list.set
sys.list.set(...)
```

## Math
### `intrinsic_math_acos`
No description available.

```ark
// Example for intrinsic_math_acos
intrinsic_math_acos(...)
```

### `intrinsic_math_asin`
No description available.

```ark
// Example for intrinsic_math_asin
intrinsic_math_asin(...)
```

### `intrinsic_math_atan`
No description available.

```ark
// Example for intrinsic_math_atan
intrinsic_math_atan(...)
```

### `intrinsic_math_atan2`
No description available.

```ark
// Example for intrinsic_math_atan2
intrinsic_math_atan2(...)
```

### `intrinsic_math_cos`
No description available.

```ark
// Example for intrinsic_math_cos
intrinsic_math_cos(...)
```

### `intrinsic_math_pow`
No description available.

```ark
// Example for intrinsic_math_pow
intrinsic_math_pow(...)
```

### `intrinsic_math_sin`
No description available.

```ark
// Example for intrinsic_math_sin
intrinsic_math_sin(...)
```

### `intrinsic_math_sqrt`
No description available.

```ark
// Example for intrinsic_math_sqrt
intrinsic_math_sqrt(...)
```

### `intrinsic_math_tan`
No description available.

```ark
// Example for intrinsic_math_tan
intrinsic_math_tan(...)
```

### `math.Tensor`
math.Tensor(data: List, shape: List) → Tensor struct

```ark
// Example for math.Tensor
math.Tensor(...)
```

### `math.acos`
No description available.

```ark
// Example for math.acos
math.acos(...)
```

### `math.add`
math.add(a, b) → Tensor.  Element-wise addition.

```ark
// Example for math.add
math.add(...)
```

### `math.asin`
No description available.

```ark
// Example for math.asin
math.asin(...)
```

### `math.atan`
No description available.

```ark
// Example for math.atan
math.atan(...)
```

### `math.atan2`
No description available.

```ark
// Example for math.atan2
math.atan2(...)
```

### `math.cos`
No description available.

```ark
// Example for math.cos
math.cos(...)
```

### `math.cos_scaled`
No description available.

```ark
// Example for math.cos_scaled
math.cos_scaled(...)
```

### `math.dot`
math.dot(a, b) → Integer.  Element-wise multiply and sum (1D vectors).

```ark
// Example for math.dot
math.dot(...)
```

### `math.matmul`
math.matmul(A, B) → Tensor.  A=[m,k], B=[k,n] → C=[m,n]

```ark
// Example for math.matmul
math.matmul(...)
```

### `math.mul_scalar`
math.mul_scalar(t, scalar) → Tensor.  Multiply every element by scalar.

```ark
// Example for math.mul_scalar
math.mul_scalar(...)
```

### `math.pi_scaled`
No description available.

```ark
// Example for math.pi_scaled
math.pi_scaled(...)
```

### `math.pow`
No description available.

```ark
// Example for math.pow
math.pow(...)
```

### `math.sin`
No description available.

```ark
// Example for math.sin
math.sin(...)
```

### `math.sin_scaled`
No description available.

```ark
// Example for math.sin_scaled
math.sin_scaled(...)
```

### `math.sqrt`
No description available.

```ark
// Example for math.sqrt
math.sqrt(...)
```

### `math.sub`
math.sub(a, b) → Tensor.  Element-wise subtraction.

```ark
// Example for math.sub
math.sub(...)
```

### `math.tan`
No description available.

```ark
// Example for math.tan
math.tan(...)
```

### `math.transpose`
math.transpose(T) → Tensor.  T=[m,n] → T'=[n,m]

```ark
// Example for math.transpose
math.transpose(...)
```

### `sys.math.pow_mod`
No description available.

```ark
// Example for sys.math.pow_mod
sys.math.pow_mod(...)
```

## Mem
### `sys.mem.alloc`
No description available.

```ark
// Example for sys.mem.alloc
sys.mem.alloc(...)
```

### `sys.mem.inspect`
No description available.

```ark
// Example for sys.mem.inspect
sys.mem.inspect(...)
```

### `sys.mem.read`
No description available.

```ark
// Example for sys.mem.read
sys.mem.read(...)
```

### `sys.mem.write`
No description available.

```ark
// Example for sys.mem.write
sys.mem.write(...)
```

## Net
### `sys.net.http.request`
No description available.

```ark
// Example for sys.net.http.request
sys.net.http.request(...)
```

### `sys.net.socket.accept`
No description available.

```ark
// Example for sys.net.socket.accept
sys.net.socket.accept(...)
```

### `sys.net.socket.bind`
No description available.

```ark
// Example for sys.net.socket.bind
sys.net.socket.bind(...)
```

### `sys.net.socket.close`
No description available.

```ark
// Example for sys.net.socket.close
sys.net.socket.close(...)
```

### `sys.net.socket.connect`
No description available.

```ark
// Example for sys.net.socket.connect
sys.net.socket.connect(...)
```

### `sys.net.socket.recv`
No description available.

```ark
// Example for sys.net.socket.recv
sys.net.socket.recv(...)
```

### `sys.net.socket.send`
No description available.

```ark
// Example for sys.net.socket.send
sys.net.socket.send(...)
```

### `sys.net.socket.set_timeout`
No description available.

```ark
// Example for sys.net.socket.set_timeout
sys.net.socket.set_timeout(...)
```

## Str
### `sys.str.from_code`
No description available.

```ark
// Example for sys.str.from_code
sys.str.from_code(...)
```

### `sys.str.get`
No description available.

```ark
// Example for sys.str.get
sys.str.get(...)
```

## Struct
### `sys.struct.get`
No description available.

```ark
// Example for sys.struct.get
sys.struct.get(...)
```

### `sys.struct.has`
No description available.

```ark
// Example for sys.struct.has
sys.struct.has(...)
```

### `sys.struct.set`
No description available.

```ark
// Example for sys.struct.set
sys.struct.set(...)
```

## Sys
### `sys.exec`
No description available.

```ark
// Example for sys.exec
sys.exec(...)
```

### `sys.exit`
No description available.

```ark
// Example for sys.exit
sys.exit(...)
```

### `sys.len`
No description available.

```ark
// Example for sys.len
sys.len(...)
```

### `sys.log`
No description available.

```ark
// Example for sys.log
sys.log(...)
```

## Time
### `sys.time.now`
No description available.

```ark
// Example for sys.time.now
sys.time.now(...)
```

### `sys.time.sleep`
No description available.

```ark
// Example for sys.time.sleep
sys.time.sleep(...)
```
