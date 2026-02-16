/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use crate::runtime::{NativeFn, RuntimeError, Scope, Value};
#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;
#[cfg(not(target_arch = "wasm32"))]
use shell_words;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread;
use std::io::{self, Write};
use regex::Regex;

#[cfg(not(target_arch = "wasm32"))]
static AI_CLIENT: OnceLock<Client> = OnceLock::new();

pub struct IntrinsicRegistry;

impl IntrinsicRegistry {
    pub fn resolve(hash: &str) -> Option<NativeFn> {
        match hash {
            "intrinsic_add" => Some(intrinsic_add),
            "intrinsic_sub" => Some(intrinsic_sub),
            "intrinsic_mul" => Some(intrinsic_mul),
            "intrinsic_div" => Some(intrinsic_div),
            "intrinsic_mod" => Some(intrinsic_mod),
            "intrinsic_gt" => Some(intrinsic_gt),
            "intrinsic_lt" => Some(intrinsic_lt),
            "intrinsic_ge" => Some(intrinsic_ge),
            "intrinsic_le" => Some(intrinsic_le),
            "intrinsic_eq" => Some(intrinsic_eq),
            "intrinsic_and" => Some(intrinsic_and),
            "intrinsic_or" => Some(intrinsic_or),
            "intrinsic_not" => Some(intrinsic_not),
            "intrinsic_print" => Some(intrinsic_print),
            "print" => Some(intrinsic_print),
            "intrinsic_ask_ai" => Some(intrinsic_ask_ai),
            "sys_exec" | "intrinsic_exec" => Some(intrinsic_exec),
            "sys_fs_write" | "intrinsic_fs_write" | "sys.fs.write" => Some(intrinsic_fs_write),
            "sys_fs_read" | "intrinsic_fs_read" | "sys.fs.read" => Some(intrinsic_fs_read),
            "intrinsic_crypto_hash" | "sys.crypto.hash" => Some(intrinsic_crypto_hash),
            "intrinsic_crypto_verify" | "sys.crypto.verify" => Some(intrinsic_crypto_verify),
            "intrinsic_merkle_root" | "sys.crypto.merkle_root" => Some(intrinsic_merkle_root),
            "intrinsic_buffer_alloc" | "sys.mem.alloc" => Some(intrinsic_buffer_alloc),
            "intrinsic_buffer_inspect" | "sys.mem.inspect" => Some(intrinsic_buffer_inspect),
            "intrinsic_buffer_read" | "sys.mem.read" => Some(intrinsic_buffer_read),
            "intrinsic_buffer_write" | "sys.mem.write" => Some(intrinsic_buffer_write),
            "intrinsic_list_get" | "sys.list.get" | "sys.str.get" => Some(intrinsic_list_get),
            "intrinsic_list_append" | "sys.list.append" => Some(intrinsic_list_append),
            "intrinsic_list_pop" | "sys.list.pop" => Some(intrinsic_list_pop),
            "intrinsic_len" | "sys.len" => Some(intrinsic_len),
            "intrinsic_struct_get" | "sys.struct.get" => Some(intrinsic_struct_get),
            "intrinsic_struct_set" | "sys.struct.set" => Some(intrinsic_struct_set),
            "intrinsic_time_now" | "time.now" => Some(intrinsic_time_now),
            "intrinsic_math_pow" | "math.pow" => Some(intrinsic_math_pow),
            "intrinsic_math_sqrt" | "math.sqrt" => Some(intrinsic_math_sqrt),
            "intrinsic_math_sin" | "math.sin" => Some(intrinsic_math_sin),
            "intrinsic_math_cos" | "math.cos" => Some(intrinsic_math_cos),
            "intrinsic_math_tan" | "math.tan" => Some(intrinsic_math_tan),
            "intrinsic_math_asin" | "math.asin" => Some(intrinsic_math_asin),
            "intrinsic_math_acos" | "math.acos" => Some(intrinsic_math_acos),
            "intrinsic_math_atan" | "math.atan" => Some(intrinsic_math_atan),
            "intrinsic_math_atan2" | "math.atan2" => Some(intrinsic_math_atan2),
            "intrinsic_io_cls" | "io.cls" => Some(intrinsic_io_cls),
            "intrinsic_list_set" | "sys.list.set" => Some(intrinsic_list_set),
            "intrinsic_chain_height" | "sys.chain.height" => Some(intrinsic_chain_height),
            "intrinsic_chain_get_balance" | "sys.chain.get_balance" => {
                Some(intrinsic_chain_get_balance)
            }
            "intrinsic_chain_submit_tx" | "sys.chain.submit_tx" => Some(intrinsic_chain_submit_tx),
            "intrinsic_chain_verify_tx" | "sys.chain.verify_tx" => Some(intrinsic_chain_verify_tx),
            "sys.fs.write_buffer" => Some(intrinsic_fs_write_buffer),
            "sys.fs.read_buffer" => Some(intrinsic_fs_read_buffer),
            "math.sin_scaled" => Some(intrinsic_math_sin_scaled),
            "math.cos_scaled" => Some(intrinsic_math_cos_scaled),
            "math.pi_scaled" => Some(intrinsic_math_pi_scaled),
            "sys.str.from_code" => Some(intrinsic_str_from_code),
            "sys.time.sleep" | "intrinsic_time_sleep" => Some(intrinsic_time_sleep),
            "sys.io.read_bytes" | "intrinsic_io_read_bytes" => Some(intrinsic_io_read_bytes),
            "sys.io.read_line" | "intrinsic_io_read_line" => Some(intrinsic_io_read_line),
            "sys.io.write" | "intrinsic_io_write" => Some(intrinsic_io_write),
            "sys.io.read_file_async" | "intrinsic_io_read_file_async" => {
                Some(intrinsic_io_read_file_async)
            }
            "sys.extract_code" | "intrinsic_extract_code" => Some(intrinsic_extract_code),
            _ => None,
        }
    }

    pub fn register_all(scope: &mut Scope) {
        scope.set(
            "intrinsic_add".to_string(),
            Value::NativeFunction(intrinsic_add),
        );
        scope.set(
            "intrinsic_sub".to_string(),
            Value::NativeFunction(intrinsic_sub),
        );
        scope.set(
            "intrinsic_mul".to_string(),
            Value::NativeFunction(intrinsic_mul),
        );
        scope.set(
            "intrinsic_div".to_string(),
            Value::NativeFunction(intrinsic_div),
        );
        scope.set(
            "intrinsic_mod".to_string(),
            Value::NativeFunction(intrinsic_mod),
        );
        scope.set(
            "intrinsic_gt".to_string(),
            Value::NativeFunction(intrinsic_gt),
        );
        scope.set(
            "intrinsic_lt".to_string(),
            Value::NativeFunction(intrinsic_lt),
        );
        scope.set(
            "intrinsic_ge".to_string(),
            Value::NativeFunction(intrinsic_ge),
        );
        scope.set(
            "intrinsic_le".to_string(),
            Value::NativeFunction(intrinsic_le),
        );
        scope.set(
            "intrinsic_eq".to_string(),
            Value::NativeFunction(intrinsic_eq),
        );
        scope.set(
            "intrinsic_and".to_string(),
            Value::NativeFunction(intrinsic_and),
        );
        scope.set(
            "intrinsic_or".to_string(),
            Value::NativeFunction(intrinsic_or),
        );
        scope.set(
            "intrinsic_not".to_string(),
            Value::NativeFunction(intrinsic_not),
        );
        scope.set(
            "intrinsic_print".to_string(),
            Value::NativeFunction(intrinsic_print),
        );
        scope.set("print".to_string(), Value::NativeFunction(intrinsic_print)); // Alias
        scope.set(
            "intrinsic_ask_ai".to_string(),
            Value::NativeFunction(intrinsic_ask_ai),
        );

        // System
        scope.set("sys.len".to_string(), Value::NativeFunction(intrinsic_len));
        scope.set(
            "sys.exec".to_string(),
            Value::NativeFunction(intrinsic_exec),
        );
        scope.set(
            "sys.fs.write".to_string(),
            Value::NativeFunction(intrinsic_fs_write),
        );
        scope.set(
            "sys.fs.read".to_string(),
            Value::NativeFunction(intrinsic_fs_read),
        );
        scope.set(
            "sys.crypto.hash".to_string(),
            Value::NativeFunction(intrinsic_crypto_hash),
        );
        scope.set(
            "sys.crypto.verify".to_string(),
            Value::NativeFunction(intrinsic_crypto_verify),
        );

        // List/Struct
        scope.set(
            "sys.list.get".to_string(),
            Value::NativeFunction(intrinsic_list_get),
        );
        scope.set(
            "sys.list.append".to_string(),
            Value::NativeFunction(intrinsic_list_append),
        );
        scope.set(
            "intrinsic_list_pop".to_string(),
            Value::NativeFunction(intrinsic_list_pop),
        );
        scope.set(
            "sys.struct.get".to_string(),
            Value::NativeFunction(intrinsic_struct_get),
        );
        scope.set(
            "sys.struct.set".to_string(),
            Value::NativeFunction(intrinsic_struct_set),
        );
        scope.set(
            "time.now".to_string(),
            Value::NativeFunction(intrinsic_time_now),
        );
        scope.set(
            "intrinsic_math_pow".to_string(),
            Value::NativeFunction(intrinsic_math_pow),
        );
        scope.set(
            "math.pow".to_string(),
            Value::NativeFunction(intrinsic_math_pow),
        );
        scope.set(
            "intrinsic_math_sqrt".to_string(),
            Value::NativeFunction(intrinsic_math_sqrt),
        );
        scope.set(
            "math.sqrt".to_string(),
            Value::NativeFunction(intrinsic_math_sqrt),
        );
        scope.set(
            "intrinsic_math_sin".to_string(),
            Value::NativeFunction(intrinsic_math_sin),
        );
        scope.set(
            "math.sin".to_string(),
            Value::NativeFunction(intrinsic_math_sin),
        );
        scope.set(
            "intrinsic_math_cos".to_string(),
            Value::NativeFunction(intrinsic_math_cos),
        );
        scope.set(
            "math.cos".to_string(),
            Value::NativeFunction(intrinsic_math_cos),
        );
        scope.set(
            "intrinsic_math_tan".to_string(),
            Value::NativeFunction(intrinsic_math_tan),
        );
        scope.set(
            "math.tan".to_string(),
            Value::NativeFunction(intrinsic_math_tan),
        );
        scope.set(
            "intrinsic_math_asin".to_string(),
            Value::NativeFunction(intrinsic_math_asin),
        );
        scope.set(
            "math.asin".to_string(),
            Value::NativeFunction(intrinsic_math_asin),
        );
        scope.set(
            "intrinsic_math_acos".to_string(),
            Value::NativeFunction(intrinsic_math_acos),
        );
        scope.set(
            "math.acos".to_string(),
            Value::NativeFunction(intrinsic_math_acos),
        );
        scope.set(
            "intrinsic_math_atan".to_string(),
            Value::NativeFunction(intrinsic_math_atan),
        );
        scope.set(
            "math.atan".to_string(),
            Value::NativeFunction(intrinsic_math_atan),
        );
        scope.set(
            "intrinsic_math_atan2".to_string(),
            Value::NativeFunction(intrinsic_math_atan2),
        );
        scope.set(
            "math.atan2".to_string(),
            Value::NativeFunction(intrinsic_math_atan2),
        );
        scope.set(
            "io.cls".to_string(),
            Value::NativeFunction(intrinsic_io_cls),
        );
        scope.set(
            "sys.list.set".to_string(),
            Value::NativeFunction(intrinsic_list_set),
        );
        scope.set(
            "sys.chain.height".to_string(),
            Value::NativeFunction(intrinsic_chain_height),
        );
        scope.set(
            "sys.chain.get_balance".to_string(),
            Value::NativeFunction(intrinsic_chain_get_balance),
        );
        scope.set(
            "sys.chain.submit_tx".to_string(),
            Value::NativeFunction(intrinsic_chain_submit_tx),
        );
        scope.set(
            "sys.chain.verify_tx".to_string(),
            Value::NativeFunction(intrinsic_chain_verify_tx),
        );
        scope.set(
            "sys.fs.write_buffer".to_string(),
            Value::NativeFunction(intrinsic_fs_write_buffer),
        );
        scope.set(
            "sys.fs.read_buffer".to_string(),
            Value::NativeFunction(intrinsic_fs_read_buffer),
        );
        scope.set(
            "math.sin_scaled".to_string(),
            Value::NativeFunction(intrinsic_math_sin_scaled),
        );
        scope.set(
            "math.cos_scaled".to_string(),
            Value::NativeFunction(intrinsic_math_cos_scaled),
        );
        scope.set(
            "sys.fs.write".to_string(),
            Value::NativeFunction(intrinsic_fs_write),
        );
        scope.set(
            "sys.time.sleep".to_string(),
            Value::NativeFunction(intrinsic_time_sleep),
        );
        scope.set(
            "sys.io.read_bytes".to_string(),
            Value::NativeFunction(intrinsic_io_read_bytes),
        );
        scope.set(
            "sys.io.read_line".to_string(),
            Value::NativeFunction(intrinsic_io_read_line),
        );
        scope.set(
            "sys.io.write".to_string(),
            Value::NativeFunction(intrinsic_io_write),
        );
        scope.set(
            "sys.io.read_file_async".to_string(),
            Value::NativeFunction(intrinsic_io_read_file_async),
        );
        scope.set(
            "sys.extract_code".to_string(),
            Value::NativeFunction(intrinsic_extract_code),
        );
        /*
        scope.set(
            "sys.net.http.request".to_string(),
            Value::NativeFunction(intrinsic_http_request),
        );
        // Audio Intrinsics (PR-69)
        scope.set(
            "sys.audio.play_wav".to_string(),
            Value::NativeFunction(intrinsic_audio_play_wav),
        );
        scope.set(
            "sys.audio.synth_tone".to_string(),
            Value::NativeFunction(intrinsic_audio_synth_tone),
        );
        */
    }
}

fn check_path_security(path: &str) -> Result<(), RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Ok(());

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::env;
        use std::path::Path;

        let cwd = env::current_dir().map_err(|_| RuntimeError::NotExecutable)?;
        let path_obj = Path::new(path);

        // Construct absolute path
        let abs_path = if path_obj.is_absolute() {
            path_obj.to_path_buf()
        } else {
            cwd.join(path_obj)
        };

        // To handle both read and write (where file might not exist),
        // we check if the path or its parent exists and is within CWD.
        // If neither exists, we can't write anyway (fs::write doesn't mkdir -p).

        let path_to_check = if abs_path.exists() {
            abs_path
        } else {
            match abs_path.parent() {
                Some(p) => p.to_path_buf(),
                None => return Err(RuntimeError::NotExecutable),
            }
        };

        // If parent doesn't exist, canonicalize fails.
        let canonical_path =
            std::fs::canonicalize(&path_to_check).map_err(|_| RuntimeError::NotExecutable)?;

        if !canonical_path.starts_with(&cwd) {
            println!(
                "[Ark:Sandbox] Access Denied: Path '{}' resolves outside CWD.",
                path
            );
            return Err(RuntimeError::NotExecutable);
        }

        Ok(())
    }
}

pub fn intrinsic_ask_ai(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }

    let prompt = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        return Ok(Value::String(
            "[Ark:AI] Unavailable in Browser Runtime (OIS: Low)".to_string(),
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let api_key = std::env::var("GOOGLE_API_KEY").map_err(|_| {
            println!("[Ark:AI] Error: GOOGLE_API_KEY not set.");
            RuntimeError::NotExecutable
        })?;

        let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent".to_string();

        // Optimization: Reuse Client (Connection Pool)
        let client = AI_CLIENT.get_or_init(|| {
            Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new())
        });

        let payload = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        });

        println!("[Ark:AI] Contacting Gemini (Native Rust)...");

        // Execute async logic within blocking context
        // SAFETY: This blocks the current thread. Do not call this from within an existing
        // Tokio runtime context (e.g., inside an async function running on a runtime)
        // or it will panic. The VM is designed to run in a dedicated thread or process.
        // Create a new runtime for this blocking operation
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Simple Retry Logic
            for attempt in 0..3 {
                match client
                    .post(&url)
                    .header("x-goog-api-key", &api_key)
                    .json(&payload)
                    .send()
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            let json_resp = match resp.json::<serde_json::Value>() {
                                Ok(v) => v,
                                Err(e) => {
                                    println!("[Ark:AI] JSON Error: {}", e);
                                    return Err(RuntimeError::NotExecutable);
                                }
                            };

                            if let Some(text) =
                                json_resp["candidates"][0]["content"]["parts"][0]["text"].as_str()
                            {
                                return Ok(Value::String(text.to_string()));
                            }
                        } else if resp.status().as_u16() == 429 {
                            println!("[Ark:AI] Rate limit (429). Retrying...");
                            tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                            continue;
                        } else {
                            println!("[Ark:AI] HTTP Error: {}", resp.status());
                        }
                    }
                    Err(e) => println!("[Ark:AI] Network Error: {}", e),
                }
            }

            // Fallback Mock
            println!("[Ark:AI] WARNING: API Failed. Using Fallback Mock.");
            let start = "```python\n";
            let code =
                "import datetime\nprint(f'Sovereignty Established: {datetime.datetime.now()}')\n";
            let end = "```";
            Ok(Value::String(format!("{}{}{}", start, code, end)))
        })
    }
}

pub fn intrinsic_exec(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }

    // Support both String (legacy, parsed) and List (secure, explicit)
    let (program, args_list) = match &args[0] {
        Value::String(s) => {
            eprintln!(
                "[Ark:Exec] WARNING: usage of sys.exec(String) is deprecated for security. Use sys.exec([cmd, arg1, ...])"
            );
            #[cfg(not(target_arch = "wasm32"))]
            {
                let parts = shell_words::split(s).map_err(|_| RuntimeError::NotExecutable)?;
                if parts.is_empty() {
                    return Err(RuntimeError::NotExecutable);
                }
                (parts[0].clone(), parts[1..].to_vec())
            }
            #[cfg(target_arch = "wasm32")]
            {
                (s.clone(), vec![])
            }
        }
        Value::List(l) => {
            let mut parts = Vec::new();
            for item in l {
                if let Value::String(s) = item {
                    parts.push(s.clone());
                } else {
                    return Err(RuntimeError::TypeMismatch(
                        "String".to_string(),
                        item.clone(),
                    ));
                }
            }
            if parts.is_empty() {
                return Err(RuntimeError::NotExecutable);
            }
            (parts[0].clone(), parts[1..].to_vec())
        }
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or List".to_string(),
                args[0].clone(),
            ));
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        println!("[Ark:WASM] Security Block: sys.exec('{}') denied.", program);
        return Err(RuntimeError::NotExecutable);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("[Ark:Exec] {} {:?}", program, args_list);

        let mut cmd = Command::new(&program);
        cmd.args(&args_list);

        let output = cmd.output().map_err(|_| RuntimeError::NotExecutable)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(Value::String(stdout))
    }
}

pub fn intrinsic_fs_write(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };
    let content = match &args[1] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[1].clone(),
            ));
        }
    };

    check_path_security(path_str)?;

    #[cfg(target_arch = "wasm32")]
    {
        println!(
            "[Ark:VFS] Write to '{}': (Simulated) [Content Size: {}]",
            path_str,
            content.len()
        );
        Ok(Value::Unit)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // NTS Protocol: Intentional Friction (Level 1)
        if std::path::Path::new(path_str).exists() {
            println!(
                "[Ark:NTS] WARNING: Overwriting existing file '{}' without explicit lock (LAT).",
                path_str
            );
        }

        println!("[Ark:FS] Writing to {}", path_str);
        fs::write(path_str, content).map_err(|_| RuntimeError::NotExecutable)?;
        Ok(Value::Unit)
    }
}

pub fn intrinsic_add(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }

    let mut iter = args.into_iter();
    let left = iter.next().unwrap();
    let right = iter.next().unwrap();

    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
        (Value::String(mut a), Value::String(b)) => {
            a.push_str(&b);
            Ok(Value::String(a))
        }
        (Value::String(mut a), Value::Integer(b)) => {
            a.push_str(&b.to_string());
            Ok(Value::String(a))
        }
        (Value::Integer(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (Value::String(mut a), Value::Boolean(b)) => {
            a.push_str(&b.to_string());
            Ok(Value::String(a))
        }
        (Value::Boolean(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (l, _) => Err(RuntimeError::TypeMismatch(
            "Integer, String, or Boolean".to_string(),
            l,
        )),
    }
}

pub fn intrinsic_sub(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_mul(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_div(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => {
            if *b == 0 {
                return Err(RuntimeError::NotExecutable); // Div by zero
            }
            Ok(Value::Integer(a / b))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_mod(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => {
            if *b == 0 {
                return Err(RuntimeError::NotExecutable); // Mod by zero
            }
            Ok(Value::Integer(a % b))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_gt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a > b { 1 } else { 0 })),
        (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a > b { 1 } else { 0 })),
        (Value::String(a), Value::Integer(b)) => {
            Ok(Value::Integer(if a > &b.to_string() { 1 } else { 0 }))
        }
        (Value::Integer(a), Value::String(b)) => {
            Ok(Value::Integer(if &a.to_string() > b { 1 } else { 0 }))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_lt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
        (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a < b)),
        _ => Err(RuntimeError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_not(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Boolean(b) => Ok(Value::Boolean(!b)),
        _ => Err(RuntimeError::TypeMismatch(
            "Boolean".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_ge(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a >= b { 1 } else { 0 })),
        (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a >= b { 1 } else { 0 })),
        (Value::String(a), Value::Integer(b)) => {
            Ok(Value::Integer(if a >= &b.to_string() { 1 } else { 0 }))
        }
        (Value::Integer(a), Value::String(b)) => {
            Ok(Value::Integer(if &a.to_string() >= b { 1 } else { 0 }))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_le(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a <= b { 1 } else { 0 })),
        (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a <= b { 1 } else { 0 })),
        (Value::String(a), Value::Integer(b)) => {
            Ok(Value::Integer(if a <= &b.to_string() { 1 } else { 0 }))
        }
        (Value::Integer(a), Value::String(b)) => {
            Ok(Value::Integer(if &a.to_string() <= b { 1 } else { 0 }))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_eq(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a == b { 1 } else { 0 })),
        (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a == b { 1 } else { 0 })),
        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Integer(if a == b { 1 } else { 0 })),
        _ => Ok(Value::Integer(0)), // Default inequality for mismatched types/objects
    }
}

pub fn intrinsic_and(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    // Truthy check: Integer != 0, Boolean == true, String != "0" and != ""
    let is_truthy = |v: &Value| match v {
        Value::Integer(n) => *n != 0,
        Value::Boolean(b) => *b,
        Value::String(s) => s != "0" && !s.is_empty() && s != "false",
        _ => false,
    };

    let left = is_truthy(&args[0]);
    let right = is_truthy(&args[1]);

    Ok(Value::Boolean(left && right))
}

pub fn intrinsic_or(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let is_truthy = |v: &Value| match v {
        Value::Integer(n) => *n != 0,
        Value::Boolean(b) => *b,
        Value::String(s) => s != "0" && !s.is_empty() && s != "false",
        _ => false,
    };

    let left = is_truthy(&args[0]);
    let right = is_truthy(&args[1]);

    Ok(Value::Boolean(left || right))
}

pub fn intrinsic_print(args: Vec<Value>) -> Result<Value, RuntimeError> {
    for arg in args {
        print_value(&arg);
    }
    println!(); // Newline at the end
    Ok(Value::Unit)
}

fn print_value(v: &Value) {
    match v {
        Value::Integer(i) => print!("{}", i),
        Value::String(s) => print!("{}", s),
        Value::Boolean(b) => print!("{}", b),
        Value::Unit => print!("unit"),
        Value::LinearObject { id, .. } => print!("<LinearObject:{}>", id),
        Value::Function(_) => print!("<Function>"),
        Value::NativeFunction(_) => print!("<NativeFunction>"),
        Value::List(l) => {
            print!("[");
            for (i, item) in l.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print_value(item);
            }
            print!("]");
        }
        Value::Buffer(b) => print!("<Buffer: len={}, ptr={:p}>", b.len(), b.as_ptr()),
        Value::Struct(fields) => {
            print!("{{");
            for (i, (k, v)) in fields.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("{}: ", k);
                print_value(v);
            }
            print!("}}");
        }
        Value::Return(val) => print_value(val),
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn validate_safe_path(path_str: &str) -> Result<PathBuf, RuntimeError> {
    let path = Path::new(path_str);

    // 1. Canonicalize the requested path (resolves symlinks and ..)
    // If the file does not exist, canonicalize fails. For read, this is fine (file must exist).
    let canonical_path = fs::canonicalize(path).map_err(|_| RuntimeError::NotExecutable)?;

    // 2. Canonicalize the current working directory (sandbox root)
    let current_dir = env::current_dir().map_err(|_| RuntimeError::NotExecutable)?;
    let canonical_cwd = fs::canonicalize(current_dir).map_err(|_| RuntimeError::NotExecutable)?;

    // 3. Verify that the requested path starts with the sandbox root
    if canonical_path.starts_with(&canonical_cwd) {
        Ok(canonical_path)
    } else {
        println!(
            "[Ark:FS] Security Violation: Path traversal attempt to '{}'",
            path_str
        );
        Err(RuntimeError::NotExecutable)
    }
}

pub fn intrinsic_fs_read(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    check_path_security(path_str)?;

    #[cfg(target_arch = "wasm32")]
    {
        println!("[Ark:VFS] Read from '{}': (Simulated) [Empty]", path_str);
        Ok(Value::String("".to_string()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("[Ark:FS] Reading from {}", path_str);
        // Security: Path Traversal Check
        let safe_path = validate_safe_path(path_str)?;
        let content = fs::read_to_string(safe_path).map_err(|_| RuntimeError::NotExecutable)?;
        Ok(Value::String(content))
    }
}

pub fn intrinsic_crypto_hash(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let data_bytes = match &args[0] {
        Value::String(s) => s.as_bytes(),
        Value::Buffer(b) => b.as_slice(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer".to_string(),
                args[0].clone(),
            ));
        }
    };

    Ok(Value::String(crate::crypto::hash(data_bytes)))
}

pub fn intrinsic_crypto_verify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    // Helper to get bytes from Buffer or Hex String
    let get_bytes = |v: &Value, name: &str| -> Result<Vec<u8>, RuntimeError> {
        match v {
            Value::Buffer(b) => Ok(b.clone()),
            Value::String(s) => hex::decode(s).map_err(|_| {
                RuntimeError::TypeMismatch(format!("Hex String for {}", name), v.clone())
            }),
            _ => Err(RuntimeError::TypeMismatch(
                format!("Buffer or Hex String for {}", name),
                v.clone(),
            )),
        }
    };

    let msg_bytes = match &args[0] {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Buffer(b) => b.clone(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer for msg".to_string(),
                args[0].clone(),
            ));
        }
    };

    let sig_bytes = get_bytes(&args[1], "signature")?;
    let pubkey_bytes = get_bytes(&args[2], "public key")?;

    match crate::crypto::verify_signature(&msg_bytes, &sig_bytes, &pubkey_bytes) {
        Ok(valid) => Ok(Value::Boolean(valid)),
        Err(e) => Err(RuntimeError::InvalidOperation(e)),
    }
}

pub fn intrinsic_merkle_root(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }

    let list = match &args[0] {
        Value::List(l) => l,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "List".to_string(),
                args[0].clone(),
            ));
        }
    };

    // Extract strings from list
    let mut leaves: Vec<String> = Vec::new();
    for item in list {
        match item {
            Value::String(s) => leaves.push(s.clone()),
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "String inside List".to_string(),
                    item.clone(),
                ));
            }
        }
    }

    Ok(Value::String(crate::crypto::merkle_root(&leaves)))
}

pub fn intrinsic_buffer_alloc(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let size = match &args[0] {
        Value::Integer(n) => *n as usize,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };
    let buf = vec![0u8; size];
    Ok(Value::Buffer(buf))
}

pub fn intrinsic_buffer_inspect(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match args.into_iter().next().unwrap() {
        Value::Buffer(b) => {
            let ptr = b.as_ptr();
            println!("<Buffer Inspect: ptr={:p}, len={}>", ptr, b.len());
            Ok(Value::Buffer(b))
        }
        v => Err(RuntimeError::TypeMismatch("Buffer".to_string(), v)),
    }
}

pub fn intrinsic_buffer_read(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // args: [buffer, index]
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let index_val = args.pop().unwrap();
    let buf_val = args.pop().unwrap();

    let index = match index_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), index_val)),
    };

    match buf_val {
        Value::Buffer(b) => {
            if index < 0 || index >= b.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            let val = b[index as usize] as i64;
            let list = vec![Value::Integer(val), Value::Buffer(b)];
            Ok(Value::List(list))
        }
        v => Err(RuntimeError::TypeMismatch("Buffer".to_string(), v)),
    }
}

pub fn intrinsic_buffer_write(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Linear Semantics: buf := sys.mem.write(buf, i, v)
    // Consumes the buffer (linear ownership), modifies it in-place, and returns the modified buffer.
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    // We need to destructure args to get ownership of Buffer
    // args is Vec<Value>.
    let mut args = args; // Allow move
    let val_to_write = args.pop().unwrap(); // value
    let idx_val = args.pop().unwrap(); // index
    let buf_val = args.pop().unwrap(); // buffer

    let index = match idx_val {
        Value::Integer(n) => n as usize,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), idx_val)),
    };

    let byte_val = match val_to_write {
        Value::Integer(n) => n as u8,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                val_to_write,
            ));
        }
    };

    match buf_val {
        Value::Buffer(mut b) => {
            if index >= b.len() {
                return Err(RuntimeError::NotExecutable);
            }
            b[index] = byte_val;
            Ok(Value::Buffer(b)) // Return modified buffer (Linear Threading)
        }
        _ => Err(RuntimeError::TypeMismatch("Buffer".to_string(), buf_val)),
    }
}

pub fn intrinsic_list_get(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let index_val = args.pop().unwrap();
    let list_val = args.pop().unwrap();

    let index = match index_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), index_val)),
    };

    match list_val {
        Value::List(list) => {
            if index < 0 || index >= list.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            let val = list[index as usize].clone();
            let new_list_val = Value::List(list);

            Ok(Value::List(vec![val, new_list_val]))
        }
        Value::String(s) => {
            if index < 0 || index >= s.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            // Unicode safety: chars().nth() is O(N). optimized: as_bytes?
            // Ark strings are UTF-8. Indexing by byte or char?
            // Python does char. Rust String is UTF-8.
            // Let's use bytes for O(1) if we assume ASCII, or chars if we want correctness.
            // Standard: chars.
            if let Some(c) = s.chars().nth(index as usize) {
                let char_str = c.to_string();
                // Return [char_str, original_string]
                Ok(Value::List(vec![Value::String(char_str), Value::String(s)]))
            } else {
                Err(RuntimeError::NotExecutable)
            }
        }
        _ => Err(RuntimeError::TypeMismatch(
            "List or String".to_string(),
            list_val,
        )),
    }
}

pub fn intrinsic_list_append(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    // args: [list, item]
    // consume args
    let mut args = args;
    let item = args.pop().unwrap();
    let list_val = args.pop().unwrap();

    match list_val {
        Value::List(mut list) => {
            // Linear append: Modify in place if we owned it (we do, because args passed by value)
            list.push(item);
            Ok(Value::List(list))
        }
        _ => Err(RuntimeError::TypeMismatch("List".to_string(), list_val)),
    }
}

pub fn intrinsic_list_pop(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let idx_val = args.pop().unwrap();
    let list_val = args.pop().unwrap();

    let index = match idx_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), idx_val)),
    };

    match list_val {
        Value::List(mut list) => {
            if index < 0 || index >= list.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            // Linear Pop: Remove element. This is O(N) for middle elements, O(1) for end.
            // Returns [val, list]
            let val = list.remove(index as usize);
            Ok(Value::List(vec![val, Value::List(list)]))
        }
        _ => Err(RuntimeError::TypeMismatch("List".to_string(), list_val)),
    }
}

pub fn intrinsic_len(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let val = args.pop().unwrap();

    let len = match &val {
        Value::String(s) => s.len() as i64,
        Value::List(l) => l.len() as i64,
        Value::Buffer(b) => b.len() as i64,
        _ => return Err(RuntimeError::TypeMismatch("Sequence".to_string(), val)),
    };

    Ok(Value::List(vec![Value::Integer(len), val]))
}

pub fn intrinsic_struct_get(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }

    let mut args = args;
    let field_val = args.pop().unwrap();
    let struct_val = args.pop().unwrap();

    let field = match field_val {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String Key".to_string(),
                field_val,
            ));
        }
    };

    match struct_val {
        Value::Struct(data) => {
            let val_opt = data.get(&field).cloned();
            if let Some(val) = val_opt {
                Ok(Value::List(vec![val, Value::Struct(data)]))
            } else {
                Err(RuntimeError::VariableNotFound(field))
            }
        }
        _ => Err(RuntimeError::TypeMismatch("Struct".to_string(), struct_val)),
    }
}

pub fn intrinsic_list_set(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }
    // args: [list, index, value]
    let mut args = args;
    let val = args.pop().unwrap();
    let idx_val = args.pop().unwrap();
    let list_val = args.pop().unwrap();

    let index = match idx_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), idx_val)),
    };

    match list_val {
        Value::List(mut list) => {
            if index < 0 || index >= list.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            list[index as usize] = val;
            Ok(Value::List(list))
        }
        _ => Err(RuntimeError::TypeMismatch("List".to_string(), list_val)),
    }
}

pub fn intrinsic_struct_set(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    let mut args = args;
    let new_val = args.pop().unwrap();
    let field_val = args.pop().unwrap();
    let struct_val = args.pop().unwrap();

    let field = match field_val {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String Key".to_string(),
                field_val,
            ));
        }
    };

    match struct_val {
        Value::Struct(mut data) => {
            // Linear Update: Mutate in place (we own it)
            data.insert(field, new_val);
            Ok(Value::Struct(data))
        }
        _ => Err(RuntimeError::TypeMismatch("Struct".to_string(), struct_val)),
    }
}

pub fn intrinsic_time_now(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RuntimeError::InvalidOperation("Time went backwards".to_string()))?;
    Ok(Value::Integer(since_the_epoch.as_millis() as i64))
}

pub fn intrinsic_time_sleep(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }

    let duration_ms = match args[0] {
        Value::Integer(n) => {
             if n < 0 {
                  return Err(RuntimeError::InvalidOperation("Negative sleep duration".to_string()));
             }
             n as u64
        }
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), args[0].clone())),
    };

    #[cfg(target_arch = "wasm32")]
    {
         // In WASM, blocking sleep is generally not supported or freezes the browser.
         // We'll log it as a simulation.
         println!("[Ark:Time] Sleep {}ms (Simulated)", duration_ms);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        thread::sleep(Duration::from_millis(duration_ms));
    }

    Ok(Value::Unit)
}

pub fn intrinsic_math_pow(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(base), Value::Integer(exp)) => {
            let res = (*base as f64).powf(*exp as f64);
            Ok(Value::Integer(res as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_sqrt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            if *n < 0 {
                return Err(RuntimeError::InvalidOperation(
                    "Square root of negative number".to_string(),
                ));
            }
            let res = (*n as f64).sqrt();
            Ok(Value::Integer(res as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_sin(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let angle = (*n as f64) / 10000.0;
            let res = angle.sin();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_cos(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let angle = (*n as f64) / 10000.0;
            let res = angle.cos();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_tan(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let angle = (*n as f64) / 10000.0;
            let res = angle.tan();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_asin(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let val = (*n as f64) / 10000.0;
            if val < -1.0 || val > 1.0 {
                return Err(RuntimeError::InvalidOperation(
                    "asin out of domain".to_string(),
                ));
            }
            let res = val.asin();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_acos(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let val = (*n as f64) / 10000.0;
            if val < -1.0 || val > 1.0 {
                return Err(RuntimeError::InvalidOperation(
                    "acos out of domain".to_string(),
                ));
            }
            let res = val.acos();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_atan(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let val = (*n as f64) / 10000.0;
            let res = val.atan();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_atan2(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(y), Value::Integer(x)) => {
            let y_val = (*y as f64) / 10000.0;
            let x_val = (*x as f64) / 10000.0;
            let res = y_val.atan2(x_val);
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_io_cls(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    print!("\x1b[2J\x1b[H");
    Ok(Value::Unit)
}

pub fn intrinsic_io_read_bytes(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
             return Err(RuntimeError::TypeMismatch("String".to_string(), args[0].clone()));
        }
    };

    check_path_security(path_str)?;

    #[cfg(target_arch = "wasm32")]
    {
         println!("[Ark:VFS] Read Bytes from '{}': (Simulated)", path_str);
         Ok(Value::List(vec![]))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
         // Security: Path Traversal Check
         let safe_path = validate_safe_path(path_str)?;
         let content = fs::read(safe_path).map_err(|_| RuntimeError::NotExecutable)?;

         // Convert Vec<u8> to Vec<Value> (List of Integers)
         let list = content.into_iter().map(|b| Value::Integer(b as i64)).collect();
         Ok(Value::List(list))
    }
}

pub fn intrinsic_io_read_line(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::NotExecutable);
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(Value::String("".to_string()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|_| RuntimeError::NotExecutable)?;
        // Trim newline
        if input.ends_with('\n') {
            input.pop();
            if input.ends_with('\r') {
                input.pop();
            }
        }
        Ok(Value::String(input))
    }
}

pub fn intrinsic_io_write(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let s = match &args[0] {
        Value::String(s) => s,
        _ => return Err(RuntimeError::TypeMismatch("String".to_string(), args[0].clone())),
    };

    print!("{}", s);
    #[cfg(not(target_arch = "wasm32"))]
    {
        io::stdout().flush().map_err(|_| RuntimeError::NotExecutable)?;
    }
    Ok(Value::Unit)
}

pub fn intrinsic_io_read_file_async(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // MVP: Blocking Fallback.
    // In a future version, this should spawn a thread or use Tokio fs and return a Promise/Future object.
    intrinsic_fs_read(args)
}

pub fn intrinsic_chain_height(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    Ok(Value::Integer(10000))
}

pub fn intrinsic_chain_get_balance(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::String(_) => Ok(Value::Integer(5000)),
        _ => Err(RuntimeError::TypeMismatch(
            "String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_chain_submit_tx(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::String(_) => Ok(Value::String("0x123...".to_string())),
        _ => Err(RuntimeError::TypeMismatch(
            "String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_chain_verify_tx(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::String(_) => Ok(Value::Boolean(true)),
        _ => Err(RuntimeError::TypeMismatch(
            "String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_fs_write_buffer(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    check_path_security(path_str)?;

    match &args[1] {
        Value::Buffer(buf) => {
            #[cfg(target_arch = "wasm32")]
            {
                println!(
                    "[Ark:VFS] Write Buffer to '{}': [Size: {}]",
                    path_str,
                    buf.len()
                );
                Ok(Value::Unit)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if std::path::Path::new(path_str).exists() {
                    println!(
                        "[Ark:NTS] WARNING: Overwriting existing file '{}' without explicit lock (LAT).",
                        path_str
                    );
                }
                println!("[Ark:FS] Writing buffer to {}", path_str);
                fs::write(path_str, buf).map_err(|_| RuntimeError::NotExecutable)?;
                Ok(Value::Unit)
            }
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Buffer".to_string(),
            args[1].clone(),
        )),
    }
}

pub fn intrinsic_fs_read_buffer(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    check_path_security(path_str)?;

    #[cfg(target_arch = "wasm32")]
    {
        println!("[Ark:VFS] Read Buffer from '{}': [Empty]", path_str);
        Ok(Value::Buffer(vec![]))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("[Ark:FS] Reading buffer from {}", path_str);
        // Security: Path Traversal Check
        let safe_path = validate_safe_path(path_str)?;
        let content = fs::read(safe_path).map_err(|_| RuntimeError::NotExecutable)?;
        Ok(Value::Buffer(content))
    }
}

pub fn intrinsic_math_sin_scaled(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }
    let angle = match &args[0] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };
    let scale_in = match &args[1] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[1].clone(),
            ));
        }
    };
    let scale_out = match &args[2] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[2].clone(),
            ));
        }
    };

    if scale_in == 0.0 {
        return Err(RuntimeError::InvalidOperation("Scale In is 0".to_string()));
    }

    let res = (angle / scale_in).sin() * scale_out;
    Ok(Value::Integer(res.round() as i64))
}

pub fn intrinsic_math_cos_scaled(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }
    let angle = match &args[0] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };
    let scale_in = match &args[1] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[1].clone(),
            ));
        }
    };
    let scale_out = match &args[2] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[2].clone(),
            ));
        }
    };

    if scale_in == 0.0 {
        return Err(RuntimeError::InvalidOperation("Scale In is 0".to_string()));
    }

    let res = (angle / scale_in).cos() * scale_out;
    Ok(Value::Integer(res.round() as i64))
}

pub fn intrinsic_math_pi_scaled(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let scale = match &args[0] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };

    let res = std::f64::consts::PI * scale;
    Ok(Value::Integer(res.round() as i64))
}

pub fn intrinsic_str_from_code(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let code = match &args[0] {
        Value::Integer(i) => *i as u32,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };
    if let Some(c) = std::char::from_u32(code) {
        Ok(Value::String(c.to_string()))
    } else {
        Err(RuntimeError::InvalidOperation(
            "Invalid Char Code".to_string(),
        ))
    }
}

pub fn intrinsic_extract_code(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let text = match &args[0] {
        Value::String(s) => s,
        _ => return Err(RuntimeError::TypeMismatch("String".to_string(), args[0].clone())),
    };

    // Regex to capture fenced code blocks: ```lang ... ```
    let re = Regex::new(r"```(?:\w+)?\n([\s\S]*?)```").map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let mut blocks = Vec::new();
    for cap in re.captures_iter(text) {
        if let Some(match_str) = cap.get(1) {
            blocks.push(Value::String(match_str.as_str().to_string()));
        }
    }

    Ok(Value::List(blocks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Value;

    #[test]
    fn test_time_now() {
        let res = intrinsic_time_now(vec![]);
        match res {
            Ok(Value::Integer(t)) => assert!(t > 0),
            _ => panic!("Expected Integer, got {:?}", res),
        }
    }

    #[test]
    fn test_math_pow() {
        // 2^3 = 8
        let args = vec![Value::Integer(2), Value::Integer(3)];
        assert_eq!(intrinsic_math_pow(args).unwrap(), Value::Integer(8));

        // 10^2 = 100
        let args = vec![Value::Integer(10), Value::Integer(2)];
        assert_eq!(intrinsic_math_pow(args).unwrap(), Value::Integer(100));

        // 2^-1 = 0 (0.5 as integer)
        let args = vec![Value::Integer(2), Value::Integer(-1)];
        assert_eq!(intrinsic_math_pow(args).unwrap(), Value::Integer(0));
    }

    #[test]
    fn test_math_sqrt() {
        // sqrt(16) = 4
        let args = vec![Value::Integer(16)];
        assert_eq!(intrinsic_math_sqrt(args).unwrap(), Value::Integer(4));

        // sqrt(10) = 3 (3.16... as integer)
        let args = vec![Value::Integer(10)];
        assert_eq!(intrinsic_math_sqrt(args).unwrap(), Value::Integer(3));

        // sqrt(-1) -> Error
        let args = vec![Value::Integer(-1)];
        assert!(intrinsic_math_sqrt(args).is_err());
    }

    #[test]
    fn test_io_cls() {
        // Just verify it runs and returns Unit
        let args = vec![];
        assert_eq!(intrinsic_io_cls(args).unwrap(), Value::Unit);
    }

    #[test]
    fn test_math_trig() {
        // sin(0) = 0
        let args = vec![Value::Integer(0)];
        assert_eq!(intrinsic_math_sin(args).unwrap(), Value::Integer(0));

        // sin(PI/2) approx 10000 (PI/2 = 1.5707... * 10000 = 15707)
        let args = vec![Value::Integer(15708)]; // 1.5708
        // sin(1.5708) is close to 1
        let res = intrinsic_math_sin(args).unwrap();
        if let Value::Integer(v) = res {
            assert!(v >= 9999 && v <= 10000);
        } else {
            panic!("Expected Integer");
        }

        // cos(0) = 10000
        let args = vec![Value::Integer(0)];
        assert_eq!(intrinsic_math_cos(args).unwrap(), Value::Integer(10000));

        // tan(45deg) = tan(PI/4) = 1 (approx)
        // PI/4 = 0.78539 * 10000 = 7854
        let args = vec![Value::Integer(7854)];
        let res = intrinsic_math_tan(args).unwrap();
        if let Value::Integer(v) = res {
            assert!(v >= 9990 && v <= 10010);
        } else {
            panic!("Expected Integer");
        }
    }

    #[test]
    fn test_crypto_verify() {
        // Valid Signature (Test Vector 2 from RFC 8032)
        // Msg: "r" (0x72)
        let msg = Value::String("r".to_string());
        let sig_hex = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";
        let pubkey_hex = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";

        let args = vec![
            msg.clone(),
            Value::String(sig_hex.to_string()),
            Value::String(pubkey_hex.to_string()),
        ];
        let res = intrinsic_crypto_verify(args).unwrap();
        assert_eq!(res, Value::Boolean(true));

        // Invalid Signature (Modified first byte 92 -> 93)
        let invalid_sig_hex = "93a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";
        let args = vec![
            msg.clone(),
            Value::String(invalid_sig_hex.to_string()),
            Value::String(pubkey_hex.to_string()),
        ];
        let res = intrinsic_crypto_verify(args).unwrap();
        assert_eq!(res, Value::Boolean(false));

        // Invalid Message
        let args = vec![
            Value::String("wrong".to_string()),
            Value::String(sig_hex.to_string()),
            Value::String(pubkey_hex.to_string()),
        ];
        let res = intrinsic_crypto_verify(args).unwrap();
        assert_eq!(res, Value::Boolean(false));
    }

    #[test]
    fn test_buffer_write_linear() {
        // Setup: Buffer of size 3
        let buf = Value::Buffer(vec![0u8; 3]);
        // args: [buffer, index, value] -> sys.mem.write(buf, 1, 42)
        let args = vec![buf, Value::Integer(1), Value::Integer(42)];

        // Execute
        let res = intrinsic_buffer_write(args).unwrap();

        // Assert
        match res {
            Value::Buffer(b) => {
                assert_eq!(b.len(), 3);
                assert_eq!(b[1], 42);
            }
            _ => panic!("Expected Buffer"),
        }
    }

    #[test]
    fn test_security_fs_write_traversal() {
        // [MODE: KINETIC_EXECUTION]
        // Rationale: Attempt to write outside the sandbox.
        // We expect this to FAIL now that the fix is applied.

        let file_name = "../intrinsics_test_exploit.txt";

        // Clean up before test just in case
        let _ = std::fs::remove_file(file_name);

        let args = vec![
            Value::String(file_name.to_string()),
            Value::String("pwned".to_string()),
        ];

        // At this stage (after fix), we expect this to FAIL.
        let res = intrinsic_fs_write(args);

        // Assert Error
        match res {
            Err(RuntimeError::NotExecutable) => {}
            _ => panic!("Expected RuntimeError::NotExecutable, got {:?}", res),
        }

        // Verify file was NOT written
        if std::path::Path::new(file_name).exists() {
            // Cleanup if it somehow wrote
            std::fs::remove_file(file_name).unwrap();
            panic!("File was written despite error!");
        }
    }

    #[test]
    fn test_security_fs_write_valid() {
        let file_name = "intrinsics_test_safe.txt";
        let _ = std::fs::remove_file(file_name);

        let args = vec![
            Value::String(file_name.to_string()),
            Value::String("safe".to_string()),
        ];

        let res = intrinsic_fs_write(args);
        assert!(res.is_ok());

        assert!(std::path::Path::new(file_name).exists());
        std::fs::remove_file(file_name).unwrap();
    }

    #[test]
    fn test_security_fs_read_traversal() {
        let file_name = "../Cargo.toml";
        // This file exists in repo root, but is outside core/ CWD.
        // So it should be blocked.

        if std::path::Path::new(file_name).exists() {
            let args = vec![Value::String(file_name.to_string())];
            let res = intrinsic_fs_read(args);
            match res {
                Err(RuntimeError::NotExecutable) => {}
                _ => panic!("Expected RuntimeError::NotExecutable, got {:?}", res),
            }
        } else {
            println!("Skipping read traversal test because ../Cargo.toml not found");
        }
    }

    #[test]
    fn test_list_pop() {
        // [1, 2, 3] pop(1) -> 2, list becomes [1, 3]
        let list = Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        let args = vec![list, Value::Integer(1)];
        let res = intrinsic_list_pop(args).unwrap();

        match res {
            Value::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], Value::Integer(2)); // Popped value
                match &items[1] {
                    Value::List(l) => {
                        assert_eq!(l.len(), 2);
                        assert_eq!(l[0], Value::Integer(1));
                        assert_eq!(l[1], Value::Integer(3));
                    }
                    _ => panic!("Expected List as second item"),
                }
            }
            _ => panic!("Expected List result"),
        }
    }

    #[test]
    fn test_time_sleep() {
        let args = vec![Value::Integer(10)];
        assert!(intrinsic_time_sleep(args).is_ok());
    }

    #[test]
    fn test_time_sleep_negative() {
        let args = vec![Value::Integer(-10)];
        assert!(intrinsic_time_sleep(args).is_err());
    }

    #[test]
    fn test_io_write_basic() {
        let args = vec![Value::String("test output".to_string())];
        assert!(intrinsic_io_write(args).is_ok());
    }

    #[test]
    fn test_io_read_bytes_valid() {
        let filename = "test_bytes.bin";
        let _ = std::fs::remove_file(filename);
        std::fs::write(filename, vec![1, 2, 3]).unwrap();

        let args = vec![Value::String(filename.to_string())];
        let res = intrinsic_io_read_bytes(args).unwrap();

        match res {
            Value::List(l) => {
                assert_eq!(l.len(), 3);
                assert_eq!(l[0], Value::Integer(1));
                assert_eq!(l[1], Value::Integer(2));
                assert_eq!(l[2], Value::Integer(3));
            }
            _ => panic!("Expected List"),
        }
        let _ = std::fs::remove_file(filename);
    }

    #[test]
    fn test_extract_code_blocks() {
        let md = "Start\n```rust\nfn main() {}\n```\nMid\n```\nraw\n```\nEnd";
        let args = vec![Value::String(md.to_string())];
        let res = intrinsic_extract_code(args).unwrap();

        match res {
            Value::List(blocks) => {
                assert_eq!(blocks.len(), 2);
                match &blocks[0] {
                    Value::String(s) => assert_eq!(s, "fn main() {}\n"),
                    _ => panic!("Expected String"),
                }
                match &blocks[1] {
                    Value::String(s) => assert_eq!(s, "raw\n"),
                    _ => panic!("Expected String"),
                }
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_extract_code_empty() {
        let md = "No code blocks here.";
        let args = vec![Value::String(md.to_string())];
        let res = intrinsic_extract_code(args).unwrap();

        match res {
            Value::List(blocks) => assert!(blocks.is_empty()),
            _ => panic!("Expected List"),
        }
    }
}
