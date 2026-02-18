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

//! `ark` — The Ark Sovereign Compiler CLI
//!
//! Usage:
//!   ark run <file.ark>           Parse and execute an Ark source file
//!   ark run <file.json>          Load and execute a JSON MAST file (legacy)
//!   ark check <file.ark>         Parse and run linear type checker
//!   ark parse <file.ark>         Parse and dump AST as JSON
//!   ark version                  Print version
//!   ark help                     Print usage

use ark_0_zheng::compiler::Compiler;
use ark_0_zheng::debugger::{self, DebugAction, DebugState, StepMode};
use ark_0_zheng::loader::load_ark_program;
use ark_0_zheng::parser;
use ark_0_zheng::vm::VM;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::process;
use std::rc::Rc;

const VERSION: &str = "1.2.0";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    let command = args[1].as_str();

    match command {
        "run" => cmd_run(&args[2..]),
        "debug" => cmd_debug(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "parse" => cmd_parse(&args[2..]),
        "version" | "--version" | "-v" => {
            println!("Ark Sovereign Compiler v{}", VERSION);
        }
        "help" | "--help" | "-h" => print_usage(),
        // Legacy: if first arg is a file path, treat as `run`
        _ if command.ends_with(".json") || command.ends_with(".ark") => {
            cmd_run(&args[1..]);
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Ark Sovereign Compiler v{}", VERSION);
    println!();
    println!("Usage:");
    println!("  ark run <file.ark|file.json>    Parse and execute a program");
    println!("  ark debug <file.ark>            Interactive step-through debugger");
    println!("  ark check <file.ark|file.json>  Run the linear type checker");
    println!("  ark parse <file.ark>            Parse and dump AST as JSON");
    println!("  ark version                     Print version info");
    println!("  ark help                        Print this help message");
}

/// Run an Ark program from either .ark source or .json MAST
fn cmd_run(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'run' requires a file argument");
        eprintln!("Usage: ark run <file.ark>");
        process::exit(1);
    }

    let filename = &args[0];
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    // Determine file type
    let ast = if filename.ends_with(".ark") {
        // Native Ark source → parse directly
        match parser::parse_source(&source, filename) {
            Ok(node) => node,
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    } else if filename.ends_with(".json") {
        // Legacy JSON MAST
        match load_ark_program(&source) {
            Ok(mast) => mast.content,
            Err(e) => {
                eprintln!("Load Error: {:?}", e);
                process::exit(1);
            }
        }
    } else {
        // Try as .ark first, fall back to JSON
        match parser::parse_source(&source, filename) {
            Ok(node) => node,
            Err(_) => match load_ark_program(&source) {
                Ok(mast) => mast.content,
                Err(e) => {
                    eprintln!(
                        "Error: Cannot parse '{}' as Ark source or JSON MAST: {:?}",
                        filename, e
                    );
                    process::exit(1);
                }
            },
        }
    };

    // Build program args for VM
    let mut ark_args = Vec::new();
    for arg in args {
        ark_args.push(ark_0_zheng::runtime::Value::String(arg.clone()));
    }

    // Compile
    let compiler = Compiler::new();
    let chunk = compiler.compile(&ast);

    // Hash for VM (use filename as fallback)
    let hash = format!("ark_native_{}", filename);

    // Setup VM
    let security_level = env::var("ARK_SECURITY_LEVEL")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<u8>()
        .unwrap_or(0);

    match VM::new(chunk, &hash, security_level) {
        Ok(mut vm) => {
            // Inject args
            if let Some(scope) = vm.scopes.get_mut(0) {
                scope.set(
                    "sys_args".to_string(),
                    ark_0_zheng::runtime::Value::List(ark_args),
                );
            }

            // Execute
            match vm.run() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Runtime Error: {}", e);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("VM Initialization Error: {}", e);
            process::exit(1);
        }
    }
}

/// Check linear types in a program
fn cmd_check(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'check' requires a file argument");
        process::exit(1);
    }

    let filename = &args[0];
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    let ast = if filename.ends_with(".ark") {
        match parser::parse_source(&source, filename) {
            Ok(node) => node,
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    } else {
        match load_ark_program(&source) {
            Ok(mast) => mast.content,
            Err(e) => {
                eprintln!("Load Error: {:?}", e);
                process::exit(1);
            }
        }
    };

    println!("Running Linear Check on {}...", filename);
    match ark_0_zheng::checker::LinearChecker::check(&ast) {
        Ok(_) => println!("✓ Linear Check Passed"),
        Err(e) => {
            eprintln!("✗ Linear Check Failed: {}", e);
            process::exit(1);
        }
    }
}

/// Parse an Ark file and dump AST as JSON
fn cmd_parse(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'parse' requires a file argument");
        process::exit(1);
    }

    let filename = &args[0];
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    match parser::parse_to_mast(&source, filename) {
        Ok(mast) => match serde_json::to_string_pretty(&mast) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("JSON serialization error: {}", e);
                process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

/// Interactive step-through debugger
fn cmd_debug(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'debug' requires a file argument");
        eprintln!("Usage: ark debug <file.ark>");
        process::exit(1);
    }

    let filename = &args[0];
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    // Store source lines for display
    let source_lines: Vec<&str> = source.lines().collect();

    let ast = match parser::parse_source(&source, filename) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Compile
    let compiler = Compiler::new();
    let chunk = compiler.compile(&ast);

    // Hash for VM
    let hash = format!("ark_debug_{}", filename);

    // Setup VM with debugger
    match VM::new(chunk, &hash, 0) {
        Ok(mut vm) => {
            // Debug state (shared with the hook via Rc<RefCell>)
            let state = Rc::new(RefCell::new(DebugState::new()));
            let state_clone = state.clone();

            // Clone source lines into an owned Vec for the closure
            let source_lines_owned: Vec<String> =
                source_lines.iter().map(|s| s.to_string()).collect();

            println!(
                "\x1b[1;36m[ARK DEBUGGER]\x1b[0m Loaded '{}' ({} lines)",
                filename,
                source_lines_owned.len()
            );
            println!(
                "Commands: \x1b[33mb <line>\x1b[0m (breakpoint), \x1b[33mn\x1b[0m (next), \x1b[33ms\x1b[0m (step), \x1b[33mc\x1b[0m (continue), \x1b[33mp <var>\x1b[0m (print), \x1b[33mvars\x1b[0m (all vars), \x1b[33mbt\x1b[0m (backtrace), \x1b[33mq\x1b[0m (quit)"
            );
            println!();

            // Set up the debug hook
            vm.debug_hook = Some(Box::new(move |stack, scopes, ip, chunk| {
                let mut dbg = state_clone.borrow_mut();
                let frame_depth = 0; // Simplified; real depth from frames

                if !dbg.should_break(ip, chunk, frame_depth) {
                    return DebugAction::Continue;
                }

                let loc = chunk.get_source_loc(ip);
                let line = loc.map(|l| l.line).unwrap_or(0);
                dbg.last_line = line;

                // Display current position
                println!();
                println!("\x1b[1;32m→ Stopped\x1b[0m at line {} (ip={})", line, ip);

                // Show source context (3 lines around current)
                let line_idx = line as usize;
                let start = if line_idx > 2 { line_idx - 2 } else { 1 };
                let end = std::cmp::min(line_idx + 2, source_lines_owned.len());
                for i in start..=end {
                    if i > 0 && i <= source_lines_owned.len() {
                        let marker = if i == line_idx { "►" } else { " " };
                        let color = if i == line_idx {
                            "\x1b[1;33m"
                        } else {
                            "\x1b[90m"
                        };
                        println!(
                            "{}{:>4} {} {}\x1b[0m",
                            color,
                            i,
                            marker,
                            source_lines_owned[i - 1]
                        );
                    }
                }
                println!();

                // Interactive REPL
                loop {
                    use std::io::{self, Write};
                    print!("\x1b[1;36m(ark-dbg)\x1b[0m ");
                    io::stdout().flush().unwrap();

                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_err() {
                        return DebugAction::Quit;
                    }
                    let input = input.trim();

                    if input.is_empty() {
                        // Repeat last action (default: step)
                        dbg.step_mode = StepMode::StepInto;
                        dbg.stepping = true;
                        return DebugAction::Continue;
                    }

                    let parts: Vec<&str> = input.splitn(2, ' ').collect();
                    match parts[0] {
                        "n" | "next" => {
                            dbg.step_mode = StepMode::StepOver;
                            dbg.step_over_depth = frame_depth;
                            dbg.stepping = true;
                            return DebugAction::Continue;
                        }
                        "s" | "step" => {
                            dbg.step_mode = StepMode::StepInto;
                            dbg.stepping = true;
                            return DebugAction::Continue;
                        }
                        "c" | "continue" => {
                            dbg.step_mode = StepMode::Continue;
                            dbg.stepping = false;
                            return DebugAction::Continue;
                        }
                        "q" | "quit" => {
                            println!("\x1b[1;31m[ARK DEBUGGER]\x1b[0m Quitting.");
                            return DebugAction::Quit;
                        }
                        "b" | "break" => {
                            if parts.len() > 1 {
                                if let Ok(line_num) = parts[1].parse::<u32>() {
                                    let was_set = dbg.toggle_breakpoint(line_num);
                                    if was_set {
                                        println!(
                                            "  \x1b[32m●\x1b[0m Breakpoint set at line {}",
                                            line_num
                                        );
                                    } else {
                                        println!(
                                            "  \x1b[31m○\x1b[0m Breakpoint removed from line {}",
                                            line_num
                                        );
                                    }
                                } else {
                                    println!("  Usage: b <line_number>");
                                }
                            } else {
                                // List breakpoints
                                if dbg.breakpoints.is_empty() {
                                    println!("  No breakpoints set.");
                                } else {
                                    let mut bps: Vec<u32> =
                                        dbg.breakpoints.iter().cloned().collect();
                                    bps.sort();
                                    for bp in bps {
                                        println!("  \x1b[32m●\x1b[0m Line {}", bp);
                                    }
                                }
                            }
                        }
                        "p" | "print" => {
                            if parts.len() > 1 {
                                let var_name = parts[1];
                                let mut found = false;
                                for scope in scopes.iter().rev() {
                                    if let Some(val) = scope.get(var_name) {
                                        println!(
                                            "  {} = {}",
                                            var_name,
                                            debugger::format_value(&val)
                                        );
                                        found = true;
                                        break;
                                    }
                                }
                                if !found {
                                    println!("  Variable '{}' not found in scope", var_name);
                                }
                            } else {
                                println!("  Usage: p <variable_name>");
                            }
                        }
                        "vars" => {
                            let vars = debugger::inspect_scopes(scopes);
                            if vars.is_empty() {
                                println!("  No user variables in scope.");
                            } else {
                                for (name, val) in &vars {
                                    println!("  {} = {}", name, val);
                                }
                            }
                        }
                        "stack" => {
                            println!("  Stack ({} items):", stack.len());
                            for (i, val) in stack.iter().enumerate().rev().take(10) {
                                println!("    [{}] {}", i, debugger::format_value(val));
                            }
                        }
                        "bt" | "backtrace" => {
                            println!("  {}", debugger::format_backtrace(ip, chunk, frame_depth));
                        }
                        "h" | "help" => {
                            println!("  Commands:");
                            println!("    n, next       Step over (next line)");
                            println!("    s, step       Step into");
                            println!("    c, continue   Continue to next breakpoint");
                            println!("    b <line>      Toggle breakpoint at line");
                            println!("    b             List all breakpoints");
                            println!("    p <var>       Print variable value");
                            println!("    vars          Print all visible variables");
                            println!("    stack         Print value stack");
                            println!("    bt            Print backtrace");
                            println!("    q, quit       Quit debugger");
                            println!("    h, help       Show this help");
                        }
                        _ => {
                            println!("  Unknown command: '{}'. Type 'h' for help.", parts[0]);
                        }
                    }
                }
            }));

            // Run with debugger
            match vm.run() {
                Ok(result) => {
                    println!();
                    println!(
                        "\x1b[1;36m[ARK DEBUGGER]\x1b[0m Program finished. Result: {}",
                        debugger::format_value(&result)
                    );
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[ARK DEBUGGER]\x1b[0m Runtime Error: {}", e);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("VM Initialization Error: {}", e);
            process::exit(1);
        }
    }
}
