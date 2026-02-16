# Ark Standard Library Reference

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

## Ai
### `ask(prompt)`
Ark Standard Library: AI (The Neural Bridge)
(c) 2026 Sovereign Systems

```ark
// Example for ask
ask(...)
```

### `chat(message)`
No description available.

```ark
// Example for chat
chat(...)
```

### `new(persona)`
No description available.

```ark
// Example for new
new(...)
```

## Audio
### `_buf_to_str(buf, start, len)`
No description available.

```ark
// Example for _buf_to_str
_buf_to_str(..., ..., ...)
```

### `_mp3_read_metadata(path)`
--- Mp3 ---

```ark
// Example for _mp3_read_metadata
_mp3_read_metadata(...)
```

### `_read_u16_le(buf, idx)`
No description available.

```ark
// Example for _read_u16_le
_read_u16_le(..., ...)
```

### `_read_u32_le(buf, idx)`
No description available.

```ark
// Example for _read_u32_le
_read_u32_le(..., ...)
```

### `_synth_sawtooth(freq, duration_ms, sample_rate, volume)`
No description available.

```ark
// Example for _synth_sawtooth
_synth_sawtooth(..., ..., ..., ...)
```

### `_synth_sine(freq, duration_ms, sample_rate, volume)`
--- Synth ---

```ark
// Example for _synth_sine
_synth_sine(..., ..., ..., ...)
```

### `_synth_square(freq, duration_ms, sample_rate, volume)`
No description available.

```ark
// Example for _synth_square
_synth_square(..., ..., ..., ...)
```

### `_wav_read(path)`
No description available.

```ark
// Example for _wav_read
_wav_read(...)
```

### `_wav_write(path, buffer, sample_rate, channels)`
--- Wav ---

```ark
// Example for _wav_write
_wav_write(..., ..., ..., ...)
```

### `_write_u16_le(buf, idx, val)`
lib/std/audio.ark

```ark
// Example for _write_u16_le
_write_u16_le(..., ..., ...)
```

### `_write_u32_le(buf, idx, val)`
No description available.

```ark
// Example for _write_u32_le
_write_u32_le(..., ..., ...)
```

## Chain
### `chain_get_balance(addr)`
No description available.

```ark
// Example for chain_get_balance
chain_get_balance(...)
```

### `chain_height()`
Standard Library: Chain

```ark
// Example for chain_height
chain_height()
```

### `chain_submit_tx(payload)`
No description available.

```ark
// Example for chain_submit_tx
chain_submit_tx(...)
```

### `chain_verify_tx(tx_id)`
No description available.

```ark
// Example for chain_verify_tx
chain_verify_tx(...)
```

## Event
### `_event_loop()`
No description available.

```ark
// Example for _event_loop
_event_loop()
```

### `_event_poll()`
No description available.

```ark
// Example for _event_poll
_event_poll()
```

### `_event_sleep(s)`
No description available.

```ark
// Example for _event_sleep
_event_sleep(...)
```

### `loop()`
No description available.

```ark
// Example for loop
loop()
```

### `poll()`
No description available.

```ark
// Example for poll
poll()
```

### `sleep(s)`
No description available.

```ark
// Example for sleep
sleep(...)
```

## Fs
### `append_file(path, content)`
No description available.

```ark
// Example for append_file
append_file(..., ...)
```

### `read_file(path)`
Ark Standard Library: Filesystem
(c) 2026 Sovereign Systems

```ark
// Example for read_file
read_file(...)
```

### `write_file(path, content)`
No description available.

```ark
// Example for write_file
write_file(..., ...)
```

## Io
### `net_request_async(url, cb)`
No description available.

```ark
// Example for net_request_async
net_request_async(..., ...)
```

### `print(msg)`
No description available.

```ark
// Example for print
print(...)
```

### `println(msg)`
No description available.

```ark
// Example for println
println(...)
```

### `read_file(path)`
Sync Operations

```ark
// Example for read_file
read_file(...)
```

### `read_file_async(path, cb)`
Async Operations

```ark
// Example for read_file_async
read_file_async(..., ...)
```

### `write_file(path, content)`
No description available.

```ark
// Example for write_file
write_file(..., ...)
```

## Math
### `Tensor(data, shape)`
Standard Library: Math
Fixed-Point Trigonometry (Scale: 10000)

```ark
// Example for Tensor
Tensor(..., ...)
```

### `add(a, b)`
No description available.

```ark
// Example for add
add(..., ...)
```

### `dot(a, b)`
No description available.

```ark
// Example for dot
dot(..., ...)
```

### `matmul(a, b)`
No description available.

```ark
// Example for matmul
matmul(..., ...)
```

### `mul_scalar(t, s)`
No description available.

```ark
// Example for mul_scalar
mul_scalar(..., ...)
```

### `sub(a, b)`
No description available.

```ark
// Example for sub
sub(..., ...)
```

### `transpose(t)`
No description available.

```ark
// Example for transpose
transpose(...)
```

## Net
### `http_get(url)`
--- HTTP Helpers ---

```ark
// Example for http_get
http_get(...)
```

### `http_post(url, body)`
No description available.

```ark
// Example for http_post
http_post(..., ...)
```

### `net_broadcast(msg)`
No description available.

```ark
// Example for net_broadcast
net_broadcast(...)
```

### `net_connect(ip, port)`
No description available.

```ark
// Example for net_connect
net_connect(..., ...)
```

### `net_listen(port, handler)`
No description available.

```ark
// Example for net_listen
net_listen(..., ...)
```

### `noise_handshake(handle)`
No description available.

```ark
// Example for noise_handshake
noise_handshake(...)
```

### `secure_recv(handle, size)`
No description available.

```ark
// Example for secure_recv
secure_recv(..., ...)
```

### `secure_send(handle, data)`
No description available.

```ark
// Example for secure_send
secure_send(..., ...)
```

## Result
### `_Result_Err(e)`
No description available.

```ark
// Example for _Result_Err
_Result_Err(...)
```

### `_Result_Ok(val)`
No description available.

```ark
// Example for _Result_Ok
_Result_Ok(...)
```

### `_Result_is_err(res)`
No description available.

```ark
// Example for _Result_is_err
_Result_is_err(...)
```

### `_Result_is_ok(res)`
No description available.

```ark
// Example for _Result_is_ok
_Result_is_ok(...)
```

### `_Result_map(res, f)`
No description available.

```ark
// Example for _Result_map
_Result_map(..., ...)
```

### `_Result_map_err(res, f)`
No description available.

```ark
// Example for _Result_map_err
_Result_map_err(..., ...)
```

### `_Result_unwrap(res)`
No description available.

```ark
// Example for _Result_unwrap
_Result_unwrap(...)
```

### `_Result_unwrap_err(res)`
No description available.

```ark
// Example for _Result_unwrap_err
_Result_unwrap_err(...)
```

### `_Result_unwrap_or(res, default)`
No description available.

```ark
// Example for _Result_unwrap_or
_Result_unwrap_or(..., ...)
```

## String
### `_string_trim_find_end(s)`
No description available.

```ark
// Example for _string_trim_find_end
_string_trim_find_end(...)
```

### `_string_trim_find_start(s)`
No description available.

```ark
// Example for _string_trim_find_start
_string_trim_find_start(...)
```

### `string_concat(a, b)`
No description available.

```ark
// Example for string_concat
string_concat(..., ...)
```

### `string_contains(s, sub)`
No description available.

```ark
// Example for string_contains
string_contains(..., ...)
```

### `string_ends_with(s, suffix)`
No description available.

```ark
// Example for string_ends_with
string_ends_with(..., ...)
```

### `string_find(s, sub, start_index)`
No description available.

```ark
// Example for string_find
string_find(..., ..., ...)
```

### `string_get(s, i)`
No description available.

```ark
// Example for string_get
string_get(..., ...)
```

### `string_is_space(c)`
No description available.

```ark
// Example for string_is_space
string_is_space(...)
```

### `string_join(lst, sep)`
No description available.

```ark
// Example for string_join
string_join(..., ...)
```

### `string_len(s)`
No description available.

```ark
// Example for string_len
string_len(...)
```

### `string_replace(s, old, new)`
No description available.

```ark
// Example for string_replace
string_replace(..., ..., ...)
```

### `string_slice(s, start, end)`
No description available.

```ark
// Example for string_slice
string_slice(..., ..., ...)
```

### `string_split(s, delim)`
No description available.

```ark
// Example for string_split
string_split(..., ...)
```

### `string_starts_with(s, prefix)`
No description available.

```ark
// Example for string_starts_with
string_starts_with(..., ...)
```

### `string_trim(s)`
No description available.

```ark
// Example for string_trim
string_trim(...)
```
