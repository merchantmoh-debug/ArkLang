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
use std::os::raw::c_char;

/// Helper to safely create a CString from a Rust String.
/// If the string contains null bytes, it returns an error message C-string.
fn safe_cstring(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => {
            // Return a safe error message if the string contains a null byte.
            // This is safe because the error message is guaranteed not to contain null bytes.
            CString::new("Error: String contained null byte")
                .unwrap()
                .into_raw()
        }
    }
}

/// Evaluates a JSON string representing an Ark AST.
/// Returns a pointer to a C-string containing the result (Debug formatted).
/// The caller must free the returned string using `ark_free_string`.
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
/// The caller must ensure `json_ptr` is a valid pointer to a null-terminated C string.
#[no_mangle]
/// # Safety
/// The caller must ensure `json_ptr` is a valid, non-null pointer to a null-terminated C string.
pub unsafe extern "C" fn ark_eval_string(json_ptr: *const c_char) -> *mut c_char {
    if json_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = CStr::from_ptr(json_ptr);
    let json_str = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => return safe_cstring(format!("Error: Invalid UTF-8: {}", e)),
    };

    let mast = match load_ark_program(json_str) {
        Ok(n) => n,
        Err(e) => return safe_cstring(format!("Error: {}", e)),
    };

    let compiler = Compiler::new();
    let chunk = compiler.compile(&mast.content);
    let mut vm = match VM::new(chunk, &mast.hash, 0) {
        Ok(v) => v,
        Err(e) => return safe_cstring(format!("Error: {}", e)),
    };

    match vm.run() {
        Ok(val) => {
            let output = format!("{:?}", val);
            safe_cstring(output)
        }
        Err(e) => safe_cstring(format!("Error: {}", e)),
    }
}

/// Frees a string returned by `ark_eval_string`.
#[no_mangle]
/// # Safety
/// The caller must ensure `ptr` was returned by `ark_eval_string` and has not been freed already.
pub unsafe extern "C" fn ark_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_ark_eval_string() {
        let json = r#"{"Statement": {"Expression": {"Literal": "Hello FFI"}}}"#;
        let c_json = CString::new(json).unwrap();

        let result_ptr = unsafe { ark_eval_string(c_json.as_ptr()) };
        assert!(!result_ptr.is_null());

        let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
        let result_str = result_cstr.to_str().unwrap();

        assert_eq!(result_str, "String(\"Hello FFI\")");

        unsafe { ark_free_string(result_ptr) };
    }

    #[test]
    fn test_ark_eval_expression() {
        let json = r#"{"Statement": {"Expression": {"Call": {"function_hash": "add", "args": [{"Integer": 10}, {"Integer": 20}]}}}}"#;
        let c_json = CString::new(json).unwrap();

        let result_ptr = unsafe { ark_eval_string(c_json.as_ptr()) };
        assert!(!result_ptr.is_null());

        let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
        let result_str = result_cstr.to_str().unwrap();

        assert_eq!(result_str, "Integer(30)");

        unsafe { ark_free_string(result_ptr) };
    }

    #[test]
    fn test_safe_cstring_interior_null() {
        let s = String::from("Hello\0World");
        let ptr = safe_cstring(s);

        let c_str = unsafe { CStr::from_ptr(ptr) };
        let str_slice = c_str.to_str().unwrap();

        assert_eq!(str_slice, "Error: String contained null byte");

        unsafe { ark_free_string(ptr) };
    }

    // --- FFI Tests (Disabled: Implementation moved/removed) ---
    // Tests for ffi_call, ffi_load_library, etc. are temporarily removed until re-implementation.
}
