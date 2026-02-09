/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use crate::eval::EvalError;
use crate::runtime::Value;
use reqwest::blocking::Client;
use serde_json::json;
use std::fs;
use std::process::Command;
use std::time::Duration;

type NativeFn = fn(Vec<Value>) -> Result<Value, EvalError>;

pub struct IntrinsicRegistry;

impl IntrinsicRegistry {
    pub fn resolve(hash: &str) -> Option<NativeFn> {
        match hash {
            "intrinsic_add" => Some(intrinsic_add),
            "intrinsic_sub" => Some(intrinsic_sub),
            "intrinsic_mul" => Some(intrinsic_mul),
            "intrinsic_gt" => Some(intrinsic_gt),
            "intrinsic_lt" => Some(intrinsic_lt),
            "intrinsic_eq" => Some(intrinsic_eq),
            "intrinsic_print" => Some(intrinsic_print),
            "print" => Some(intrinsic_print),
            "intrinsic_ask_ai" => Some(intrinsic_ask_ai),
            "sys_exec" | "intrinsic_exec" => Some(intrinsic_exec),
            "sys_fs_write" | "intrinsic_fs_write" => Some(intrinsic_fs_write),
            _ => None,
        }
    }
}

fn intrinsic_ask_ai(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 1 {
        return Err(EvalError::NotExecutable);
    }

    let prompt = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(EvalError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ))
        }
    };

    let api_key = std::env::var("GOOGLE_API_KEY").map_err(|_| {
        println!("[Ark:AI] Error: GOOGLE_API_KEY not set.");
        EvalError::NotExecutable
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
                        EvalError::NotExecutable
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

fn intrinsic_exec(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 1 {
        return Err(EvalError::NotExecutable);
    }
    let cmd_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(EvalError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ))
        }
    };

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

    let output = cmd.output().map_err(|_| EvalError::NotExecutable)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(Value::String(stdout))
}

fn intrinsic_fs_write(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(EvalError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ))
        }
    };
    let content = match &args[1] {
        Value::String(s) => s,
        _ => {
            return Err(EvalError::TypeMismatch(
                "String".to_string(),
                args[1].clone(),
            ))
        }
    };

    // NTS Protocol: Intentional Friction (Level 1)
    if std::path::Path::new(path_str).exists() {
        println!(
            "[Ark:NTS] WARNING: Overwriting existing file '{}' without explicit lock (LAT).",
            path_str
        );
    }

    println!("[Ark:FS] Writing to {}", path_str);
    fs::write(path_str, content).map_err(|_| EvalError::NotExecutable)?;
    Ok(Value::Unit)
}

fn intrinsic_add(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (Value::String(a), Value::Integer(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (Value::Integer(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        _ => Err(EvalError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

fn intrinsic_sub(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
        _ => Err(EvalError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

fn intrinsic_mul(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
        _ => Err(EvalError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

fn intrinsic_gt(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a > b { 1 } else { 0 })),
        _ => Err(EvalError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

fn intrinsic_lt(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a < b { 1 } else { 0 })),
        _ => Err(EvalError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

fn intrinsic_eq(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a == b { 1 } else { 0 })),
        _ => Err(EvalError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

fn intrinsic_print(args: Vec<Value>) -> Result<Value, EvalError> {
    for arg in args {
        println!("[Ark:Out] {:?}", arg);
    }
    Ok(Value::Unit)
}
