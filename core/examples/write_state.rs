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

#[cfg(feature = "ipc")]
use ark_0_zheng::bridge::write_dummy_state;

#[cfg(feature = "ipc")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Ark-0 (Zheng): Writing 'ark_state.arrow' via Zero-Copy Bridge...");
    // Write to meta directory so python can find it easily
    write_dummy_state("../meta/ark_state.arrow")?;
    println!("Ark-0 (Zheng): Write Complete.");
    Ok(())
}

#[cfg(not(feature = "ipc"))]
fn main() {
    println!("IPC feature disabled. Skipping write_state example.");
}
