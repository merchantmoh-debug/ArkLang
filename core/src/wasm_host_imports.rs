/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * Ark Host Import Bridge — Tier 3 Intrinsics for WASM.
 *
 * This module provides host-side implementations of Ark intrinsics that
 * cannot be satisfied by WASI alone. Host functions are registered under
 * the `ark_host` import module and called via the standard WASM import
 * mechanism.
 *
 * Architecture:
 *   - Math functions use f64 reinterpret_cast (i64 ↔ f64 bit patterns)
 *   - Crypto/JSON functions use linear memory for byte array I/O
 *   - All functions return i64 to maintain Ark's uniform value model
 */

use wasmtime::{Caller, Extern, Linker};

/// Host state re-exported from wasm_runner.
/// We use the same HostState so linker registrations are compatible.
use crate::wasm_runner::HostState;

// =============================================================================
// Constants — Host Import Module
// =============================================================================

/// The import module name for all Ark host functions.
pub const ARK_HOST_MODULE: &str = "ark_host";

/// Number of host imports provided by this module.
pub const ARK_HOST_IMPORT_COUNT: u32 = 14;

// Host function indices (offset from WASI imports in the codegen function table)
// These are the order in which functions appear in the import section AFTER the
// 11 WASI imports.  So host function 0 is at global import index 11, etc.
//
// Math (i64 → reinterpreted f64 → i64):
//   0: math_sin(x:i64) -> i64
//   1: math_cos(x:i64) -> i64
//   2: math_tan(x:i64) -> i64
//   3: math_asin(x:i64) -> i64
//   4: math_acos(x:i64) -> i64
//   5: math_atan(x:i64) -> i64
//   6: math_atan2(y:i64, x:i64) -> i64
//   7: math_sqrt(x:i64) -> i64
//   8: math_pow(base:i64, exp:i64) -> i64
//   9: math_pow_mod(base:i64, exp:i64, modulus:i64) -> i64
//
// Crypto (linear memory):
//  10: crypto_sha512(data_ptr:i32, data_len:i32, out_ptr:i32) -> i32 (errno)
//
// JSON (linear memory):
//  11: json_parse(str_ptr:i32, str_len:i32, out_ptr:i32) -> i32 (bytes written)
//  12: json_stringify(val_ptr:i32, val_len:i32, out_ptr:i32) -> i32 (bytes written)
//
// AI:
//  13: ask_ai(prompt_ptr:i32, prompt_len:i32, out_ptr:i32, out_cap:i32) -> i32 (bytes written)

/// Error type alias
type LinkResult = Result<(), crate::wasm_runner::WasmRunError>;

fn link_err(msg: String) -> crate::wasm_runner::WasmRunError {
    crate::wasm_runner::WasmRunError {
        message: msg,
        context: "link_ark_host".to_string(),
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Register all Ark host imports into the wasmtime Linker.
///
/// Must be called AFTER `link_wasi_fd_write` and `link_wasi_stubs` so that
/// all WASI imports are already in the linker.
pub fn link_ark_host_imports(linker: &mut Linker<HostState>) -> LinkResult {
    link_math_imports(linker)?;
    link_crypto_imports(linker)?;
    link_json_imports(linker)?;
    link_ai_imports(linker)?;
    Ok(())
}

// =============================================================================
// Math Imports — f64 reinterpret via i64 bit patterns
// =============================================================================
//
// Ark integers are i64.  For trig/sqrt/pow we transmute the i64 to f64,
// apply the operation, then transmute back.  This preserves full IEEE 754
// precision without needing f64 WASM types in the Ark value model.

fn link_math_imports(linker: &mut Linker<HostState>) -> LinkResult {
    // Helper macro to avoid boilerplate for unary f64 ops
    macro_rules! link_unary_f64 {
        ($linker:expr, $name:expr, $op:expr) => {
            $linker
                .func_wrap(
                    ARK_HOST_MODULE,
                    $name,
                    |_caller: Caller<'_, HostState>, x: i64| -> i64 {
                        let xf = f64::from_bits(x as u64);
                        let result: f64 = $op(xf);
                        result.to_bits() as i64
                    },
                )
                .map_err(|e| link_err(format!("Failed to link {}: {}", $name, e)))?;
        };
    }

    link_unary_f64!(linker, "math_sin", f64::sin);
    link_unary_f64!(linker, "math_cos", f64::cos);
    link_unary_f64!(linker, "math_tan", f64::tan);
    link_unary_f64!(linker, "math_asin", f64::asin);
    link_unary_f64!(linker, "math_acos", f64::acos);
    link_unary_f64!(linker, "math_atan", f64::atan);
    link_unary_f64!(linker, "math_sqrt", f64::sqrt);

    // atan2(y, x) → binary
    linker
        .func_wrap(
            ARK_HOST_MODULE,
            "math_atan2",
            |_caller: Caller<'_, HostState>, y: i64, x: i64| -> i64 {
                let yf = f64::from_bits(y as u64);
                let xf = f64::from_bits(x as u64);
                let result = yf.atan2(xf);
                result.to_bits() as i64
            },
        )
        .map_err(|e| link_err(format!("Failed to link math_atan2: {}", e)))?;

    // pow(base, exp) → binary f64
    linker
        .func_wrap(
            ARK_HOST_MODULE,
            "math_pow",
            |_caller: Caller<'_, HostState>, base: i64, exp: i64| -> i64 {
                let bf = f64::from_bits(base as u64);
                let ef = f64::from_bits(exp as u64);
                let result = bf.powf(ef);
                result.to_bits() as i64
            },
        )
        .map_err(|e| link_err(format!("Failed to link math_pow: {}", e)))?;

    // pow_mod(base, exp, modulus) → integer modular exponentiation
    // This works on raw i64 values (NOT f64 reinterpret)
    linker
        .func_wrap(
            ARK_HOST_MODULE,
            "math_pow_mod",
            |_caller: Caller<'_, HostState>, base: i64, exp: i64, modulus: i64| -> i64 {
                if modulus <= 0 {
                    return 0; // error: modulus must be positive
                }
                let m = modulus as u64;
                let mut result: u64 = 1;
                let mut b = (base % modulus) as u64;
                let mut e = exp as u64;
                while e > 0 {
                    if e & 1 == 1 {
                        result = result.wrapping_mul(b) % m;
                    }
                    e >>= 1;
                    b = b.wrapping_mul(b) % m;
                }
                result as i64
            },
        )
        .map_err(|e| link_err(format!("Failed to link math_pow_mod: {}", e)))?;

    Ok(())
}

// =============================================================================
// Crypto Imports — SHA-512 via linear memory
// =============================================================================

fn link_crypto_imports(linker: &mut Linker<HostState>) -> LinkResult {
    // crypto_sha512(data_ptr, data_len, out_ptr) -> errno (0 = success)
    //
    // Reads `data_len` bytes from `data_ptr` in WASM memory,
    // computes SHA-512, writes 64 bytes to `out_ptr`.
    linker
        .func_wrap(
            ARK_HOST_MODULE,
            "crypto_sha512",
            |mut caller: Caller<'_, HostState>,
             data_ptr: i32,
             data_len: i32,
             out_ptr: i32|
             -> i32 {
                use sha2::{Digest, Sha512};

                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return 1, // no memory export
                };

                let dp = data_ptr as usize;
                let dl = data_len as usize;
                let op = out_ptr as usize;

                // Read input data
                let data_bytes: Vec<u8> = {
                    let mem_data = memory.data(&caller);
                    if dp + dl > mem_data.len() || op + 64 > mem_data.len() {
                        return 2; // EFAULT
                    }
                    mem_data[dp..dp + dl].to_vec()
                };

                // Compute SHA-512
                let mut hasher = Sha512::new();
                hasher.update(&data_bytes);
                let hash = hasher.finalize();

                // Write 64 bytes of hash to output
                let mem_data = memory.data_mut(&mut caller);
                mem_data[op..op + 64].copy_from_slice(&hash);

                0 // success
            },
        )
        .map_err(|e| link_err(format!("Failed to link crypto_sha512: {}", e)))?;

    Ok(())
}

// =============================================================================
// JSON Imports — stub implementations
// =============================================================================

fn link_json_imports(linker: &mut Linker<HostState>) -> LinkResult {
    // json_parse(str_ptr, str_len, out_ptr) -> bytes_written
    // Stub: reads JSON string from memory, writes back the same bytes
    // (passthrough until full Value serialization is implemented)
    linker
        .func_wrap(
            ARK_HOST_MODULE,
            "json_parse",
            |mut caller: Caller<'_, HostState>, str_ptr: i32, str_len: i32, out_ptr: i32| -> i32 {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return -1,
                };

                let sp = str_ptr as usize;
                let sl = str_len as usize;
                let op = out_ptr as usize;

                // Read input JSON string
                let json_bytes: Vec<u8> = {
                    let data = memory.data(&caller);
                    if sp + sl > data.len() {
                        return -1;
                    }
                    data[sp..sp + sl].to_vec()
                };

                // Validate it's valid JSON
                let json_str = match std::str::from_utf8(&json_bytes) {
                    Ok(s) => s,
                    Err(_) => return -1,
                };

                // Try to parse (validation only for now)
                if serde_json::from_str::<serde_json::Value>(json_str).is_err() {
                    return -1;
                }

                // Write back the raw JSON bytes (passthrough)
                let data = memory.data_mut(&mut caller);
                if op + sl > data.len() {
                    return -1;
                }
                data[op..op + sl].copy_from_slice(&json_bytes);

                sl as i32
            },
        )
        .map_err(|e| link_err(format!("Failed to link json_parse: {}", e)))?;

    // json_stringify(val_ptr, val_len, out_ptr) -> bytes_written
    // Stub: passthrough — copies input to output
    linker
        .func_wrap(
            ARK_HOST_MODULE,
            "json_stringify",
            |mut caller: Caller<'_, HostState>, val_ptr: i32, val_len: i32, out_ptr: i32| -> i32 {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return -1,
                };

                let vp = val_ptr as usize;
                let vl = val_len as usize;
                let op = out_ptr as usize;

                let val_bytes: Vec<u8> = {
                    let data = memory.data(&caller);
                    if vp + vl > data.len() {
                        return -1;
                    }
                    data[vp..vp + vl].to_vec()
                };

                let data = memory.data_mut(&mut caller);
                if op + vl > data.len() {
                    return -1;
                }
                data[op..op + vl].copy_from_slice(&val_bytes);

                vl as i32
            },
        )
        .map_err(|e| link_err(format!("Failed to link json_stringify: {}", e)))?;

    Ok(())
}

// =============================================================================
// AI Imports — stub
// =============================================================================

fn link_ai_imports(linker: &mut Linker<HostState>) -> LinkResult {
    // ask_ai(prompt_ptr, prompt_len, out_ptr, out_cap) -> bytes_written
    // Stub: returns a static response "AI response placeholder"
    linker
        .func_wrap(
            ARK_HOST_MODULE,
            "ask_ai",
            |mut caller: Caller<'_, HostState>,
             _prompt_ptr: i32,
             _prompt_len: i32,
             out_ptr: i32,
             out_cap: i32|
             -> i32 {
                let response = b"AI response placeholder";
                let rlen = response.len().min(out_cap as usize);

                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return -1,
                };

                let data = memory.data_mut(&mut caller);
                let op = out_ptr as usize;
                if op + rlen > data.len() {
                    return -1;
                }
                data[op..op + rlen].copy_from_slice(&response[..rlen]);

                rlen as i32
            },
        )
        .map_err(|e| link_err(format!("Failed to link ask_ai: {}", e)))?;

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_math_reinterpret_roundtrip() {
        // Verify f64 ↔ i64 reinterpret works correctly
        let pi = std::f64::consts::PI;
        let bits = pi.to_bits() as i64;
        let back = f64::from_bits(bits as u64);
        assert_eq!(pi, back);
    }

    #[test]
    fn test_pow_mod_basic() {
        // 2^10 mod 1000 = 1024 mod 1000 = 24
        let base: i64 = 2;
        let exp: i64 = 10;
        let modulus: i64 = 1000;

        let m = modulus as u64;
        let mut result: u64 = 1;
        let mut b = (base % modulus) as u64;
        let mut e = exp as u64;
        while e > 0 {
            if e & 1 == 1 {
                result = result.wrapping_mul(b) % m;
            }
            e >>= 1;
            b = b.wrapping_mul(b) % m;
        }
        assert_eq!(result as i64, 24);
    }

    #[test]
    fn test_link_all_host_imports() {
        // Verify that all host functions can be linked without error
        let engine = wasmtime::Engine::default();
        let mut linker = Linker::<HostState>::new(&engine);
        let result = link_ark_host_imports(&mut linker);
        assert!(
            result.is_ok(),
            "Failed to link host imports: {:?}",
            result.err()
        );
    }
}
