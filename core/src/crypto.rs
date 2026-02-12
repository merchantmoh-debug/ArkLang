/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::convert::TryInto;

pub fn hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn verify_signature(
    msg: &[u8],
    sig_bytes: &[u8],
    pubkey_bytes: &[u8],
) -> Result<bool, String> {
    if sig_bytes.len() != 64 {
        return Err(format!(
            "Invalid signature length: expected 64, got {}",
            sig_bytes.len()
        ));
    }
    if pubkey_bytes.len() != 32 {
        return Err(format!(
            "Invalid public key length: expected 32, got {}",
            pubkey_bytes.len()
        ));
    }

    let signature = Signature::from_bytes(sig_bytes.try_into().unwrap());

    // VerifyingKey::from_bytes checks for weak keys or invalid points
    let verifying_key = VerifyingKey::from_bytes(
        pubkey_bytes
            .try_into()
            .map_err(|_| "Invalid public key bytes".to_string())?,
    )
    .map_err(|e| format!("Invalid public key: {}", e))?;

    match verifying_key.verify(msg, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

pub fn merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return String::new();
    }

    // Hash leaves first
    let mut current_level: Vec<String> = leaves
        .iter()
        .map(|s| hash(s.as_bytes()))
        .collect();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for i in (0..current_level.len()).step_by(2) {
            let left = &current_level[i];
            let right = if i + 1 < current_level.len() {
                &current_level[i + 1]
            } else {
                left // Duplicate last if odd
            };

            let mut hasher = Sha256::new();
            // In the original intrinsic, it hashed the HEX strings of the previous level.
            hasher.update(left);
            hasher.update(right);
            next_level.push(hex::encode(hasher.finalize()));
        }
        current_level = next_level;
    }

    current_level[0].clone()
}
