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

pub mod ast;
#[cfg(feature = "ipc")]
pub mod bridge;
pub mod bytecode;
pub mod checker;
pub mod compiler;
#[cfg(test)]
pub mod eval; // Deprecated by VM, enabled for tests
pub mod ffi;
pub mod intrinsics;
pub mod loader;
// pub mod repl; // Deprecated interpreter REPL
pub mod runtime;
pub mod types;
pub mod vm;
pub mod wasm;
pub use wasm::*;
