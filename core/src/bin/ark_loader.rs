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

use ark_0_zheng::compiler::Compiler; // JIT
use ark_0_zheng::loader::load_ark_program;
use ark_0_zheng::vm::VM; // Bytecode VM
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ark_loader <program.json> [args...]");
        return;
    }

    let filename = &args[1];
    let json_content = fs::read_to_string(filename).expect("Failed to read file");

    match load_ark_program(&json_content) {
        Ok(mast) => {
            // println!("MAST Loaded Successfully. Hash: {}", mast.hash);

            let mut ark_args = Vec::new();
            for arg in &args[1..] {
                ark_args.push(ark_0_zheng::runtime::Value::String(arg.clone()));
            }

            // 1. JIT Compile
            let compiler = Compiler::new();
            let chunk = compiler.compile(&mast.content);

            // 2. Setup VM
            // Allow setting security level via env var (Default: 0)
            let security_level = env::var("ARK_SECURITY_LEVEL")
                .unwrap_or("0".to_string())
                .parse::<u8>()
                .unwrap_or(0);

            match VM::new(chunk, &mast.hash, security_level) {
                Ok(mut vm) => {
                    // 3. Inject Args into Global Scope (Scope 0)
                    if let Some(scope) = vm.scopes.get_mut(0) {
                        scope.set(
                            "sys_args".to_string(),
                            ark_0_zheng::runtime::Value::List(ark_args),
                        );
                    }

                    // 4. Run
                    match vm.run() {
                        Ok(_val) => {
                            // println!("Execution Result: {:?}", val);
                        }
                        Err(e) => {
                            eprintln!("Runtime Error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("VM Initialization Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => eprintln!("Load Error: {:?}", e),
    }
}
