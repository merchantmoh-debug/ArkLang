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
use std::mem;
use std::slice;
use std::str;

/// Allocates memory for a string of `size` bytes.
/// Returns a pointer to the allocated memory.
#[no_mangle]
pub extern "C" fn ark_alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}

/// Deallocates memory.
#[no_mangle]
pub unsafe extern "C" fn ark_dealloc(ptr: *mut u8, size: usize) {
    let _ = Vec::from_raw_parts(ptr, 0, size);
}

/// Evaluates an Ark program (JSON MAST).
///
/// Input:
/// - input_ptr: Pointer to the JSON string.
/// - input_len: Length of the JSON string.
///
/// Output:
/// - Returns a pointer to a buffer containing [len (u32) + content (utf8)].
/// - The caller is responsible for freeing this buffer via ark_dealloc (size = len + 4).
#[no_mangle]
pub unsafe extern "C" fn ark_eval(input_ptr: *mut u8, input_len: usize) -> *mut u8 {
    // 1. Reconstruct the input string
    let input_slice = slice::from_raw_parts(input_ptr, input_len);
    let input_str = match str::from_utf8(input_slice) {
        Ok(s) => s,
        Err(_) => return make_response("Error: Invalid UTF-8 input"),
    };

    // 2. Load and Execute
    let response = match load_ark_program(input_str) {
        Ok(node) => {
            let compiler = Compiler::new();
            let chunk = compiler.compile(&node);
            let mut vm = VM::new(chunk);
            match vm.run() {
                Ok(val) => format!("Result: {:?}", val),
                Err(e) => format!("Runtime Error: {:?}", e),
            }
        }
        Err(e) => format!("Load Error: {:?}", e),
    };

    // 3. Serialize Response
    make_response(&response)
}

unsafe fn make_response(s: &str) -> *mut u8 {
    let bytes = s.as_bytes();
    let len = bytes.len() as u32;

    // Layout: [len (4 bytes)] [content...]
    let mut buf = Vec::with_capacity(4 + bytes.len());

    // Write Length (Little Endian)
    buf.extend_from_slice(&len.to_le_bytes());
    // Write Content
    buf.extend_from_slice(bytes);

    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}
