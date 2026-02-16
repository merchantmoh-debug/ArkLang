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

use wasm_bindgen::prelude::*;
use crate::compiler::Compiler;
use crate::loader::{load_ark_program, LoadError};
use crate::runtime::Value;
use crate::vm::VM;
use crate::checker::LinearChecker;

#[cfg(target_arch = "wasm32")]
use std::panic;
#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Reflect};

/// Initialize panic hook for WASM environment.
/// This ensures panics are logged to the browser console.
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(target_arch = "wasm32")]
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

/// Evaluates an Ark program (JSON MAST).
///
/// # Arguments
/// * `source` - A JSON string representing the Ark program (MAST).
///
/// # Returns
/// * A JSON string containing the result of the evaluation, or an error object.
///
/// # Example
/// ```js
/// const result = ark_eval('{"Expression": {"Literal": "3"}}');
/// console.log(result); // "3"
/// ```
#[wasm_bindgen]
pub fn ark_eval(source: &str) -> String {
    match load_ark_program(source) {
        Ok(mast) => {
            let compiler = Compiler::new();
            let chunk = compiler.compile(&mast.content);
            // One-shot VM: new VM, run, drop.
            match VM::new(chunk, &mast.hash, 0) {
                Ok(mut vm) => match vm.run() {
                    Ok(val) => {
                         value_to_json_string(&val)
                    },
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
        Ok(mast) => {
             match serde_json::to_string_pretty(&mast.content) {
                 Ok(s) => s,
                 Err(e) => make_error_json(&format!("Serialization Error: {}", e)),
             }
        }
        Err(e) => {
             match e {
                 LoadError::ParseError(err) => {
                     serde_json::json!({
                         "error": format!("{}", err),
                         "line": err.line(),
                         "column": err.column()
                     }).to_string()
                 }
                 _ => make_error_json(&format!("{}", e)),
            }
        }
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
                     serde_json::to_string_pretty(&vec![err_obj]).unwrap_or_else(|_| "[]".to_string())
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
        Ok(v) => {
            serde_json::to_string_pretty(&v).unwrap_or_else(|e| make_error_json(&format!("Format Error: {}", e)))
        }
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
        Value::List(l) => {
             serde_json::Value::Array(l.iter().map(value_to_json).collect())
        }
        Value::Struct(m) => {
             let map = m.iter().map(|(k, v)| (k.clone(), value_to_json(v))).collect();
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
            Value::Struct(m) => {
                let obj = Object::new();
                for (k, v) in m {
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
