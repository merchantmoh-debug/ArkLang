/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * WASM Component Interop for the Ark Language.
 *
 * Provides the ability to load, inspect, and call functions in external
 * WebAssembly modules from Ark code. Uses a lightweight approach:
 * - Parses WASM binary format to extract exports (no heavy runtime dep)
 * - Delegates execution to the wasmtime crate when the `wasm-interop` feature is enabled
 * - Falls back to a stub that returns error messages when the feature is disabled
 *
 * Intrinsics exposed to Ark:
 *   sys.wasm.load(path)           → Handle (integer ID)
 *   sys.wasm.exports(handle)      → List<String> of exported function names
 *   sys.wasm.call(handle, fn, args) → Value result
 *   sys.wasm.drop(handle)         → Boolean success
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use crate::runtime::{RuntimeError, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// =============================================================================
// WASM Module Registry
// =============================================================================

/// Represents a loaded WASM module's metadata.
#[derive(Clone)]
pub struct WasmModule {
    /// Path the module was loaded from
    pub path: String,
    /// Names of exported functions
    pub exports: Vec<String>,
    /// Raw WASM bytes (for potential future execution)
    pub bytes: Arc<Vec<u8>>,
}

lazy_static::lazy_static! {
    /// Global registry of loaded WASM modules, keyed by integer handle.
    static ref WASM_REGISTRY: Mutex<HashMap<i64, WasmModule>> = Mutex::new(HashMap::new());
    /// Next handle ID.
    static ref NEXT_HANDLE: Mutex<i64> = Mutex::new(1);
}

// =============================================================================
// WASM Binary Parser (Lightweight — no runtime dependency)
// =============================================================================

/// Parse a WASM binary to extract exported function names.
/// Implements minimal parsing of the WASM binary format (magic + version + sections).
fn parse_wasm_exports(bytes: &[u8]) -> Result<Vec<String>, String> {
    // Validate WASM magic number: \0asm
    if bytes.len() < 8 {
        return Err("Invalid WASM file: too short".to_string());
    }
    if &bytes[0..4] != b"\0asm" {
        return Err("Invalid WASM file: bad magic number".to_string());
    }
    // Version check (1)
    let _version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

    let mut exports = Vec::new();
    let mut offset = 8;

    // Walk sections to find the Export section (ID = 7)
    while offset < bytes.len() {
        if offset >= bytes.len() {
            break;
        }
        let section_id = bytes[offset];
        offset += 1;

        // Read section length (LEB128)
        let (section_len, consumed) = read_leb128_u32(&bytes[offset..])?;
        offset += consumed;
        let section_end = offset + section_len as usize;

        if section_id == 7 {
            // Export section
            let mut pos = offset;
            let (num_exports, consumed) = read_leb128_u32(&bytes[pos..])?;
            pos += consumed;

            for _ in 0..num_exports {
                // Read name length
                let (name_len, consumed) = read_leb128_u32(&bytes[pos..])?;
                pos += consumed;
                // Read name
                let name = std::str::from_utf8(&bytes[pos..pos + name_len as usize])
                    .map_err(|_| "Invalid UTF-8 in export name".to_string())?
                    .to_string();
                pos += name_len as usize;
                // Read export kind (0=func, 1=table, 2=memory, 3=global)
                let kind = bytes[pos];
                pos += 1;
                // Read export index (LEB128)
                let (_index, consumed) = read_leb128_u32(&bytes[pos..])?;
                pos += consumed;

                // Only collect function exports
                if kind == 0 {
                    exports.push(name);
                }
            }
            break;
        }

        offset = section_end;
    }

    Ok(exports)
}

/// Read an unsigned LEB128 integer. Returns (value, bytes_consumed).
fn read_leb128_u32(bytes: &[u8]) -> Result<(u32, usize), String> {
    let mut result: u32 = 0;
    let mut shift = 0;
    let mut i = 0;

    loop {
        if i >= bytes.len() {
            return Err("Unexpected end of LEB128".to_string());
        }
        let byte = bytes[i];
        result |= ((byte & 0x7F) as u32) << shift;
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            return Err("LEB128 overflow".to_string());
        }
    }

    Ok((result, i))
}

// =============================================================================
// Public API — Intrinsic Functions
// =============================================================================

/// sys.wasm.load(path) → Integer handle
///
/// Load a WASM module from disk, parse its exports, and register it.
/// Returns an integer handle for subsequent calls.
pub fn intrinsic_wasm_load(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "sys.wasm.load expects 1 argument: (path)".to_string(),
        ));
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.wasm.load: argument must be a String path".to_string(),
            ));
        }
    };

    // Read the WASM binary
    let bytes = std::fs::read(&path).map_err(|e| {
        RuntimeError::InvalidOperation(format!("sys.wasm.load: failed to read '{}': {}", path, e))
    })?;

    // Parse exports
    let exports = parse_wasm_exports(&bytes).map_err(|e| {
        RuntimeError::InvalidOperation(format!("sys.wasm.load: failed to parse '{}': {}", path, e))
    })?;

    // Register the module
    let handle = {
        let mut next = NEXT_HANDLE.lock().unwrap();
        let id = *next;
        *next += 1;
        id
    };

    let module = WasmModule {
        path,
        exports,
        bytes: Arc::new(bytes),
    };

    WASM_REGISTRY.lock().unwrap().insert(handle, module);

    Ok(Value::Integer(handle))
}

/// sys.wasm.exports(handle) → List<String>
///
/// Return the list of exported function names from a loaded WASM module.
pub fn intrinsic_wasm_exports(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "sys.wasm.exports expects 1 argument: (handle)".to_string(),
        ));
    }
    let handle = match &args[0] {
        Value::Integer(h) => *h,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.wasm.exports: argument must be an Integer handle".to_string(),
            ));
        }
    };

    let registry = WASM_REGISTRY.lock().unwrap();
    match registry.get(&handle) {
        Some(module) => {
            let names: Vec<Value> = module
                .exports
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect();
            Ok(Value::List(names))
        }
        None => Err(RuntimeError::InvalidOperation(format!(
            "sys.wasm.exports: no module with handle {}",
            handle
        ))),
    }
}

/// sys.wasm.call(handle, function_name, args_list) → Value
///
/// Call an exported function in a loaded WASM module via wasmtime.
/// Converts Ark Value arguments to i64, invokes the function, and
/// converts the result back to a Value.
pub fn intrinsic_wasm_call(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            "sys.wasm.call expects 3 arguments: (handle, function_name, args_list)".to_string(),
        ));
    }
    let handle = match &args[0] {
        Value::Integer(h) => *h,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.wasm.call: first argument must be Integer handle".to_string(),
            ));
        }
    };
    let func_name = match &args[1] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.wasm.call: second argument must be String function name".to_string(),
            ));
        }
    };
    let call_args = match &args[2] {
        Value::List(l) => l.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.wasm.call: third argument must be List of arguments".to_string(),
            ));
        }
    };

    // Convert Ark Values to i64 for WASM
    let i64_args: Vec<i64> = call_args
        .iter()
        .map(|v| match v {
            Value::Integer(n) => Ok(*n),
            Value::Boolean(b) => Ok(if *b { 1i64 } else { 0i64 }),
            _ => Err(RuntimeError::InvalidOperation(format!(
                "sys.wasm.call: argument {:?} cannot be converted to i64",
                v
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Get the module bytes from the registry
    let wasm_bytes = {
        let registry = WASM_REGISTRY.lock().unwrap();
        match registry.get(&handle) {
            Some(module) => {
                // Verify the function exists in exports
                if !module.exports.contains(&func_name) {
                    return Err(RuntimeError::InvalidOperation(format!(
                        "sys.wasm.call: function '{}' not found in module '{}'. Available: {:?}",
                        func_name, module.path, module.exports
                    )));
                }
                Arc::clone(&module.bytes)
            }
            None => {
                return Err(RuntimeError::InvalidOperation(format!(
                    "sys.wasm.call: no module with handle {}",
                    handle
                )));
            }
        }
    };
    // Registry lock is dropped here before the potentially long execution

    // Execute via wasmtime
    match crate::wasm_runner::call_exported(&wasm_bytes, &func_name, &i64_args) {
        Ok(Some(result)) => Ok(Value::Integer(result)),
        Ok(None) => Ok(Value::Unit),
        Err(e) => Err(RuntimeError::InvalidOperation(format!(
            "sys.wasm.call: execution failed: {}",
            e
        ))),
    }
}

/// sys.wasm.drop(handle) → Boolean
///
/// Unload a WASM module, freeing its memory.
pub fn intrinsic_wasm_drop(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "sys.wasm.drop expects 1 argument: (handle)".to_string(),
        ));
    }
    let handle = match &args[0] {
        Value::Integer(h) => *h,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.wasm.drop: argument must be an Integer handle".to_string(),
            ));
        }
    };

    let mut registry = WASM_REGISTRY.lock().unwrap();
    Ok(Value::Boolean(registry.remove(&handle).is_some()))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leb128_simple() {
        // Single byte: 0x05 = 5
        let (val, consumed) = read_leb128_u32(&[0x05]).unwrap();
        assert_eq!(val, 5);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_leb128_multibyte() {
        // Two bytes: 0x80 0x01 = 128
        let (val, consumed) = read_leb128_u32(&[0x80, 0x01]).unwrap();
        assert_eq!(val, 128);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_parse_exports_invalid_magic() {
        let bad = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
        assert!(parse_wasm_exports(&bad).is_err());
    }

    #[test]
    fn test_parse_exports_too_short() {
        let short = vec![0x00, 0x61, 0x73, 0x6D];
        assert!(parse_wasm_exports(&short).is_err());
    }

    #[test]
    fn test_parse_exports_minimal_wasm() {
        // Minimal valid WASM with no sections
        let wasm = vec![
            0x00, 0x61, 0x73, 0x6D, // magic: \0asm
            0x01, 0x00, 0x00, 0x00, // version: 1
        ];
        let exports = parse_wasm_exports(&wasm).unwrap();
        assert!(exports.is_empty());
    }

    #[test]
    fn test_wasm_load_nonexistent() {
        let args = vec![Value::String("/nonexistent/path.wasm".to_string())];
        let result = intrinsic_wasm_load(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_wasm_exports_bad_handle() {
        let args = vec![Value::Integer(999999)];
        let result = intrinsic_wasm_exports(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_wasm_drop_nonexistent() {
        let args = vec![Value::Integer(888888)];
        let result = intrinsic_wasm_drop(args).unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn test_wasm_call_bad_args() {
        let args = vec![Value::Integer(1)];
        let result = intrinsic_wasm_call(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_wasm_call_real_execution() {
        // Compile an Ark program with an exported function
        let source = "func add(a, b) { return a + b }\nprint(0)";
        let ast = crate::parser::parse_source(source, "test.ark").expect("parse failed");
        let wasm_bytes =
            crate::wasm_codegen::WasmCodegen::compile_to_bytes(&ast).expect("compile failed");

        // Write to temp file, load, call, verify
        let tmp_path = std::env::temp_dir().join("ark_interop_test.wasm");
        std::fs::write(&tmp_path, &wasm_bytes).expect("write failed");

        // Load the module
        let load_result =
            intrinsic_wasm_load(vec![Value::String(tmp_path.to_string_lossy().to_string())])
                .expect("load failed");

        let handle = match load_result {
            Value::Integer(h) => h,
            _ => panic!("expected Integer handle"),
        };

        // Call the 'add' function with args [10, 20]
        let call_result = intrinsic_wasm_call(vec![
            Value::Integer(handle),
            Value::String("add".to_string()),
            Value::List(vec![Value::Integer(10), Value::Integer(20)]),
        ])
        .expect("call failed");

        assert_eq!(call_result, Value::Integer(30), "add(10, 20) should be 30");

        // Clean up
        intrinsic_wasm_drop(vec![Value::Integer(handle)]).unwrap();
        let _ = std::fs::remove_file(&tmp_path);
    }

    #[test]
    fn test_wasm_call_missing_export() {
        let source = "print(1)";
        let ast = crate::parser::parse_source(source, "test.ark").expect("parse failed");
        let wasm_bytes =
            crate::wasm_codegen::WasmCodegen::compile_to_bytes(&ast).expect("compile failed");

        let tmp_path = std::env::temp_dir().join("ark_interop_missing.wasm");
        std::fs::write(&tmp_path, &wasm_bytes).expect("write failed");

        let handle =
            match intrinsic_wasm_load(vec![Value::String(tmp_path.to_string_lossy().to_string())])
                .unwrap()
            {
                Value::Integer(h) => h,
                _ => panic!("expected handle"),
            };

        let result = intrinsic_wasm_call(vec![
            Value::Integer(handle),
            Value::String("nonexistent".to_string()),
            Value::List(vec![]),
        ]);
        assert!(result.is_err(), "calling nonexistent export should fail");

        intrinsic_wasm_drop(vec![Value::Integer(handle)]).unwrap();
        let _ = std::fs::remove_file(&tmp_path);
    }

    #[test]
    fn test_wasm_call_non_integer_args() {
        let result = intrinsic_wasm_call(vec![
            Value::Integer(1),
            Value::String("foo".to_string()),
            Value::List(vec![Value::String("not_a_number".to_string())]),
        ]);
        // Should fail because string args can't be converted to i64
        assert!(result.is_err());
    }
}
