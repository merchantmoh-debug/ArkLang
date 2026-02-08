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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ArkType {
    /// Linear: Must be used exactly once.
    /// Example: Ticket, Lock, HotPotato
    Linear(String),

    /// Affine: Can be used at most once (can be dropped).
    /// Example: FileHandle (if drop closes it)
    Affine(String),

    /// Shared: Can be used many times (Copy/Clone).
    /// Example: int, float, ReadOnlyConfig
    Shared(String),
}

impl ArkType {
    pub fn is_linear(&self) -> bool {
        matches!(self, ArkType::Linear(_))
    }
}
