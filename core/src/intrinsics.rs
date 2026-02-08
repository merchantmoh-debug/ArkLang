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

use crate::eval::EvalError;
use crate::runtime::Value;

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

    // Construct path to bridge script (relative to binary or absolute)
    let bridge_path = "C:\\Users\\Stran\\.gemini\\antigravity\\brain\\87a06051-a8dc-48c3-9d36-cf0f67b80b77\\ark_bridge.py";

    println!("[Neuro-Link] Transmitting thought to Gemini 3.0 Pro...");

    let output = std::process::Command::new("python")
        .arg(bridge_path)
        .arg(prompt)
        .output()
        .map_err(|_| EvalError::NotExecutable)?;

    if output.status.success() {
        let response = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Value::String(response))
    } else {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        println!("[Neuro-Link] Error: {}", error);
        Ok(Value::String(format!("AI Error: {}", error)))
    }
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
