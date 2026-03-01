/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * Wasmtime Runtime Execution for Ark WASM Binaries.
 *
 * Executes compiled Ark .wasm files via wasmtime with lightweight
 * WASI fd_write support (no full wasmtime-wasi dependency).
 *
 * The compiled Ark WASM imports a single WASI function:
 *   wasi_snapshot_preview1::fd_write(fd, iovs, iovs_len, nwritten) -> errno
 *
 * Print currently emits raw i64 bytes (8 bytes per value). This runner
 * interprets those bytes as little-endian i64 and formats them as decimal
 * strings for human-readable output.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use std::fmt;
use wasmtime::{Caller, Engine, Extern, Linker, Module, Store};

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, Clone)]
pub struct WasmRunError {
    pub message: String,
    pub context: String,
}

impl fmt::Display for WasmRunError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}", self.context, self.message)
    }
}

impl std::error::Error for WasmRunError {}

// =============================================================================
// Host State — Captures stdout from WASI fd_write
// =============================================================================

/// Holds state for the WASM execution, primarily the captured stdout buffer.
#[derive(Debug, Clone, Default)]
pub struct HostState {
    /// Raw bytes written to stdout via fd_write.
    pub stdout_raw: Vec<u8>,
}

// =============================================================================
// Output
// =============================================================================

/// Result of executing a WASM binary.
#[derive(Debug, Clone)]
pub struct WasmOutput {
    /// Raw bytes captured from stdout.
    pub stdout_raw: Vec<u8>,
    /// Formatted stdout: raw i64 bytes are converted to decimal strings.
    pub stdout: String,
}

impl WasmOutput {
    /// Convert raw stdout bytes to human-readable output.
    ///
    /// Since Ark's `print` now emits ASCII decimal text (itoa conversion
    /// happens in WASM), this is a simple UTF-8 decode passthrough.
    fn format_raw(raw: &[u8]) -> String {
        if raw.is_empty() {
            return String::new();
        }
        String::from_utf8_lossy(raw).into_owned()
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Run a compiled WASM binary's `_start` function and capture stdout.
///
/// This is the main entry point for executing Ark WASM programs.
pub fn run_wasm(bytes: &[u8]) -> Result<WasmOutput, WasmRunError> {
    let engine = Engine::default();
    let module = Module::from_binary(&engine, bytes).map_err(|e| WasmRunError {
        message: format!("Failed to load WASM module: {}", e),
        context: "run_wasm::load".to_string(),
    })?;

    let mut linker = Linker::<HostState>::new(&engine);
    link_wasi_fd_write(&mut linker)?;
    link_wasi_stubs(&mut linker)?;
    crate::wasm_host_imports::link_ark_host_imports(&mut linker)?;

    let mut store = Store::new(&engine, HostState::default());

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| WasmRunError {
            message: format!("Failed to instantiate: {}", e),
            context: "run_wasm::instantiate".to_string(),
        })?;

    // Call _start
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| WasmRunError {
            message: format!("No _start export: {}", e),
            context: "run_wasm::get_start".to_string(),
        })?;

    start.call(&mut store, ()).map_err(|e| WasmRunError {
        message: format!("Execution trapped: {}", e),
        context: "run_wasm::call_start".to_string(),
    })?;

    let raw = store.data().stdout_raw.clone();
    let stdout = WasmOutput::format_raw(&raw);

    Ok(WasmOutput {
        stdout_raw: raw,
        stdout,
    })
}

/// Call a specific exported function by name with i64 arguments.
///
/// Returns the i64 return value(s) from the function.
pub fn call_exported(bytes: &[u8], name: &str, args: &[i64]) -> Result<Option<i64>, WasmRunError> {
    let engine = Engine::default();
    let module = Module::from_binary(&engine, bytes).map_err(|e| WasmRunError {
        message: format!("Failed to load WASM module: {}", e),
        context: "call_exported::load".to_string(),
    })?;

    let mut linker = Linker::<HostState>::new(&engine);
    link_wasi_fd_write(&mut linker)?;
    link_wasi_stubs(&mut linker)?;
    crate::wasm_host_imports::link_ark_host_imports(&mut linker)?;

    let mut store = Store::new(&engine, HostState::default());

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| WasmRunError {
            message: format!("Failed to instantiate: {}", e),
            context: "call_exported::instantiate".to_string(),
        })?;

    let func = instance
        .get_func(&mut store, name)
        .ok_or_else(|| WasmRunError {
            message: format!("Export '{}' not found", name),
            context: "call_exported::get_func".to_string(),
        })?;

    // Build params and results arrays
    let params: Vec<wasmtime::Val> = args.iter().map(|&v| wasmtime::Val::I64(v)).collect();
    let func_type = func.ty(&store);
    let result_count = func_type.results().len();
    let mut results = vec![wasmtime::Val::I64(0); result_count];

    func.call(&mut store, &params, &mut results)
        .map_err(|e| WasmRunError {
            message: format!("Call to '{}' trapped: {}", name, e),
            context: "call_exported::call".to_string(),
        })?;

    // Extract first result as i64
    if let Some(val) = results.first() {
        match val {
            wasmtime::Val::I64(v) => Ok(Some(*v)),
            wasmtime::Val::I32(v) => Ok(Some(*v as i64)),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

// =============================================================================
// WASI fd_write Implementation
// =============================================================================

/// Link our lightweight fd_write implementation into the linker.
///
/// fd_write(fd: i32, iovs: i32, iovs_len: i32, nwritten_ptr: i32) -> i32
///
/// We only handle fd=1 (stdout). The iov structure is:
///   iov[i].buf_ptr: i32 at iovs + i*8
///   iov[i].buf_len: i32 at iovs + i*8 + 4
fn link_wasi_fd_write(linker: &mut Linker<HostState>) -> Result<(), WasmRunError> {
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "fd_write",
            |mut caller: Caller<'_, HostState>,
             fd: i32,
             iovs: i32,
             iovs_len: i32,
             nwritten_ptr: i32|
             -> i32 {
                // Only capture stdout (fd=1)
                if fd != 1 {
                    return 0; // silently ignore other fds
                }

                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return 8, // EBADF
                };

                let data = memory.data(&caller);
                let mut total_written: u32 = 0;

                // Ark only uses iovs_len=1, so we handle the first iov directly
                if iovs_len > 0 {
                    let iov_offset = iovs as usize;

                    // Read buf_ptr and buf_len from iovec
                    if iov_offset + 8 > data.len() {
                        return 21; // EFAULT
                    }

                    let buf_ptr =
                        u32::from_le_bytes(data[iov_offset..iov_offset + 4].try_into().expect("byte array conversion"))
                            as usize;
                    let buf_len = u32::from_le_bytes(
                        data[iov_offset + 4..iov_offset + 8].try_into().expect("byte array conversion"),
                    ) as usize;

                    if buf_ptr + buf_len > data.len() {
                        return 21; // EFAULT
                    }

                    let bytes = &data[buf_ptr..buf_ptr + buf_len];
                    let bytes_vec: Vec<u8> = bytes.to_vec();
                    total_written += buf_len as u32;

                    caller.data_mut().stdout_raw.extend_from_slice(&bytes_vec);
                }

                // Write nwritten
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return 8,
                };
                let data_mut = memory.data_mut(&mut caller);
                let nw_offset = nwritten_ptr as usize;
                if nw_offset + 4 <= data_mut.len() {
                    data_mut[nw_offset..nw_offset + 4]
                        .copy_from_slice(&total_written.to_le_bytes());
                }

                0 // success (errno = 0)
            },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link fd_write: {}", e),
            context: "link_wasi_fd_write".to_string(),
        })?;

    Ok(())
}

// =============================================================================
// WASI Stub Implementations (for Phase 11 WASI imports)
// =============================================================================

/// Link stub/no-op implementations for all WASI preview1 imports beyond fd_write.
///
/// These provide the minimal functions that wasmtime needs to instantiate modules
/// that declare Phase 11 WASI imports. Stubs return errno=0 (success) or
/// reasonable defaults.
fn link_wasi_stubs(linker: &mut Linker<HostState>) -> Result<(), WasmRunError> {
    // fd_read(fd:i32, iovs:i32, iovs_len:i32, nread:i32) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "fd_read",
            |_caller: Caller<'_, HostState>,
             _fd: i32,
             _iovs: i32,
             _iovs_len: i32,
             _nread: i32|
             -> i32 {
                0 // stub: success, 0 bytes read
            },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link fd_read: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // clock_time_get(clock_id:i32, precision:i64, timestamp_ptr:i32) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "clock_time_get",
            |mut caller: Caller<'_, HostState>,
             _clock_id: i32,
             _precision: i64,
             timestamp_ptr: i32|
             -> i32 {
                // Write a non-zero timestamp for testing (1_000_000_000 = 1 second)
                if let Some(Extern::Memory(memory)) = caller.get_export("memory") {
                    let data = memory.data_mut(&mut caller);
                    let offset = timestamp_ptr as usize;
                    if offset + 8 <= data.len() {
                        let ts: u64 = 1_000_000_000; // 1 second in nanoseconds
                        data[offset..offset + 8].copy_from_slice(&ts.to_le_bytes());
                    }
                }
                0
            },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link clock_time_get: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // random_get(buf:i32, buf_len:i32) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "random_get",
            |mut caller: Caller<'_, HostState>, buf: i32, buf_len: i32| -> i32 {
                // Fill with pseudo-random bytes (simple counter pattern)
                if let Some(Extern::Memory(memory)) = caller.get_export("memory") {
                    let data = memory.data_mut(&mut caller);
                    let offset = buf as usize;
                    let len = buf_len as usize;
                    if offset + len <= data.len() {
                        for i in 0..len {
                            data[offset + i] = (i % 256) as u8;
                        }
                    }
                }
                0
            },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link random_get: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // args_get(argv:i32, argv_buf:i32) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "args_get",
            |_caller: Caller<'_, HostState>, _argv: i32, _argv_buf: i32| -> i32 { 0 },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link args_get: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // args_sizes_get(argc:i32, argv_buf_size:i32) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "args_sizes_get",
            |mut caller: Caller<'_, HostState>, argc_ptr: i32, argv_buf_size_ptr: i32| -> i32 {
                if let Some(Extern::Memory(memory)) = caller.get_export("memory") {
                    let data = memory.data_mut(&mut caller);
                    let argc_off = argc_ptr as usize;
                    let buf_off = argv_buf_size_ptr as usize;
                    if argc_off + 4 <= data.len() {
                        data[argc_off..argc_off + 4].copy_from_slice(&0u32.to_le_bytes());
                        // 0 args
                    }
                    if buf_off + 4 <= data.len() {
                        data[buf_off..buf_off + 4].copy_from_slice(&0u32.to_le_bytes());
                        // 0 buf size
                    }
                }
                0
            },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link args_sizes_get: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // environ_get(environ:i32, environ_buf:i32) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "environ_get",
            |_caller: Caller<'_, HostState>, _environ: i32, _environ_buf: i32| -> i32 { 0 },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link environ_get: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // environ_sizes_get(environc:i32, environ_buf_size:i32) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "environ_sizes_get",
            |mut caller: Caller<'_, HostState>,
             environc_ptr: i32,
             environ_buf_size_ptr: i32|
             -> i32 {
                if let Some(Extern::Memory(memory)) = caller.get_export("memory") {
                    let data = memory.data_mut(&mut caller);
                    let ec_off = environc_ptr as usize;
                    let buf_off = environ_buf_size_ptr as usize;
                    if ec_off + 4 <= data.len() {
                        data[ec_off..ec_off + 4].copy_from_slice(&0u32.to_le_bytes());
                    }
                    if buf_off + 4 <= data.len() {
                        data[buf_off..buf_off + 4].copy_from_slice(&0u32.to_le_bytes());
                    }
                }
                0
            },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link environ_sizes_get: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // proc_exit(code:i32)
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "proc_exit",
            |_caller: Caller<'_, HostState>, _code: i32| {
                // In tests, proc_exit is a no-op
            },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link proc_exit: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // path_open(fd, dirflags, path, path_len, oflags, rights_base, rights_inherit, fdflags, opened_fd) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "path_open",
            |_caller: Caller<'_, HostState>,
             _fd: i32,
             _dirflags: i32,
             _path: i32,
             _path_len: i32,
             _oflags: i32,
             _rights_base: i64,
             _rights_inherit: i64,
             _fdflags: i32,
             _opened_fd: i32|
             -> i32 {
                76 // ENOSYS — not supported in test environment
            },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link path_open: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    // fd_close(fd:i32) -> i32
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "fd_close",
            |_caller: Caller<'_, HostState>, _fd: i32| -> i32 { 0 },
        )
        .map_err(|e| WasmRunError {
            message: format!("Failed to link fd_close: {}", e),
            context: "link_wasi_stubs".to_string(),
        })?;

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::wasm_codegen::WasmCodegen;

    /// Helper: compile an Ark source string to WASM bytes.
    fn compile_ark(source: &str) -> Vec<u8> {
        let ast = parser::parse_source(source, "test.ark").expect("parse failed");
        WasmCodegen::compile_to_bytes(&ast).expect("compile failed")
    }

    #[test]
    fn test_run_hello_wasm() {
        // hello_wasm.ark: x := 10 + 20; y := x * 2; print(y)
        // Expected: y = 60, printed as ASCII decimal text "60\n"
        let wasm = compile_ark("x := 10 + 20\ny := x * 2\nprint(y)");
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("60"),
            "stdout should contain '60', got: {}",
            output.stdout
        );
    }

    #[test]
    fn test_run_simple_arithmetic() {
        let wasm = compile_ark("print(7 + 3)");
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("10"),
            "stdout should contain '10', got: {}",
            output.stdout
        );
    }

    #[test]
    fn test_call_exported_function() {
        // Define a function and call it
        let source = "func add(a, b) { return a + b }\nprint(add(1, 2))";
        let wasm = compile_ark(source);
        // Call the exported 'add' function directly
        let result = call_exported(&wasm, "add", &[10, 20]).expect("call failed");
        assert_eq!(result, Some(30));
    }

    #[test]
    fn test_run_factorial() {
        let source = r#"
func factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
print(factorial(5))
"#;
        let wasm = compile_ark(source);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("120"),
            "stdout should contain '120' (5!), got: {}",
            output.stdout
        );
    }

    #[test]
    fn test_call_factorial_directly() {
        let source = r#"
func factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
print(0)
"#;
        let wasm = compile_ark(source);
        let result = call_exported(&wasm, "factorial", &[6]).expect("call failed");
        assert_eq!(result, Some(720), "6! should be 720");
    }

    #[test]
    fn test_format_raw_ascii_passthrough() {
        // format_raw is now a simple UTF-8 passthrough
        let raw = b"42\n-7\n0\n";
        let formatted = WasmOutput::format_raw(raw);
        assert_eq!(formatted, "42\n-7\n0\n");
    }

    #[test]
    fn test_format_raw_empty() {
        assert_eq!(WasmOutput::format_raw(&[]), "");
    }

    #[test]
    fn test_invalid_wasm() {
        let result = run_wasm(&[0, 1, 2, 3]);
        assert!(result.is_err(), "invalid WASM should error");
    }

    #[test]
    fn test_missing_export() {
        let wasm = compile_ark("print(1)");
        let result = call_exported(&wasm, "nonexistent", &[]);
        assert!(result.is_err(), "missing export should error");
    }

    // =========================================================================
    // Phase 15: End-to-End Pipeline Tests
    // =========================================================================

    #[test]
    fn test_e2e_print_string() {
        // print("hello") should output "hello\n"
        let wasm = compile_ark(r#"print("hello")"#);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("hello"),
            "stdout should contain 'hello', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_print_integer() {
        // Explicit test: print(42) should output "42"
        let wasm = compile_ark("print(42)");
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("42"),
            "stdout should contain '42', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_if_else_true() {
        // Test conditional: if 1 == 1, print "yes"
        let source = r#"
if 1 == 1 {
    print(1)
} else {
    print(0)
}
"#;
        let wasm = compile_ark(source);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("1"),
            "if-else true branch should print '1', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_let_variable() {
        // Test variable binding and use
        let source = r#"
let x := 5
let y := 7
print(x + y)
"#;
        let wasm = compile_ark(source);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("12"),
            "let-variable should produce '12', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_multi_print() {
        // Multiple print statements should all appear in stdout
        let source = r#"
print(1)
print(2)
print(3)
"#;
        let wasm = compile_ark(source);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("1"),
            "should contain '1', got: {:?}",
            output.stdout
        );
        assert!(
            output.stdout.contains("2"),
            "should contain '2', got: {:?}",
            output.stdout
        );
        assert!(
            output.stdout.contains("3"),
            "should contain '3', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_nested_arithmetic() {
        // Nested expression: (3 + 4) * 6 = 42
        let source = "print((3 + 4) * 6)";
        let wasm = compile_ark(source);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("42"),
            "nested arithmetic should produce '42', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_function_with_return() {
        // User-defined function with explicit return
        let source = r#"
func triple(n) {
    return n * 3
}
print(triple(14))
"#;
        let wasm = compile_ark(source);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("42"),
            "triple(14) should produce '42', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_while_loop() {
        // While loop that counts down and prints
        let source = r#"
let i := 3
while i > 0 {
    print(i)
    i := i - 1
}
"#;
        let wasm = compile_ark(source);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("3"),
            "while loop should print '3', got: {:?}",
            output.stdout
        );
        assert!(
            output.stdout.contains("2"),
            "while loop should print '2', got: {:?}",
            output.stdout
        );
        assert!(
            output.stdout.contains("1"),
            "while loop should print '1', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_boolean_logic() {
        // Test boolean expression: 10 > 5 should print 1 (true)
        let source = r#"
if 10 > 5 {
    print(99)
} else {
    print(0)
}
"#;
        let wasm = compile_ark(source);
        let output = run_wasm(&wasm).expect("run failed");
        assert!(
            output.stdout.contains("99"),
            "boolean logic should print '99', got: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_e2e_call_exported_add() {
        // Test call_exported with two user-defined functions
        let source = r#"
func square(x) {
    return x * x
}
print(0)
"#;
        let wasm = compile_ark(source);
        let result = call_exported(&wasm, "square", &[7]).expect("call failed");
        assert_eq!(result, Some(49), "square(7) should return 49");
    }
}
