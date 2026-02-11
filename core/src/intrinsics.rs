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
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;
use std::time::Duration;

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
            "intrinsic_merkle_root" | "sys.crypto.merkle_root" => Some(intrinsic_merkle_root),
            "intrinsic_buffer_alloc" | "sys.mem.alloc" => Some(intrinsic_buffer_alloc),
            "intrinsic_buffer_inspect" | "sys.mem.inspect" => Some(intrinsic_buffer_inspect),
            "intrinsic_buffer_read" | "sys.mem.read" => Some(intrinsic_buffer_read),
            "intrinsic_buffer_write" | "sys.mem.write" => Some(intrinsic_buffer_write),
            "intrinsic_list_get" | "sys.list.get" | "sys.str.get" => Some(intrinsic_list_get),
            "intrinsic_list_append" | "sys.list.append" => Some(intrinsic_list_append),
            "intrinsic_len" | "sys.len" => Some(intrinsic_len),
            "intrinsic_struct_get" | "sys.struct.get" => Some(intrinsic_struct_get),
            "intrinsic_struct_set" | "sys.struct.set" => Some(intrinsic_struct_set),
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
            "sys.struct.get".to_string(),
            Value::NativeFunction(intrinsic_struct_get),
        );
        scope.set(
            "sys.struct.set".to_string(),
            Value::NativeFunction(intrinsic_struct_set),
        );
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
        return Ok(Value::String("[Ark:AI] Unavailable in Browser Runtime (OIS: Low)".to_string()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let api_key = std::env::var("GOOGLE_API_KEY").map_err(|_| {
            println!("[Ark:AI] Error: GOOGLE_API_KEY not set.");
            RuntimeError::NotExecutable
        })?;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
            api_key
        );

        let client = Client::new();
        let payload = json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        });

        println!("[Ark:AI] Contacting Gemini (Native Rust)...");

        // Simple Retry Logic
        for attempt in 0..3 {
            match client.post(&url).json(&payload).send() {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json_resp: serde_json::Value = resp.json().map_err(|e| {
                            println!("[Ark:AI] JSON Error: {}", e);
                            RuntimeError::NotExecutable
                        })?;

                        if let Some(text) =
                            json_resp["candidates"][0]["content"]["parts"][0]["text"].as_str()
                        {
                            return Ok(Value::String(text.to_string()));
                        }
                    } else if resp.status().as_u16() == 429 {
                        println!("[Ark:AI] Rate limit (429). Retrying...");
                        std::thread::sleep(Duration::from_secs(2u64.pow(attempt)));
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
        let code = "import datetime\nprint(f'Sovereignty Established: {datetime.datetime.now()}')\n";
        let end = "```";
        Ok(Value::String(format!("{}{}{}", start, code, end)))
    }
}

pub fn intrinsic_exec(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let cmd_str = match &args[0] {
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
        println!("[Ark:WASM] Security Block: sys.exec('{}') denied.", cmd_str);
        return Err(RuntimeError::NotExecutable);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("[Ark:Exec] {}", cmd_str);

        // Windows vs Unix
        #[cfg(target_os = "windows")]
        let mut cmd = Command::new("cmd");
        #[cfg(target_os = "windows")]
        cmd.args(["/C", cmd_str]);

        #[cfg(not(target_os = "windows"))]
        let mut cmd = Command::new("sh");
        #[cfg(not(target_os = "windows"))]
        cmd.args(["-c", cmd_str]);

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
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (Value::String(a), Value::Integer(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (Value::Integer(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (Value::String(a), Value::Boolean(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (Value::Boolean(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        _ => Err(RuntimeError::TypeMismatch(
            "Integer, String, or Boolean".to_string(),
            args[0].clone(),
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

    #[cfg(target_arch = "wasm32")]
    {
        println!("[Ark:VFS] Read from '{}': (Simulated) [Empty]", path_str);
        Ok(Value::String("".to_string()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("[Ark:FS] Reading from {}", path_str);
        let content = fs::read_to_string(path_str).map_err(|_| RuntimeError::NotExecutable)?;
        Ok(Value::String(content))
    }
}

pub fn intrinsic_crypto_hash(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let data = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    Ok(Value::String(hex::encode(result)))
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

    if leaves.is_empty() {
        return Ok(Value::String("".to_string()));
    }

    // Hash leaves first
    let mut current_level: Vec<String> = leaves
        .into_iter()
        .map(|s| {
            let mut hasher = Sha256::new();
            hasher.update(s);
            hex::encode(hasher.finalize())
        })
        .collect();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for i in (0..current_level.len()).step_by(2) {
            let left = &current_level[i];
            let right = if i + 1 < current_level.len() {
                &current_level[i + 1]
            } else {
                left // Duplicate last if odd
            };

            let mut hasher = Sha256::new();
            // Hash(left + right)
            hasher.update(left);
            hasher.update(right);
            next_level.push(hex::encode(hasher.finalize()));
        }
        current_level = next_level;
    }

    Ok(Value::String(current_level[0].clone()))
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
    // args: [buffer, index, value] BUT Value::Buffer is cloned in args?
    // CRITICAL ISSUE: Value passed to intrinsic is a CLONE if not Linear.
    // In Rust AST/Eval, we pass "Vec<Value>" which owns the values.
    // If the Buffer IS the value, we are modifying the local copy in `args`.
    // We need to mutate the original. But `intrinsic` signature consumes args.
    // THE ONLY WAY to mutate is if the intrinsic returns the modified buffer
    // OR if we use Reference types (which we don't have yet)
    // OR if Buffer internally uses Arc<Mutex<Vec>> or Unsafe Pointer.
    // A "Bio-Bridge" needs shared memory.
    // "Pointer Swapping" strategy:
    // We can't mutate `args[0]` and have it reflect caller unless we return it.
    // BUT `intrinsic_buffer_write(buf, i, v)` -> returns `buf`?
    // That's functional style.
    // S-Lang "Linear Types" - we consume the buffer and return a new one (same data, effectively).
    // Let's implement that: Consume Buffer, Mutate in place, Return Buffer.
    // This aligns with "Zombie Killer" Linear constraints too!

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
        _ => Err(RuntimeError::TypeMismatch(
            "Struct".to_string(),
            struct_val,
        )),
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
        _ => Err(RuntimeError::TypeMismatch(
            "Struct".to_string(),
            struct_val,
        )),
    }
}
