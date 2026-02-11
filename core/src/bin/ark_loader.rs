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

use ark_0_zheng::eval::Interpreter;
use ark_0_zheng::loader::load_ark_program;
use ark_0_zheng::runtime::Scope;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ark_loader <program.json>");
        return;
    }

    let filename = &args[1];
    let json_content = fs::read_to_string(filename).expect("Failed to read file");

    match load_ark_program(&json_content) {
        Ok(node) => {
            // println!("MAST Loaded Successfully.");
            let mut scope = Scope::new();

            // Inject sys.args (List of Strings)
            let mut ark_args = Vec::new();
            for arg in args {
                ark_args.push(ark_0_zheng::runtime::Value::String(arg));
            }
            // We need to set this in a global or intrinsic accessible way.
            // But intrinsics don't capture scope.
            // Option 1: Add to Scope as "sys.args" variable.
            // Option 2: Add to Intrinsic Registry STATE (Registry is stateless).
            // Option 3: Add to Scope, and users access via variable `sys_args` (not intrinsic).
            // Python prototype used `sys.argv`.
            // Let's inject it as a variable "sys_args" in the root scope.
            // Users can do: `args := sys_args`

            scope.set(
                "sys_args".to_string(),
                ark_0_zheng::runtime::Value::List(ark_args),
            );

            let mut interpreter = Interpreter::new();
            match interpreter.eval(&node, &mut scope) {
                Ok(_val) => {
                    // println!("Execution Result: {:?}", val);
                }
                Err(e) => eprintln!("Execution Error: {:?}", e),
            }
        }
        Err(e) => eprintln!("Load Error: {:?}", e),
    }
}
