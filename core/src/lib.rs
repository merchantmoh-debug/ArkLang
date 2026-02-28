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

#![allow(unexpected_cfgs)]

pub mod a2a;
pub mod adn;
pub mod approval;
pub mod ast;
pub mod audit;
pub mod capability;
pub mod channel_formatter;
pub mod channel_router;
pub mod channel_types;
pub mod context_budget;
pub mod context_overflow;

pub mod blockchain;
#[cfg(feature = "ipc")]
#[allow(dead_code)]
pub mod bridge;
pub mod bytecode;
pub mod checker;
pub mod compiler;
pub mod consensus;
pub mod crypto;
pub mod debugger;
pub mod diagnostic;
pub mod embedding;
#[cfg(test)]
pub mod eval; // Deprecated by VM, enabled for tests
pub mod ffi;
pub mod governance;
pub mod graceful_shutdown;
pub mod hooks;
pub mod intrinsics;
pub mod kernel_handle;
pub mod llm_driver;
pub mod loader;
pub mod loop_guard;
pub mod macros;
pub mod manifest_signing;
pub mod metering;
pub mod model_catalog;
pub mod parser;
pub mod persistent;
pub mod provider_health;
// pub mod repl; // Deprecated interpreter REPL
#[cfg(test)]
pub mod bench_intrinsics;
pub mod retry;
pub mod routing;
pub mod runtime;
pub mod semantic_memory;
pub mod shell_bleed;
#[cfg(test)]
pub mod snapshot_tests;
pub mod taint;
pub mod tool_policy;
pub mod triggers;
pub mod types;
pub mod vm;
pub mod wasm;
pub mod wasm_codegen;
#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_host_imports;
#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_interop;
#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_runner;
pub mod wit_gen;
pub use wasm::*;
