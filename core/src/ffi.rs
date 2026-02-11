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

use crate::compiler::Compiler;
use crate::loader::load_ark_program;
use crate::vm::VM;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char};

/// Evaluates a JSON string representing an Ark AST.
/// Returns a pointer to a C-string containing the result (Debug formatted).
/// The caller must free the returned string using `ark_free_string`.
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
/// The caller must ensure `json_ptr` is a valid pointer to a null-terminated C string.
#[no_mangle]
pub extern "C" fn ark_eval_string(json_ptr: *const c_char) -> *mut c_char {
    if json_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(json_ptr) };
    let json_str = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => return CString::new(format!("Error: Invalid UTF-8: {}", e)).unwrap().into_raw(),
    };

    let node = match load_ark_program(json_str) {
        Ok(n) => n,
        Err(e) => return CString::new(format!("Error: {}", e)).unwrap().into_raw(),
    };

    let compiler = Compiler::new();
    let chunk = compiler.compile(&node);
    let mut vm = VM::new(chunk);

    match vm.run() {
        Ok(val) => {
            let output = format!("{:?}", val);
            CString::new(output).unwrap().into_raw()
        }
        Err(e) => CString::new(format!("Error: {}", e)).unwrap().into_raw(),
    }
}

/// Frees a string returned by `ark_eval_string`.
#[no_mangle]
pub extern "C" fn ark_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_ark_eval_string() {
        // Return a literal string: "Hello FFI"
        // JSON structure: {"Statement": {"Return": {"Literal": "Hello FFI"}}}
        let json = r#"{"Statement": {"Return": {"Literal": "Hello FFI"}}}"#;
        let c_json = CString::new(json).unwrap();

        let result_ptr = ark_eval_string(c_json.as_ptr());
        assert!(!result_ptr.is_null());

        let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
        let result_str = result_cstr.to_str().unwrap();

        // Value::String("Hello FFI") debug format is String("Hello FFI")
        assert_eq!(result_str, r#"String("Hello FFI")"#);

        ark_free_string(result_ptr);
    }

    #[test]
    fn test_ark_eval_invalid_json() {
        let json = r#"{"Invalid": "JSON"}"#;
        let c_json = CString::new(json).unwrap();

        let result_ptr = ark_eval_string(c_json.as_ptr());
        assert!(!result_ptr.is_null());

        let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
        let result_str = result_cstr.to_str().unwrap();

        assert!(result_str.starts_with("Error:"));

        ark_free_string(result_ptr);
    }
}
