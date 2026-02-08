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

use crate::ast::ArkNode;
use serde_json::from_str;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("JSON Parse Error: {0}")]
    ParseError(#[from] serde_json::Error),
}

pub fn load_ark_program(json: &str) -> Result<ArkNode, LoadError> {
    let node: ArkNode = from_str(json)?;
    Ok(node)
}
