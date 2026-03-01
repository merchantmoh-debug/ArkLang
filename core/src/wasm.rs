/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * 1. OPEN SOURCE: You may use this file under the terms of the GNU Affero
 * General Public License v3.0. If you link to this code, your ENTIRE
 * application must be open-sourced under AGPLv3.
 *
 * 2. COMMERCIAL: For proprietary use, you must obtain a Commercial License
 * from Sovereign Systems.
 *
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 * NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.
 */

use crate::checker::LinearChecker;
use crate::compiler::Compiler;
use crate::loader::{LoadError, load_ark_program};
use crate::parser;
use crate::runtime::Value;
use crate::vm::VM;
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Reflect};
#[cfg(target_arch = "wasm32")]
use std::panic;

use std::cell::RefCell;

// Thread-local buffers for WASM
thread_local! {
    static OUTPUT_BUFFER: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static MAST_SOURCE: RefCell<Option<String>> = const { RefCell::new(None) };
}

// ─── Raw C-style FFI for snake.html ─────────────────────────────────────────
// These functions are called directly via WebAssembly.instantiate (no wasm-bindgen).
// The snake game JS bridge uses: ark_alloc, ark_dealloc, ark_init, ark_call.

/// Allocate `len` bytes and return a pointer. Caller owns the memory.
#[no_mangle]
pub extern "C" fn ark_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Deallocate `len` bytes starting at `ptr`.
#[no_mangle]
pub unsafe extern "C" fn ark_dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Vec::from_raw_parts(ptr, 0, len);
    }
}

/// Write a string result into a length-prefixed buffer: [len: u32 LE][content: bytes]
fn write_result(s: &str) -> *mut u8 {
    let bytes = s.as_bytes();
    let total = 4 + bytes.len();
    let ptr = ark_alloc(total);
    unsafe {
        // Write length as little-endian u32
        let len_bytes = (bytes.len() as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(len_bytes.as_ptr(), ptr, 4);
        // Write content
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());
    }
    ptr
}

/// Initialize the Ark VM with a MAST JSON program.
/// The JS bridge sends the snake.json content here.
#[no_mangle]
pub unsafe extern "C" fn ark_init(input_ptr: *mut u8, input_len: usize) -> *mut u8 {
    let input_slice = std::slice::from_raw_parts(input_ptr, input_len);
    let source = match std::str::from_utf8(input_slice) {
        Ok(s) => s,
        Err(_) => return write_result("Error: Invalid UTF-8"),
    };

    // Validate that the JSON is parseable as an Ark program
    match crate::loader::load_ark_program(source) {
        Ok(_) => {
            // Store source for later calls
            MAST_SOURCE.with(|src| {
                *src.borrow_mut() = Some(source.to_string());
            });
            write_result("OK")
        }
        Err(e) => write_result(&format!("Error: {:?}", e)),
    }
}

/// Call a named Ark function with JSON arguments.
/// The JS bridge calls this for "init" and "update" game functions.
#[no_mangle]
pub unsafe extern "C" fn ark_call(
    name_ptr: *mut u8,
    name_len: usize,
    args_ptr: *mut u8,
    args_len: usize,
) -> *mut u8 {
    let name_slice = std::slice::from_raw_parts(name_ptr, name_len);
    let name = match std::str::from_utf8(name_slice) {
        Ok(s) => s,
        Err(_) => return write_result(r#"{"error":"Invalid function name"}"#),
    };

    let args_slice = std::slice::from_raw_parts(args_ptr, args_len);
    let args_str = match std::str::from_utf8(args_slice) {
        Ok(s) => s,
        Err(_) => return write_result(r#"{"error":"Invalid args UTF-8"}"#),
    };

    // Parse arguments
    let args: Vec<serde_json::Value> = match serde_json::from_str(args_str) {
        Ok(v) => v,
        Err(_) => return write_result(r#"{"error":"Invalid args JSON"}"#),
    };

    // Get stored source
    let source = MAST_SOURCE.with(|src| src.borrow().clone());
    let source = match source {
        Some(s) => s,
        None => return write_result(r#"{"error":"VM not initialized. Call ark_init first."}"#),
    };

    // Load the program
    let mast = match crate::loader::load_ark_program(&source) {
        Ok(m) => m,
        Err(e) => return write_result(&format!(r#"{{"error":"Load: {:?}"}}"#, e)),
    };

    // Clear output buffer
    OUTPUT_BUFFER.with(|buf| buf.borrow_mut().clear());

    // Compile the entire program (this registers all function definitions)
    let compiler = Compiler::new();
    let chunk = compiler.compile(&mast.content);

    // Create VM and run the program to register function definitions
    match VM::new(chunk, &mast.hash, 0) {
        Ok(mut vm) => {
            // Run the top-level code first (this defines all functions in scope)
            match vm.run() {
                Ok(_) => {}
                Err(e) => {
                    return write_result(&format!(r#"{{"error":"Init run: {:?}"}}"#, e));
                }
            }

            // Convert JSON args to Ark Values
            let ark_args: Vec<Value> = args.iter().map(json_to_value).collect();

            // Now call the requested function
            match vm.call_public_function(name, ark_args) {
                Ok(val) => {
                    let json = value_to_json(&val);
                    write_result(&json.to_string())
                }
                Err(e) => write_result(&format!(r#"{{"error":"Runtime: {:?}"}}"#, e)),
            }
        }
        Err(e) => write_result(&format!(r#"{{"error":"VM Init: {:?}"}}"#, e)),
    }
}

/// Convert a serde_json::Value to an Ark Value
fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else {
                Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => Value::List(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(map) => {
            let fields: Vec<(String, Value)> = map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_value(v)))
                .collect();
            Value::Struct(fields)
        }
    }
}

/// Append a line to the WASM output buffer (called by VM's print intrinsic).
pub fn wasm_print(msg: &str) {
    OUTPUT_BUFFER.with(|buf| {
        buf.borrow_mut().push(msg.to_string());
    });
}

/// Drain and return captured output.
fn drain_output() -> String {
    OUTPUT_BUFFER.with(|buf| {
        let lines: Vec<String> = buf.borrow_mut().drain(..).collect();
        lines.join("\n")
    })
}

/// Initialize panic hook for WASM environment.
/// This ensures panics are logged to the browser console.
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(target_arch = "wasm32")]
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

/// Evaluates raw Ark source code (.ark syntax).
///
/// This is the primary entry point for the WASM playground.
/// It uses the native Rust parser — no JSON MAST needed.
///
/// # Arguments
/// * `source` - Raw Ark source code (e.g., `func main() { print("hi") }`)
///
/// # Returns
/// * A JSON object: `{ "result": "...", "stdout": "...", "error": null }`
#[wasm_bindgen]
pub fn ark_eval_source(source: &str) -> String {
    // Clear output buffer
    OUTPUT_BUFFER.with(|buf| buf.borrow_mut().clear());

    // 1. Parse
    let ast = match parser::parse_source(source, "<wasm>") {
        Ok(node) => node,
        Err(e) => {
            return serde_json::json!({
                "result": null,
                "stdout": "",
                "error": format!("Parse Error: {}", e)
            })
            .to_string();
        }
    };

    // 2. Compile
    let compiler = Compiler::new();
    let chunk = compiler.compile(&ast);

    // 3. Execute
    let hash = "wasm_playground";
    match VM::new(chunk, hash, 0) {
        Ok(mut vm) => match vm.run() {
            Ok(val) => {
                let stdout = drain_output();
                serde_json::json!({
                    "result": value_to_json(&val),
                    "stdout": stdout,
                    "error": null
                })
                .to_string()
            }
            Err(e) => {
                let stdout = drain_output();
                serde_json::json!({
                    "result": null,
                    "stdout": stdout,
                    "error": format!("Runtime Error: {}", e)
                })
                .to_string()
            }
        },
        Err(e) => serde_json::json!({
            "result": null,
            "stdout": "",
            "error": format!("VM Init Error: {}", e)
        })
        .to_string(),
    }
}

/// Parses raw Ark source code and returns the AST as JSON.
///
/// # Arguments
/// * `source` - Raw Ark source code.
///
/// # Returns
/// * The formatted JSON AST string, or `{ "error": "..." }` on failure.
#[wasm_bindgen]
pub fn ark_parse_source(source: &str) -> String {
    match parser::parse_source(source, "<wasm>") {
        Ok(node) => match serde_json::to_string_pretty(&node) {
            Ok(s) => s,
            Err(e) => make_error_json(&format!("Serialization Error: {}", e)),
        },
        Err(e) => serde_json::json!({
            "error": format!("{}", e),
        })
        .to_string(),
    }
}

/// Runs the linear type checker on raw Ark source code.
#[wasm_bindgen]
pub fn ark_check_source(source: &str) -> String {
    match parser::parse_source(source, "<wasm>") {
        Ok(node) => match LinearChecker::check(&node) {
            Ok(_) => "[]".to_string(),
            Err(e) => {
                let err_obj = serde_json::json!({
                    "error": format!("{}", e),
                    "type": "LinearError"
                });
                serde_json::to_string_pretty(&vec![err_obj]).unwrap_or_else(|_| "[]".to_string())
            }
        },
        Err(e) => make_error_json(&format!("Parse Error: {}", e)),
    }
}

/// Evaluates an Ark program (JSON MAST) — legacy interface.
///
/// # Arguments
/// * `source` - A JSON string representing the Ark program (MAST).
///
/// # Returns
/// * A JSON string containing the result of the evaluation, or an error object.
#[wasm_bindgen]
pub fn ark_eval(source: &str) -> String {
    match load_ark_program(source) {
        Ok(mast) => {
            let compiler = Compiler::new();
            let chunk = compiler.compile(&mast.content);
            match VM::new(chunk, &mast.hash, 0) {
                Ok(mut vm) => match vm.run() {
                    Ok(val) => value_to_json_string(&val),
                    Err(e) => make_error_json(&format!("Runtime Error: {:?}", e)),
                },
                Err(e) => make_error_json(&format!("VM Init Error: {:?}", e)),
            }
        }
        Err(e) => make_error_json(&format!("Load Error: {:?}", e)),
    }
}

/// Parses Ark source code (JSON MAST) and validates it.
/// Returns the AST as a JSON string.
///
/// # Arguments
/// * `source` - A JSON string representing the Ark program.
///
/// # Returns
/// * The formatted JSON AST string.
/// * On error: `{"error": "...", "line": N, "column": N}`.
#[wasm_bindgen]
pub fn ark_parse(source: &str) -> String {
    match load_ark_program(source) {
        Ok(mast) => match serde_json::to_string_pretty(&mast.content) {
            Ok(s) => s,
            Err(e) => make_error_json(&format!("Serialization Error: {}", e)),
        },
        Err(e) => match e {
            LoadError::ParseError(err) => serde_json::json!({
                "error": format!("{}", err),
                "line": err.line(),
                "column": err.column()
            })
            .to_string(),
            _ => make_error_json(&format!("{}", e)),
        },
    }
}

/// Runs the type checker (Linear Type System) on the source code.
///
/// # Arguments
/// * `source` - A JSON string representing the Ark program.
///
/// # Returns
/// * A JSON array of error objects, or `[]` if valid.
#[wasm_bindgen]
pub fn ark_check(source: &str) -> String {
    match load_ark_program(source) {
        Ok(mast) => {
            match LinearChecker::check(&mast.content) {
                Ok(_) => "[]".to_string(), // No errors
                Err(e) => {
                    let err_obj = serde_json::json!({
                        "error": format!("{}", e),
                        "type": "LinearError"
                    });
                    serde_json::to_string_pretty(&vec![err_obj])
                        .unwrap_or_else(|_| "[]".to_string())
                }
            }
        }
        Err(e) => make_error_json(&format!("Load Error: {:?}", e)),
    }
}

/// Formats Ark source code (JSON MAST).
///
/// # Arguments
/// * `source` - A JSON string representing the Ark program.
///
/// # Returns
/// * A pretty-printed JSON string.
#[wasm_bindgen]
pub fn ark_format(source: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(source) {
        Ok(v) => serde_json::to_string_pretty(&v)
            .unwrap_or_else(|e| make_error_json(&format!("Format Error: {}", e))),
        Err(e) => make_error_json(&format!("JSON Parse Error: {}", e)),
    }
}

/// Returns the current version of the Ark Core.
#[wasm_bindgen]
pub fn ark_version() -> String {
    "0.1.0".to_string()
}

// Helpers

fn make_error_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

fn value_to_json_string(v: &Value) -> String {
    let json_val = value_to_json(v);
    json_val.to_string()
}

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Unit => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Integer(i) => serde_json::Value::Number((*i).into()),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::List(l) => serde_json::Value::Array(l.iter().map(value_to_json).collect()),
        Value::PVec(pv) => {
            serde_json::Value::Array(pv.to_vec().iter().map(value_to_json).collect())
        }
        Value::Struct(m) => {
            let map = m
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        Value::PMap(pm) => {
            let map = pm
                .entries()
                .into_iter()
                .map(|(k, v)| (k, value_to_json(&v)))
                .collect();
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::String(format!("{:?}", v)),
    }
}

// Point 4: JAVASCRIPT INTEROP TYPES
// Implement From<Value> for JsValue
#[cfg(target_arch = "wasm32")]
impl From<Value> for JsValue {
    fn from(val: Value) -> Self {
        match val {
            Value::Unit => JsValue::NULL,
            Value::Boolean(b) => JsValue::from_bool(b),
            Value::Integer(i) => JsValue::from_f64(i as f64),
            Value::String(s) => JsValue::from_str(&s),
            Value::List(l) => {
                let arr = Array::new();
                for item in l {
                    arr.push(&JsValue::from(item));
                }
                arr.into()
            }
            Value::PVec(pv) => {
                let arr = Array::new();
                for item in pv.to_vec() {
                    arr.push(&JsValue::from(item));
                }
                arr.into()
            }
            Value::Struct(m) => {
                let obj = Object::new();
                for (k, v) in m {
                    let _ = Reflect::set(&obj, &JsValue::from_str(&k), &JsValue::from(v));
                }
                obj.into()
            }
            Value::PMap(pm) => {
                let obj = Object::new();
                for (k, v) in pm.entries() {
                    let _ = Reflect::set(&obj, &JsValue::from_str(&k), &JsValue::from(v));
                }
                obj.into()
            }
            _ => JsValue::from_str(&format!("{:?}", val)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{ArkNode, Expression};

    #[test]
    fn test_ark_version() {
        assert_eq!(ark_version(), "0.1.0");
    }

    #[test]
    fn test_ark_eval_simple() {
        // Create a minimal valid ArkNode
        let content = ArkNode::Expression(Expression::Literal("3".to_string()));
        let source = serde_json::to_string(&content).unwrap();

        let result_json = ark_eval(&source);
        assert_eq!(result_json, "\"3\"");
    }

    #[test]
    fn test_ark_parse_error() {
        let source = "!!!";
        let result_json = ark_parse(source);
        let result: serde_json::Value = serde_json::from_str(&result_json).unwrap();
        assert!(result.get("error").is_some());
        assert!(result.get("line").is_some());
        assert!(result.get("column").is_some());
    }

    #[test]
    fn test_ark_check_valid() {
        let content = ArkNode::Expression(Expression::Literal("3".to_string()));
        let source = serde_json::to_string(&content).unwrap();

        let result = ark_check(&source);
        assert_eq!(result, "[]");
    }
}
