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

use ark_0_zheng::{checker, compiler::Compiler, loader, vm::VM};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let filename = &args[1];

        match fs::read_to_string(filename) {
            Ok(json_content) => match loader::load_ark_program(&json_content) {
                Ok(mast) => {
                    // Phase 3: The Linear Shield
                    if let Err(e) = checker::LinearChecker::check(&mast.content) {
                        println!("[Ark:TypeCheck] Error: {}", e);
                        return;
                    }

                    // Compile and run via VM
                    let compiler = Compiler::new();
                    let chunk = compiler.compile(&mast.content);
                    match VM::new(chunk, &mast.hash, 0) {
                        Ok(mut vm) => match vm.run() {
                            Ok(_) => {}
                            Err(e) => println!("Runtime Error: {}", e),
                        },
                        Err(e) => println!("VM Init Error: {}", e),
                    }
                }
                Err(e) => println!("Failed to load program: {}", e),
            },
            Err(e) => println!("Failed to read file: {}", e),
        }
    } else {
        println!("Usage: ark <file.json>");
        println!("  Executes an Ark program (JSON MAST format).");
    }
}
