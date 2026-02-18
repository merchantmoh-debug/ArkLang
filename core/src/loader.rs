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

use crate::ast::{ArkNode, AstError, MastNode};
use serde_json::from_str;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("JSON Parse Error: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("AST Error: {0}")]
    AstError(#[from] AstError),
    #[error("Integrity Error: Hash Mismatch. Expected {expected}, computed {computed}.")]
    HashMismatch { expected: String, computed: String },
}

pub fn load_ark_program(json: &str) -> Result<MastNode, LoadError> {
    let node: ArkNode = from_str(json)?;
    let mast = MastNode::new(node)?;

    // The Immune System: Verify Integrity before returning
    verify_mast_integrity(&mast)?;

    Ok(mast)
}

fn verify_mast_integrity(mast: &MastNode) -> Result<(), LoadError> {
    // Compute Hash using canonical JSON (matches Python compiler and MastNode::new)
    let computed_hash = crate::ast::calculate_hash(&mast.content).map_err(LoadError::AstError)?;

    // 3. Compare
    if computed_hash != mast.hash {
        return Err(LoadError::HashMismatch {
            expected: mast.hash.clone(),
            computed: computed_hash,
        });
    }

    // 4. Recurse (if needed - currently MastNode wraps the whole tree, but if we had nested Masts, we'd check them here)
    // For now, checks top-level signature which covers the content.
    Ok(())
}
