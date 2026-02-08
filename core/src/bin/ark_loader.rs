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
            println!("MAST Loaded Successfully.");
            let mut scope = Scope::new();
            match Interpreter::eval(&node, &mut scope) {
                Ok(val) => println!("Execution Result: {:?}", val),
                Err(e) => eprintln!("Execution Error: {:?}", e),
            }
        }
        Err(e) => eprintln!("Load Error: {:?}", e),
    }
}
