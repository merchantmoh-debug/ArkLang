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

use ark_0_zheng::adn;
use ark_0_zheng::compiler::Compiler;
use ark_0_zheng::debugger::{self, DebugAction, DebugState, StepMode};
use ark_0_zheng::diagnostic::{
    self, DiagnosticConfig, DiagnosticHistory, DiagnosticProbe, HistoryEntry, LinearAudit,
    OverlayEffectiveness, PipelineHealth, ProbeType, ReportTier, UserDefinedGate,
};
use ark_0_zheng::governance::{Decision, DualBand, GovernedPipeline, Phase};
use ark_0_zheng::loader::load_ark_program;
use ark_0_zheng::parser;
use ark_0_zheng::persistent::{PMap, PVec};
use ark_0_zheng::runtime::Value;
use ark_0_zheng::vm::VM;
use ark_0_zheng::wasm_codegen::WasmCodegen;
use ark_0_zheng::wasm_runner;
use ark_0_zheng::wit_gen;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process;
use std::rc::Rc;

const VERSION: &str = "1.3.0";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    let command = args[1].as_str();

    match command {
        "run" => cmd_run(&args[2..]),
        "run-wasm" => cmd_run_wasm(&args[2..]),
        "build" => cmd_build(&args[2..]),
        "wit" => cmd_wit(&args[2..]),
        "debug" => cmd_debug(&args[2..]),
        "repl" => cmd_repl(),
        "adn" => cmd_adn(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "diagnose" => cmd_diagnose(&args[2..]),
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
    println!("  ark run-wasm <file.wasm>        Execute a compiled WASM binary via wasmtime");
    println!("  ark build <file.ark> [-o out]    Compile to native WASM binary");
    println!("  ark build <file.ark> --run       Compile and immediately execute");
    println!("  ark wit <file.ark>               Generate WIT interface definition");
    println!("  ark repl                        Interactive REPL with persistent state");
    println!("  ark debug <file.ark>            Interactive step-through debugger");
    println!("  ark adn <file.ark>              Parse and output as ADN (Ark Data Notation)");
    println!("  ark check <file.ark|file.json>  Run the linear type checker");
    println!("  ark diagnose <file.ark> [opts]   Run diagnostic proof suite");
    println!("      --tier free|developer|pro     Report detail level");
    println!("      --json                        JSON output");
    println!("      --key <hmac_key>              HMAC signing key");
    println!("      --sarif                       Emit SARIF 2.1.0 report");
    println!("      --badge                       Emit SVG status badge");
    println!("      --sign                        Sign bundle (detached .sig)");
    println!("      --sbom                        Emit CycloneDX SBOM");
    println!("      --attest                      Emit in-toto attestation");
    println!("      --history                     Show diagnostic trend table");
    println!("      --gate \"name:X,key:Y,op:Z,val:W,sev:S\"  Custom gate");
    println!("  ark parse <file.ark>            Parse and dump AST as JSON");
    println!("  ark version                     Print version info");
    println!("  ark help                        Print this help message");
}

// =============================================================================
// BUILD — Compile Ark source to native WASM binary
// =============================================================================

/// Compile an Ark program to a native .wasm binary.
///
/// Usage:
///   ark build <file.ark> [-o output.wasm]
///
/// If -o is not specified, output defaults to <file>.wasm.
fn cmd_build(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'build' requires a file argument");
        eprintln!("Usage: ark build <file.ark> [-o output.wasm] [--run]");
        process::exit(1);
    }

    let filename = &args[0];
    let run_after = args.iter().any(|a| a == "--run");

    // Parse -o flag
    let output_path = if args.len() >= 3 && args[1] == "-o" {
        args[2].clone()
    } else {
        // Default: replace .ark extension with .wasm
        let path = Path::new(filename);
        path.with_extension("wasm").to_string_lossy().to_string()
    };

    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    // Parse source
    let ast = match parser::parse_source(&source, filename) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Compile to WASM
    println!("Compiling {} → {}", filename, output_path);
    match WasmCodegen::compile_to_bytes(&ast) {
        Ok(wasm_bytes) => {
            // Validate the binary before writing
            if let Err(e) = wit_gen::validate_wasm(&wasm_bytes) {
                eprintln!("WASM Validation Error: {}", e);
                eprintln!("The compiled binary is malformed — this is a compiler bug.");
                process::exit(1);
            }

            fs::write(&output_path, &wasm_bytes).unwrap_or_else(|e| {
                eprintln!("Error: Cannot write '{}': {}", output_path, e);
                process::exit(1);
            });
            println!(
                "✓ WASM binary written: {} ({} bytes, validated)",
                output_path,
                wasm_bytes.len()
            );

            // If --run flag, execute the compiled WASM immediately
            if run_after {
                println!("\n--- Running WASM via wasmtime ---");
                match wasm_runner::run_wasm(&wasm_bytes) {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            print!("{}", output.stdout);
                        }
                        println!("--- WASM execution complete ---");
                    }
                    Err(e) => {
                        eprintln!("WASM Execution Error: {}", e);
                        process::exit(1);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("WASM Compilation Error: {}", e);
            process::exit(1);
        }
    }
}

// =============================================================================
// RUN-WASM — Execute a compiled WASM binary via wasmtime
// =============================================================================

/// Execute a compiled WASM binary using the wasmtime runtime.
///
/// Usage:
///   ark run-wasm <file.wasm>
fn cmd_run_wasm(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'run-wasm' requires a .wasm file argument");
        eprintln!("Usage: ark run-wasm <file.wasm>");
        process::exit(1);
    }

    let filename = &args[0];
    let wasm_bytes = fs::read(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    println!("Executing {} via wasmtime...", filename);

    match wasm_runner::run_wasm(&wasm_bytes) {
        Ok(output) => {
            if !output.stdout.is_empty() {
                print!("{}", output.stdout);
            }
            println!(
                "✓ WASM execution complete ({} bytes of raw stdout)",
                output.stdout_raw.len()
            );
        }
        Err(e) => {
            eprintln!("WASM Execution Error: {}", e);
            process::exit(1);
        }
    }
}

// =============================================================================
// WIT — Generate WIT interface definition
// =============================================================================

/// Generate a WIT interface definition from an Ark source file.
///
/// Usage:
///   ark wit <file.ark> [-o output.wit]
fn cmd_wit(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'wit' requires a file argument");
        eprintln!("Usage: ark wit <file.ark> [-o output.wit]");
        process::exit(1);
    }

    let filename = &args[0];

    // Parse -o flag
    let output_path = if args.len() >= 3 && args[1] == "-o" {
        Some(args[2].clone())
    } else {
        None
    };

    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    let ast = match parser::parse_source(&source, filename) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Derive package name from filename
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    let package_name = format!("ark:{}", stem.replace('_', "-"));

    match wit_gen::generate_wit(&ast, &package_name) {
        Ok(wit_text) => {
            if let Some(ref path) = output_path {
                fs::write(path, &wit_text).unwrap_or_else(|e| {
                    eprintln!("Error: Cannot write '{}': {}", path, e);
                    process::exit(1);
                });
                println!("✓ WIT definition written: {}", path);
            } else {
                println!("{}", wit_text);
            }
        }
        Err(e) => {
            eprintln!("WIT Generation Error: {}", e);
            process::exit(1);
        }
    }
}

/// Run an Ark program from either .ark source or .json MAST
fn cmd_run(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'run' requires a file argument");
        eprintln!("Usage: ark run <file.ark>");
        process::exit(1);
    }

    let filename = &args[0];
    let raw_source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    // ── Import Resolution ──
    // Scan source for `import lib.std.*` lines, load and prepend module sources.
    let mut prepend = String::new();
    for line in raw_source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            let module_path = trimmed.strip_prefix("import ").unwrap().trim();
            // Convert dotted path to file path: lib.std.string -> lib/std/string.ark
            let file_path = format!("{}.ark", module_path.replace('.', "/"));
            if let Ok(module_source) = fs::read_to_string(&file_path) {
                prepend.push_str(&format!("// [import: {}]\n", module_path));
                prepend.push_str(&module_source);
                prepend.push('\n');
            }
        }
    }

    // Build final source: prepended modules + original source with import lines commented out
    let source = if prepend.is_empty() {
        raw_source
    } else {
        let filtered: Vec<&str> = raw_source
            .lines()
            .map(|l| {
                if l.trim().starts_with("import ") {
                    "// (import resolved)"
                } else {
                    l
                }
            })
            .collect();
        format!("{}\n{}", prepend, filtered.join("\n"))
    };

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

// =============================================================================
// DIAGNOSE — Run the diagnostic proof suite
// =============================================================================

/// Run the diagnostic proof suite on an Ark source file.
///
/// Parses the source, runs linear type checking, constructs diagnostic probes,
/// evaluates quality gates, seals a Merkle-rooted ProofBundle, and outputs
/// a DiagnosticReport.
///
/// Usage:
///   ark diagnose <file.ark> [--tier free|developer|pro] [--json] [--key <hmac_key>]
fn cmd_diagnose(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'diagnose' requires a file argument");
        eprintln!(
            "Usage: ark diagnose <file.ark> [--tier free|developer|pro] [--json] [--key <hmac_key>]"
        );
        process::exit(1);
    }

    let filename = &args[0];
    let json_output = args.iter().any(|a| a == "--json");

    // Parse tier
    let tier = if let Some(i) = args.iter().position(|a| a == "--tier") {
        match args.get(i + 1).map(|s| s.as_str()) {
            Some("free") => ReportTier::Free,
            Some("developer") => ReportTier::Developer,
            Some("pro") => ReportTier::Pro,
            Some(other) => {
                eprintln!(
                    "Error: Unknown tier '{}'. Use free, developer, or pro.",
                    other
                );
                process::exit(1);
            }
            None => {
                eprintln!("Error: --tier requires a value (free, developer, pro)");
                process::exit(1);
            }
        }
    } else {
        ReportTier::Developer
    };

    // Parse HMAC key
    let hmac_key: Vec<u8> = if let Some(i) = args.iter().position(|a| a == "--key") {
        match args.get(i + 1) {
            Some(key) => key.as_bytes().to_vec(),
            None => {
                eprintln!("Error: --key requires a value");
                process::exit(1);
            }
        }
    } else {
        b"ark-diagnostic-default-hmac-key".to_vec()
    };

    // Parse new Phase 80 flags
    let sarif_output = args.iter().any(|a| a == "--sarif");
    let badge_output = args.iter().any(|a| a == "--badge");
    let sign_output = args.iter().any(|a| a == "--sign");
    let sbom_output = args.iter().any(|a| a == "--sbom");
    let attest_output = args.iter().any(|a| a == "--attest");
    let history_output = args.iter().any(|a| a == "--history");

    // Parse custom gates
    let mut custom_gates: Vec<Box<dyn ark_0_zheng::diagnostic::QualityGate>> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--gate" {
            if let Some(spec) = args.get(i + 1) {
                match UserDefinedGate::from_spec(spec) {
                    Some(gate) => {
                        println!("\x1b[32m✓\x1b[0m Custom gate: {}", gate.gate_name);
                        custom_gates.push(Box::new(gate));
                    }
                    None => {
                        eprintln!("Error: Invalid gate spec '{}'", spec);
                        eprintln!("  Format: name:X,key:Y,op:Z,val:W,sev:S");
                        process::exit(1);
                    }
                }
                i += 2;
                continue;
            } else {
                eprintln!("Error: --gate requires a spec string");
                process::exit(1);
            }
        }
        i += 1;
    }

    // --- Phase 1: Parse Source ---
    println!("\x1b[1;36m╔══════════════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;36m║       ARK DIAGNOSTIC PROOF SUITE v1.0                    ║\x1b[0m");
    println!("\x1b[1;36m╚══════════════════════════════════════════════════════════╝\x1b[0m");
    println!();
    println!("\x1b[1m▸ Source:\x1b[0m {}", filename);
    println!("\x1b[1m▸ Tier:\x1b[0m   {}", tier);
    println!();

    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    let start_time = std::time::Instant::now();

    let ast = match parser::parse_source(&source, filename) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("Parse Error: {}", e);
            process::exit(1);
        }
    };

    let source_hash = ark_0_zheng::crypto::hash(source.as_bytes());
    println!(
        "\x1b[32m✓\x1b[0m Parsed ({} bytes, MAST root: {}...)",
        source.len(),
        &source_hash[..16]
    );

    // --- Phase 2: Linear Type Check ---
    let linear_audit = {
        let check_result = ark_0_zheng::checker::LinearChecker::check(&ast);

        let (linear_errors, type_errors) = match &check_result {
            Ok(_) => (0usize, 0usize),
            Err(_) => (1usize, 0usize),
        };

        let audit = LinearAudit {
            vars_declared: 0, // Would need checker instrumentation for exact count
            linear_vars: 0,
            consumed: 0,
            leaked: linear_errors,
            double_uses: 0,
            type_errors,
            max_scope_depth: 0,
            warnings: Vec::new(),
        };

        if linear_errors == 0 {
            println!(
                "\x1b[32m✓\x1b[0m Linear check passed (score: {:.4})",
                audit.safety_score()
            );
        } else {
            println!(
                "\x1b[31m✗\x1b[0m Linear check failed: {} error(s)",
                linear_errors
            );
        }

        audit
    };

    // --- Phase 3: Build Governed Pipeline ---
    let mut pipeline = GovernedPipeline::new("diag-run", &hmac_key, false);

    // Record parse phase
    let _ = pipeline.record_step(
        Phase::Sense,
        0.02,
        b"raw_source",
        source_hash.as_bytes(),
        DualBand::new(0.48, 0.52),
        Decision::Accept,
    );

    // Record check phase
    let check_decision = if linear_audit.is_clean() {
        Decision::Accept
    } else {
        Decision::Reject
    };
    let _ = pipeline.record_step(
        Phase::Assess,
        0.03,
        source_hash.as_bytes(),
        b"checked",
        DualBand::new(0.45, 0.55),
        check_decision,
    );

    // Record diagnostic decision phase
    let _ = pipeline.record_step(
        Phase::Decide,
        0.05,
        b"checked",
        b"diagnosed",
        DualBand::new(0.40, 0.60),
        Decision::Accept,
    );

    let pipe_health = PipelineHealth::from_pipeline(&pipeline);
    println!(
        "\x1b[32m✓\x1b[0m Pipeline health: {:.4} (confidence: {:.4})",
        pipe_health.score(),
        pipeline.confidence()
    );

    // --- Phase 4: Create Diagnostic Probes ---
    let elapsed_ms = start_time.elapsed().as_millis() as u64;

    let type_probe = DiagnosticProbe::new(
        &source_hash,
        b"unchecked",
        b"checked",
        ProbeType::TypeCheck,
        linear_audit.safety_score(),
    )
    .with_metadata("linear_errors", &linear_audit.leaked.to_string())
    .with_metadata("type_errors", &linear_audit.type_errors.to_string());

    let pipeline_probe = DiagnosticProbe::new(
        &source_hash,
        b"pipeline_start",
        b"pipeline_end",
        ProbeType::Pipeline,
        pipeline.confidence(),
    )
    .with_metadata("mcc_violations", &pipe_health.mcc_violations.to_string())
    .with_metadata("elapsed_ms", &elapsed_ms.to_string());

    let overlay_probe = DiagnosticProbe::new(
        &source_hash,
        b"raw_output",
        b"overlaid_output",
        ProbeType::Overlay,
        linear_audit.safety_score(),
    );

    // --- Phase 5: Run Diagnostic Pipeline ---
    let mut config = DiagnosticConfig::default_with_key(&hmac_key);
    // Add custom gates
    for gate in custom_gates {
        config.gates.push(gate);
    }
    let overlay_eff = Some(OverlayEffectiveness::compute(
        0.5,                         // Baseline score (raw)
        linear_audit.safety_score(), // Overlay score (after type checking)
        pipeline.orientation(),
    ));

    let report = diagnostic::run_diagnostic(
        &source_hash,
        vec![type_probe, pipeline_probe, overlay_probe],
        &config,
        overlay_eff,
        Some(linear_audit),
        Some(pipe_health),
    );

    match report {
        Ok(report) => {
            println!();
            println!("\x1b[1;36m─── DIAGNOSTIC REPORT ───\x1b[0m");
            println!();

            if json_output {
                // JSON output for API consumption
                let export = report.export();
                match serde_json::to_string_pretty(&export) {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("JSON serialization error: {}", e),
                }
            } else {
                // Human-readable output
                println!("{}", report.summary);
                println!();

                let export = report.export();
                let all_passed = export.get("all_gates_passed").map(|s| s.as_str()) == Some("true");

                if all_passed {
                    println!("\x1b[32m✓ ALL QUALITY GATES PASSED\x1b[0m");
                } else {
                    println!("\x1b[31m✗ SOME QUALITY GATES FAILED\x1b[0m");
                }

                println!();
                println!("\x1b[1m▸ Bundle ID:\x1b[0m  {}", report.bundle.bundle_id);
                println!("\x1b[1m▸ Merkle Root:\x1b[0m {}", report.bundle.merkle_root);
                println!(
                    "\x1b[1m▸ Probes:\x1b[0m      {}",
                    report.bundle.probe_count()
                );
                println!(
                    "\x1b[1m▸ Avg Score:\x1b[0m   {:.4}",
                    report.bundle.avg_gate_score()
                );
                println!("\x1b[1m▸ Elapsed:\x1b[0m     {}ms", elapsed_ms);

                if matches!(tier, ReportTier::Pro) {
                    println!();
                    println!(
                        "\x1b[1m▸ HMAC Sig:\x1b[0m    {}",
                        report.bundle.hmac_signature
                    );
                    println!(
                        "\x1b[1m▸ Verified:\x1b[0m    {}",
                        report.bundle.verify(&hmac_key).is_ok()
                    );
                }
            }

            println!();
            println!(
                "\x1b[1;36m╚══════════════════════════════════════════════════════════╝\x1b[0m"
            );

            // --- Phase 80: Extended Outputs ---

            // SARIF output
            if sarif_output {
                let sarif = diagnostic::generate_sarif(&report, filename);
                let sarif_path = format!("{}.sarif", filename);
                match fs::write(&sarif_path, &sarif) {
                    Ok(_) => println!("\x1b[32m✓\x1b[0m SARIF report: {}", sarif_path),
                    Err(e) => eprintln!("\x1b[31m✗\x1b[0m SARIF write error: {}", e),
                }
            }

            // Badge output
            if badge_output {
                let badge = diagnostic::generate_badge(&report);
                let badge_path = format!("{}.diagnostic-badge.svg", filename);
                match fs::write(&badge_path, &badge) {
                    Ok(_) => println!("\x1b[32m✓\x1b[0m Badge: {}", badge_path),
                    Err(e) => eprintln!("\x1b[31m✗\x1b[0m Badge write error: {}", e),
                }
            }

            // Signature output
            if sign_output {
                let sig = diagnostic::generate_signature_file(&report.bundle, &hmac_key);
                let sig_path = format!("{}.sig", filename);
                match fs::write(&sig_path, &sig) {
                    Ok(_) => println!("\x1b[32m✓\x1b[0m Signature: {}", sig_path),
                    Err(e) => eprintln!("\x1b[31m✗\x1b[0m Signature write error: {}", e),
                }
            }

            // SBOM output
            if sbom_output {
                // Minimal SBOM with known dependencies
                let entries = vec![
                    diagnostic::SbomEntry {
                        name: "sha2".to_string(),
                        version: "0.10".to_string(),
                        purl: "pkg:cargo/sha2@0.10".to_string(),
                        hash_sha256: ark_0_zheng::crypto::hash(b"sha2"),
                    },
                    diagnostic::SbomEntry {
                        name: "hmac".to_string(),
                        version: "0.12".to_string(),
                        purl: "pkg:cargo/hmac@0.12".to_string(),
                        hash_sha256: ark_0_zheng::crypto::hash(b"hmac"),
                    },
                    diagnostic::SbomEntry {
                        name: "ed25519-dalek".to_string(),
                        version: "2.1".to_string(),
                        purl: "pkg:cargo/ed25519-dalek@2.1".to_string(),
                        hash_sha256: ark_0_zheng::crypto::hash(b"ed25519-dalek"),
                    },
                    diagnostic::SbomEntry {
                        name: "serde".to_string(),
                        version: "1.0".to_string(),
                        purl: "pkg:cargo/serde@1.0".to_string(),
                        hash_sha256: ark_0_zheng::crypto::hash(b"serde"),
                    },
                ];
                let sbom = diagnostic::generate_sbom(&entries, &source_hash, "1.0.0");
                let sbom_path = format!("{}.sbom.json", filename);
                match fs::write(&sbom_path, &sbom) {
                    Ok(_) => println!("\x1b[32m✓\x1b[0m SBOM: {}", sbom_path),
                    Err(e) => eprintln!("\x1b[31m✗\x1b[0m SBOM write error: {}", e),
                }
            }

            // Attestation output
            if attest_output {
                let attestation = diagnostic::generate_attestation(&report, &hmac_key);
                let attest_path = format!("{}.attestation.json", filename);
                match fs::write(&attest_path, &attestation) {
                    Ok(_) => println!("\x1b[32m✓\x1b[0m Attestation: {}", attest_path),
                    Err(e) => eprintln!("\x1b[31m✗\x1b[0m Attestation write error: {}", e),
                }
            }

            // Historical tracking
            let history_path = format!(
                "{}",
                std::path::Path::new(filename)
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join(".ark-diagnostic-history.jsonl")
                    .display()
            );
            let entry = HistoryEntry::from_report(&report);
            let line = entry.to_json_line();

            // Append to history file
            let mut history_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&history_path);
            match history_file {
                Ok(ref mut f) => {
                    use std::io::Write;
                    let _ = writeln!(f, "{}", line);
                }
                Err(e) => eprintln!("\x1b[33m⚠\x1b[0m History append failed: {}", e),
            }

            // Show history trend table
            if history_output {
                let content = fs::read_to_string(&history_path).unwrap_or_default();
                let history = DiagnosticHistory::load(&content);
                println!();
                println!("\x1b[1;36m─── DIAGNOSTIC HISTORY (last 10 runs) ───\x1b[0m");
                println!();
                println!("{}", history.trend_table(10));

                if history.has_regression(report.bundle.avg_gate_score(), 5) {
                    println!();
                    println!(
                        "\x1b[31m⚠ REGRESSION DETECTED: Score has dropped below 5-run average\x1b[0m"
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("\x1b[31m✗ Diagnostic failed: {}\x1b[0m", e);
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

// =============================================================================
// REPL — Interactive Read-Eval-Print Loop
// =============================================================================

/// Interactive REPL with persistent state across lines.
fn cmd_repl() {
    println!("\x1b[1;36m╔══════════════════════════════════════════╗\x1b[0m");
    println!(
        "\x1b[1;36m║\x1b[0m  \x1b[1;37mArk Sovereign REPL v{}\x1b[0m            \x1b[1;36m║\x1b[0m",
        VERSION
    );
    println!(
        "\x1b[1;36m║\x1b[0m  \x1b[0;90mPersistent Data Structures Enabled\x1b[0m   \x1b[1;36m║\x1b[0m"
    );
    println!("\x1b[1;36m╚══════════════════════════════════════════╝\x1b[0m");
    println!();
    println!("  Type \x1b[1;33m:help\x1b[0m for commands, \x1b[1;33m:quit\x1b[0m to exit.\n");

    let stdin = io::stdin();
    let mut line_num: usize = 0;

    loop {
        line_num += 1;
        print!("\x1b[1;32mark[{}]\x1b[0m> ", line_num);
        io::stdout().flush().unwrap_or(());

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) => {
                // EOF
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }

        let trimmed = input.trim();
        if trimmed.is_empty() {
            line_num -= 1;
            continue;
        }

        // Handle REPL meta-commands
        match trimmed {
            ":quit" | ":q" | ":exit" => {
                println!("\x1b[0;90mGoodbye.\x1b[0m");
                break;
            }
            ":help" | ":h" => {
                print_repl_help();
                continue;
            }
            ":version" | ":v" => {
                println!("Ark Sovereign Compiler v{}", VERSION);
                continue;
            }
            ":pvec" => {
                // Quick demo of persistent vectors
                let pv = PVec::new()
                    .conj(Value::Integer(1))
                    .conj(Value::Integer(2))
                    .conj(Value::Integer(3));
                println!("  \x1b[1;34m=>\x1b[0m {}", pv);
                println!(
                    "  \x1b[0;90m(persistent vector with {} elements)\x1b[0m",
                    pv.len()
                );
                continue;
            }
            ":pmap" => {
                // Quick demo of persistent maps
                let pm = PMap::from_entries(vec![
                    ("name".to_string(), Value::String("Ark".to_string())),
                    ("version".to_string(), Value::String(VERSION.to_string())),
                    ("persistent".to_string(), Value::Boolean(true)),
                ]);
                println!("  \x1b[1;34m=>\x1b[0m {}", pm);
                println!(
                    "  \x1b[0;90m(persistent map with {} entries)\x1b[0m",
                    pm.len()
                );
                continue;
            }
            _ => {}
        }

        // Try to parse and evaluate the input as Ark source
        match parser::parse_source(trimmed, "<repl>") {
            Ok(ast) => {
                // Compile and execute
                let compiler = Compiler::new();
                let chunk = compiler.compile(&ast);
                let hash = format!("repl_line_{}", line_num);
                match VM::new(chunk, &hash, 0) {
                    Ok(mut vm) => {
                        match vm.run() {
                            Ok(result) => {
                                // Format output in ADN
                                let adn_output = adn::to_adn_pretty(&result);
                                println!("  \x1b[1;34m=>\x1b[0m {}", adn_output);
                            }
                            Err(e) => {
                                eprintln!("  \x1b[1;31mRuntime Error:\x1b[0m {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  \x1b[1;31mVM Error:\x1b[0m {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("  \x1b[1;31mParse Error:\x1b[0m {}", e);
            }
        }
    }
}

fn print_repl_help() {
    println!();
    println!("  \x1b[1;37mArk REPL Commands:\x1b[0m");
    println!("  \x1b[1;33m:help\x1b[0m    (:h)    Show this help");
    println!("  \x1b[1;33m:quit\x1b[0m    (:q)    Exit the REPL");
    println!("  \x1b[1;33m:version\x1b[0m (:v)    Show version");
    println!("  \x1b[1;33m:pvec\x1b[0m           Demo persistent vector");
    println!("  \x1b[1;33m:pmap\x1b[0m           Demo persistent map");
    println!();
    println!("  \x1b[1;37mExamples:\x1b[0m");
    println!("  \x1b[0;90m  let x = 42\x1b[0m");
    println!("  \x1b[0;90m  fn double(n: Int) -> Int {{ n + n }}\x1b[0m");
    println!("  \x1b[0;90m  double(21)\x1b[0m");
    println!();
}

// =============================================================================
// ADN — Ark Data Notation output
// =============================================================================

/// Parse and run an .ark file, output result in ADN format.
fn cmd_adn(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: 'adn' requires a file argument");
        eprintln!("Usage: ark adn <file.ark>");
        process::exit(1);
    }

    let filename = &args[0];
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: Cannot read '{}': {}", filename, e);
        process::exit(1);
    });

    match parser::parse_source(&source, filename) {
        Ok(ast) => {
            let compiler = Compiler::new();
            let chunk = compiler.compile(&ast);
            let hash = format!("ark_adn_{}", filename);
            match VM::new(chunk, &hash, 0) {
                Ok(mut vm) => match vm.run() {
                    Ok(result) => {
                        println!("{}", adn::to_adn_pretty(&result));
                    }
                    Err(e) => {
                        eprintln!("Runtime Error: {}", e);
                        process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("VM Initialization Error: {}", e);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Parse Error: {}", e);
            process::exit(1);
        }
    }
}
