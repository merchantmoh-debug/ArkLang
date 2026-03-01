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

use crate::ast::{ArkNode, Expression};
use crate::eval::Interpreter;
use crate::runtime::Scope;
use std::io::{self, Write};

pub fn start() {
    println!("Ark-0 (Zheng) Sovereign Shell v0.1");
    println!("Type 'ADD x y' to test intrinsics, or 'exit' to quit.");

    let mut scope = Scope::new();

    loop {
        print!("> ");
        io::stdout().flush().expect("stdout flush failed");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Error reading input");
            continue;
        }

        let input = input.trim();
        if input == "exit" {
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Extremely naive 'parser' for Phase 2 prototype
        // Support: ADD 5 10
        let parts: Vec<&str> = input.split_whitespace().collect();
        let expr = if parts[0] == "ADD" && parts.len() == 3 {
            Expression::Call {
                function_hash: "intrinsic_add".to_string(),
                args: vec![
                    Expression::Literal(parts[1].to_string()),
                    Expression::Literal(parts[2].to_string()),
                ],
            }
        } else if parts[0] == "PRINT" && parts.len() > 1 {
            Expression::Call {
                function_hash: "intrinsic_print".to_string(),
                args: vec![Expression::Literal(parts[1].to_string())],
            }
        } else {
            // Default: Literal evaluation
            Expression::Literal(input.to_string())
        };

        let mut interpreter = Interpreter::new();
        match interpreter.eval(&ArkNode::Expression(expr), &mut scope) {
            Ok(val) => println!("= {:?}", val),
            Err(e) => println!("Error: {}", e),
        }
    }
}
