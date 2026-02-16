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
use crate::runtime::Value as ArkValue;
use crate::vm::VM;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long, c_void};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

// --- FFI Error Types ---

#[derive(Debug, Clone)]
pub enum FfiError {
    LibraryNotFound(String),
    FunctionNotFound(String),
    TypeConversionError(String),
    CallFailed(String),
    Timeout(String),
    InvalidPointer(String),
    SecurityError(String),
}

impl std::fmt::Display for FfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FfiError::LibraryNotFound(s) => write!(f, "Library not found: {}", s),
            FfiError::FunctionNotFound(s) => write!(f, "Function not found: {}", s),
            FfiError::TypeConversionError(s) => write!(f, "Type conversion error: {}", s),
            FfiError::CallFailed(s) => write!(f, "FFI call failed: {}", s),
            FfiError::Timeout(s) => write!(f, "FFI call timed out: {}", s),
            FfiError::InvalidPointer(s) => write!(f, "Invalid pointer: {}", s),
            FfiError::SecurityError(s) => write!(f, "Security error: {}", s),
        }
    }
}

// --- Global State ---

static ALLOWED_LIBRARIES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static LOADED_LIBRARIES: OnceLock<Mutex<HashMap<String, LibraryHandle>>> = OnceLock::new();

fn get_allowed_libraries() -> &'static Mutex<Vec<String>> {
    ALLOWED_LIBRARIES.get_or_init(|| Mutex::new(Vec::new()))
}

fn get_loaded_libraries() -> &'static Mutex<HashMap<String, LibraryHandle>> {
    LOADED_LIBRARIES.get_or_init(|| Mutex::new(HashMap::new()))
}

// --- Library Handle ---

#[derive(Debug)]
pub enum LibraryHandle {
    #[cfg(feature = "use_libloading")]
    Real(libloading::Library),
    Stub(String),
}

#[derive(Debug, Clone)]
pub enum FunctionHandle {
    #[cfg(feature = "use_libloading")]
    Real(usize), // We can't store Symbol generically easily, so we might store address or generic symbol
    // Actually, libloading::Symbol<'lib, T> is bound to library lifetime.
    // For simplicity in this structure, we might need to unsafe transmute or store raw pointer.
    // storing raw address is easiest.
    Ptr(usize),
}

impl FunctionHandle {
    pub fn as_ptr(&self) -> *const c_void {
        match self {
            #[cfg(feature = "use_libloading")]
            FunctionHandle::Real(addr) => *addr as *const c_void,
            FunctionHandle::Ptr(addr) => *addr as *const c_void,
        }
    }
}

// --- FFI Interface ---

pub fn ffi_load_library(path: &str) -> Result<String, FfiError> {
    // 1. Security Check
    let allowed = get_allowed_libraries().lock().unwrap();
    if !allowed.contains(&path.to_string()) && !allowed.contains(&"*".to_string()) {
        return Err(FfiError::SecurityError(format!(
            "Library '{}' is not in the allowed list",
            path
        )));
    }
    drop(allowed);

    // 2. Load Library
    let mut loaded = get_loaded_libraries().lock().unwrap();
    if loaded.contains_key(path) {
        return Ok(path.to_string());
    }

    #[cfg(feature = "use_libloading")]
    {
        // Real Implementation
        // unsafe {
        //     let lib = libloading::Library::new(path)
        //         .map_err(|e| FfiError::LibraryNotFound(format!("{}: {}", path, e)))?;
        //     loaded.insert(path.to_string(), LibraryHandle::Real(lib));
        // }
        // Note: Code is commented out or behind feature gate as per prompt instructions
        // about missing dependency. Since I can't compile it, I stub it.
        Err(FfiError::LibraryNotFound("Libloading feature not enabled".to_string()))
    }

    #[cfg(not(feature = "use_libloading"))]
    {
        // Stub Implementation
        loaded.insert(path.to_string(), LibraryHandle::Stub(path.to_string()));
        Ok(path.to_string())
    }
}

pub fn ffi_get_function(lib_path: &str, func_name: &str) -> Result<FunctionHandle, FfiError> {
    let loaded = get_loaded_libraries().lock().unwrap();
    let lib = loaded
        .get(lib_path)
        .ok_or_else(|| FfiError::LibraryNotFound(lib_path.to_string()))?;

    match lib {
        #[cfg(feature = "use_libloading")]
        LibraryHandle::Real(library) => {
            // Real Implementation
            // unsafe {
            //     let func: libloading::Symbol<unsafe extern "C" fn()> = library.get(func_name.as_bytes())
            //         .map_err(|e| FfiError::FunctionNotFound(format!("{}: {}", func_name, e)))?;
            //     Ok(FunctionHandle::Real(func.into_raw().into_raw() as usize))
            // }
             Err(FfiError::FunctionNotFound("Libloading feature not enabled".to_string()))
        }
        LibraryHandle::Stub(_) => {
            // For testing, we might want to return a dummy handle if checking logic.
            // Or return error.
            // If we are testing ffi_call with manual pointers, we don't use this.
            // If we want to simulate finding a function, we'd need a registry of stubs.
            Err(FfiError::FunctionNotFound(format!("{} (Stub)", func_name)))
        }
    }
}

/// Helper to configure security
pub fn ffi_allow_library(path: &str) {
    let mut allowed = get_allowed_libraries().lock().unwrap();
    if !allowed.contains(&path.to_string()) {
        allowed.push(path.to_string());
    }
}

// --- FFI Call Wrapper ---

/// Calls a C function with the given arguments.
///
/// # Safety
/// This function is unsafe because it calls arbitrary C code.
/// The caller must ensure the function pointer and arguments are valid.
pub fn ffi_call(
    func_handle: &FunctionHandle,
    args: &[ArkValue],
    return_type: &str,
) -> Result<ArkValue, FfiError> {
    let func_ptr = func_handle.as_ptr();
    if func_ptr.is_null() {
        return Err(FfiError::InvalidPointer("Function pointer is null".to_string()));
    }

    // 1. Type Conversion
    // We need to keep alive any CStrings or Vecs created during conversion until after the call.
    let mut _keep_alive_strings: Vec<CString> = Vec::new();
    let mut _keep_alive_vecs: Vec<Vec<c_long>> = Vec::new();
    let mut flat_args: Vec<u64> = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        match arg {
            ArkValue::Integer(n) => {
                flat_args.push(*n as u64);
            }
            ArkValue::Boolean(b) => {
                flat_args.push(if *b { 1 } else { 0 });
            }
            ArkValue::String(s) => {
                let c_str = CString::new(s.clone())
                    .map_err(|_| FfiError::TypeConversionError(format!("Arg {}: String contained null byte", i)))?;
                let ptr = c_str.as_ptr() as u64;
                _keep_alive_strings.push(c_str);
                flat_args.push(ptr);
            }
            ArkValue::List(items) => {
                // Convert list of integers to Vec<c_long>
                let mut c_vec: Vec<c_long> = Vec::with_capacity(items.len());
                for item in items {
                    if let ArkValue::Integer(v) = item {
                         c_vec.push(*v as c_long);
                    } else {
                         return Err(FfiError::TypeConversionError(format!("Arg {}: List must contain integers", i)));
                    }
                }
                let ptr = c_vec.as_ptr() as u64;
                let len = c_vec.len() as u64;
                _keep_alive_vecs.push(c_vec);

                // Pass Pointer AND Length
                flat_args.push(ptr);
                flat_args.push(len);
            }
            _ => return Err(FfiError::TypeConversionError(format!("Arg {}: Unsupported type {:?}", i, arg))),
        }
    }

    if flat_args.len() > 6 {
        return Err(FfiError::CallFailed("Too many arguments (max 6)".to_string()));
    }

    // 2. Safe Call with Timeout
    // We wrap the unsafe call in catch_unwind.
    // Timeout is harder without spawning a thread, but prompt asks for it.
    // Spawning a thread for FFI might be necessary if the C code hangs.

    // Since we need to pass `flat_args` and `func_ptr` to the closure, they must be Send.
    // Pointers are not Send. We cast to usize which is Send.
    let func_addr = func_ptr as usize;
    let call_args = flat_args.clone();

    // "Set a timeout for FFI calls (default 10 seconds) using a watchdog thread"
    // I'll implement a channel-based timeout.
    // But `thread::spawn` returns a JoinHandle.
    // I can't just kill it.
    // I will simulate timeout logic: if it doesn't return in time, I return Timeout error.
    // The thread will continue to run (detached) effectively leaking if it hangs.
    // This is the best we can do in safe Rust without specific OS APIs.

    // Use `std::sync::mpsc`.
    let (tx, rx) = std::sync::mpsc::channel();

    let t_handle = thread::spawn(move || {
        let func_ptr = func_addr as *const c_void;
        let res = std::panic::catch_unwind(move || {
            unsafe { dispatch_call(func_ptr, &call_args) }
        });
        let _ = tx.send(res);
    });

    let timeout = Duration::from_secs(10);
    match rx.recv_timeout(timeout) {
        Ok(thread_res) => {
             let _ = t_handle.join(); // Clean up
             match thread_res {
                 Ok(raw_ret) => {
                     // 3. Return Value Conversion
                     convert_return_value(raw_ret, return_type)
                 }
                 Err(_) => Err(FfiError::CallFailed("FFI Panic".to_string())),
             }
        }
        Err(_) => {
            // Timed out
            Err(FfiError::Timeout("FFI call timed out".to_string()))
        }
    }
}

unsafe fn dispatch_call(func_ptr: *const c_void, args: &[u64]) -> u64 {
    match args.len() {
        0 => {
            let func: extern "C" fn() -> u64 = std::mem::transmute(func_ptr);
            func()
        }
        1 => {
            let func: extern "C" fn(u64) -> u64 = std::mem::transmute(func_ptr);
            func(args[0])
        }
        2 => {
            let func: extern "C" fn(u64, u64) -> u64 = std::mem::transmute(func_ptr);
            func(args[0], args[1])
        }
        3 => {
            let func: extern "C" fn(u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            func(args[0], args[1], args[2])
        }
        4 => {
            let func: extern "C" fn(u64, u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            func(args[0], args[1], args[2], args[3])
        }
        5 => {
            let func: extern "C" fn(u64, u64, u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            func(args[0], args[1], args[2], args[3], args[4])
        }
        6 => {
            let func: extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64 = std::mem::transmute(func_ptr);
            func(args[0], args[1], args[2], args[3], args[4], args[5])
        }
        _ => 0, // Should be caught by check above
    }
}

fn convert_return_value(raw: u64, ret_type: &str) -> Result<ArkValue, FfiError> {
    match ret_type {
        "Integer" | "i64" | "long" => Ok(ArkValue::Integer(raw as i64)),
        "Boolean" | "bool" => Ok(ArkValue::Boolean(raw != 0)),
        "String" | "string" => {
            if raw == 0 {
                return Ok(ArkValue::String("".to_string()));
            }
            let ptr = raw as *const c_char;
            unsafe {
                let c_str = CStr::from_ptr(ptr);
                Ok(ArkValue::String(c_str.to_string_lossy().into_owned()))
            }
        }
        "Unit" | "void" => Ok(ArkValue::Unit),
        _ => Err(FfiError::TypeConversionError(format!("Unknown return type: {}", ret_type))),
    }
}

// --- Existing ark_eval_string code ---

/// Helper to safely create a CString from a Rust String.
/// If the string contains null bytes, it returns an error message C-string.
fn safe_cstring(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => {
            // Return a safe error message if the string contains a null byte.
            // This is safe because the error message is guaranteed not to contain null bytes.
            CString::new("Error: String contained null byte").unwrap().into_raw()
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
pub extern "C" fn ark_eval_string(json_ptr: *const c_char) -> *mut c_char {
    if json_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(json_ptr) };
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

        // VM correctly returns the value for top-level returns.
        assert_eq!(result_str, "String(\"Hello FFI\")");

        ark_free_string(result_ptr);
    }

    #[test]
    fn test_safe_cstring_interior_null() {
        // String with interior null
        let s = String::from("Hello\0World");
        let ptr = safe_cstring(s);

        let c_str = unsafe { CStr::from_ptr(ptr) };
        let str_slice = c_str.to_str().unwrap();

        // It should return the safe error message
        assert_eq!(str_slice, "Error: String contained null byte");

        ark_free_string(ptr);
    }

    // --- FFI Tests ---

    // Define a C-compatible function for testing
    #[no_mangle]
    pub extern "C" fn test_add(a: u64, b: u64) -> u64 {
        a + b
    }

    #[no_mangle]
    pub extern "C" fn test_strlen(s: *const c_char) -> u64 {
        if s.is_null() { return 0; }
        unsafe {
            CStr::from_ptr(s).to_bytes().len() as u64
        }
    }

    #[test]
    fn test_ffi_call_add() {
        let func_ptr = test_add as usize;
        let handle = FunctionHandle::Ptr(func_ptr);

        let args = vec![ArkValue::Integer(10), ArkValue::Integer(20)];
        let result = ffi_call(&handle, &args, "Integer");

        assert!(result.is_ok());
        if let Ok(ArkValue::Integer(val)) = result {
            assert_eq!(val, 30);
        } else {
            panic!("Expected Integer result");
        }
    }

    #[test]
    fn test_ffi_call_strlen() {
        let func_ptr = test_strlen as usize;
        let handle = FunctionHandle::Ptr(func_ptr);

        let args = vec![ArkValue::String("Hello".to_string())];
        let result = ffi_call(&handle, &args, "Integer");

        assert!(result.is_ok());
        if let Ok(ArkValue::Integer(val)) = result {
            assert_eq!(val, 5);
        } else {
            panic!("Expected Integer result");
        }
    }

    #[test]
    fn test_ffi_security_check() {
        // Clear allowed list (might affect other tests if run in parallel, but here ok)
        // get_allowed_libraries().lock().unwrap().clear();
        // Better: ensure "libc" is not in list.

        let res = ffi_load_library("dangerous_lib");
        assert!(matches!(res, Err(FfiError::SecurityError(_))));

        ffi_allow_library("safe_lib");
        // Should pass security check, but fail loading (stub)
        let res = ffi_load_library("safe_lib");
        assert!(res.is_ok()); // Stub returns Ok(path)
    }
}
