/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256, Sha512};
use std::convert::TryInto;

// ============================================================================
// STRUCTS
// ============================================================================

#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

// ============================================================================
// HASH UTILITIES
// ============================================================================

pub fn hash_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn hash_double_sha256(data: &[u8]) -> [u8; 32] {
    hash_sha256(&hash_sha256(data))
}

pub fn hash_to_hex(hash: &[u8]) -> String {
    hex::encode(hash)
}

/// Legacy compatibility function
pub fn hash(data: &[u8]) -> String {
    hash_to_hex(&hash_sha256(data))
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut res = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        res |= x ^ y;
    }
    res == 0
}

// ============================================================================
// HMAC-SHA256 (Governance Receipts)
// ============================================================================

const HMAC_SHA256_BLOCK: usize = 64;

/// HMAC-SHA256 for governance step-trace signing. Returns hex-encoded digest.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> String {
    let mut key_block = [0u8; HMAC_SHA256_BLOCK];

    if key.len() > HMAC_SHA256_BLOCK {
        let h = hash_sha256(key);
        key_block[..32].copy_from_slice(&h);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; HMAC_SHA256_BLOCK];
    let mut opad = [0x5cu8; HMAC_SHA256_BLOCK];

    for i in 0..HMAC_SHA256_BLOCK {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);

    hex::encode(outer.finalize())
}

// ============================================================================
// KEY DERIVATION (BIP-32 Style)
// ============================================================================

const HMAC_BLOCK_SIZE: usize = 128;

fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
    let mut key_block = [0u8; HMAC_BLOCK_SIZE];
    let key_len = key.len();

    if key_len > HMAC_BLOCK_SIZE {
        let mut hasher = Sha512::new();
        hasher.update(key);
        let hash = hasher.finalize();
        key_block[0..64].copy_from_slice(&hash);
    } else {
        key_block[0..key_len].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; HMAC_BLOCK_SIZE];
    let mut opad = [0x5cu8; HMAC_BLOCK_SIZE];

    for i in 0..HMAC_BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut hasher_inner = Sha512::new();
    hasher_inner.update(ipad);
    hasher_inner.update(data);
    let inner_hash = hasher_inner.finalize();

    let mut hasher_outer = Sha512::new();
    hasher_outer.update(opad);
    hasher_outer.update(inner_hash);

    hasher_outer.finalize().into()
}

pub fn derive_key(master_seed: &[u8], path: &str) -> Vec<u8> {
    // Initial State: HMAC-SHA512("Ark Master", master_seed)
    let mut state = hmac_sha512(b"Ark Master", master_seed);

    // Path: m/0/1
    let parts: Vec<&str> = path.split('/').collect();
    for part in parts {
        if part == "m" {
            continue;
        }
        if part.is_empty() {
            continue;
        }

        // Parse index, default to 0 if invalid (though caller should ensure valid path)
        let index: u32 = part.parse().unwrap_or(0);

        // Split state into Key (32) and ChainCode (32)
        // SHA-512 output is 64 bytes. We use first 32 as key, last 32 as chain code.
        let key = &state[0..32];
        let cc = &state[32..64];

        // Data = 0x00 || Key || Index(BE)
        let mut data = Vec::with_capacity(1 + 32 + 4);
        data.push(0x00);
        data.extend_from_slice(key);
        data.extend_from_slice(&index.to_be_bytes());

        state = hmac_sha512(cc, &data);
    }

    // Return private key part (first 32 bytes)
    state[0..32].to_vec()
}

// ============================================================================
// WALLET ADDRESS
// ============================================================================

pub fn generate_address(public_key: &[u8]) -> String {
    // SHA-256 of public key
    let h1 = hash_sha256(public_key);

    // RIPEMD-160 fallback -> SHA-256 again
    let h2 = hash_sha256(&h1);

    // Checksum: First 4 bytes of double-SHA-256 of h2
    let d1 = hash_sha256(&h2);
    let d2 = hash_sha256(&d1);
    let checksum = &d2[0..4];

    // Result: "ark:" + hex(h2 + checksum)
    let mut result_bytes = Vec::new();
    result_bytes.extend_from_slice(&h2);
    result_bytes.extend_from_slice(checksum);

    format!("ark:{}", hex::encode(result_bytes))
}

// ============================================================================
// SIGNING & VERIFICATION
// ============================================================================

pub fn sign_transaction(data: &[u8], private_key: &[u8]) -> SignedTransaction {
    if private_key.len() != 32 {
        panic!("Invalid private key length: expected 32");
    }

    let key_bytes: [u8; 32] = private_key.try_into().expect("operation failed");
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let signature = signing_key.sign(data);
    let verifying_key = signing_key.verifying_key();

    SignedTransaction {
        data: data.to_vec(),
        signature: signature.to_bytes().to_vec(),
        public_key: verifying_key.to_bytes().to_vec(),
    }
}

pub fn verify_transaction(tx: &SignedTransaction) -> bool {
    verify_signature(&tx.data, &tx.signature, &tx.public_key).unwrap_or(false)
}

pub fn verify_signature(msg: &[u8], sig_bytes: &[u8], pubkey_bytes: &[u8]) -> Result<bool, String> {
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

    let signature = Signature::from_bytes(sig_bytes.try_into().expect("byte array conversion"));

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

// ============================================================================
// SECURE RANDOM
// ============================================================================

#[allow(unused_variables)]
pub fn secure_random_bytes(count: usize) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::fs::File;
        use std::io::Read;
        let mut file = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
        let mut buf = vec![0u8; count];
        file.read_exact(&mut buf)
            .expect("Failed to read from /dev/urandom");
        buf
    }

    #[cfg(not(unix))]
    {
        // NO TOYS: If we can't be secure, we panic or fail.
        // For WASM or Windows without `rand` crate, we cannot guarantee security.
        panic!("Secure random not supported on this platform without `rand` crate.");
    }
}

pub fn secure_random_hex(count: usize) -> String {
    hex::encode(secure_random_bytes(count))
}

pub fn generate_nonce() -> [u8; 32] {
    let bytes = secure_random_bytes(32);
    bytes.try_into().expect("unexpected failure")
}

// ============================================================================
// MERKLE TREE (Legacy)
// ============================================================================

pub fn merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return String::new();
    }

    // Hash leaves first
    let mut current_level: Vec<String> = leaves.iter().map(|s| hash(s.as_bytes())).collect();

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

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_known_vector() {
        // SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let empty_hash = hash_sha256(b"");
        let hex_hash = hash_to_hex(&empty_hash);
        assert_eq!(
            hex_hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_double_sha256() {
        let data = b"hello";
        let h1 = hash_sha256(data);
        let h2 = hash_sha256(&h1);
        let double = hash_double_sha256(data);
        assert_eq!(h2, double);
    }

    #[test]
    fn test_constant_time_eq_same() {
        let a = b"secret";
        let b = b"secret";
        assert!(constant_time_eq(a, b));
    }

    #[test]
    fn test_constant_time_eq_different() {
        let a = b"secret";
        let b = b"secred";
        assert!(!constant_time_eq(a, b));
    }

    #[test]
    fn test_derive_key_deterministic() {
        let seed = b"test_seed";
        let k1 = derive_key(seed, "m/0/1");
        let k2 = derive_key(seed, "m/0/1");
        assert_eq!(k1, k2);

        let k3 = derive_key(seed, "m/0/2");
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_address_generation_format() {
        let pubkey = vec![0u8; 32]; // Dummy key
        let addr = generate_address(&pubkey);
        assert!(addr.starts_with("ark:"));
        // 32 bytes (SHA256) + 4 bytes (checksum) = 36 bytes -> 72 hex chars
        // "ark:" is 4 chars. Total 76.
        assert_eq!(addr.len(), 4 + 72);
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let seed = b"signing_test";
        let priv_key = derive_key(seed, "m/44/0/0");
        let data = b"transaction_data";

        let tx = sign_transaction(data, &priv_key);
        assert!(verify_transaction(&tx));

        // Tamper with data
        let mut tampered_tx = tx.clone();
        tampered_tx.data[0] ^= 0xFF;
        assert!(!verify_transaction(&tampered_tx));
    }

    #[cfg(unix)]
    #[test]
    fn test_secure_random() {
        let r1 = secure_random_bytes(32);
        let r2 = secure_random_bytes(32);
        assert_eq!(r1.len(), 32);
        assert_ne!(r1, r2); // Extremely unlikely to be equal

        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 32);
    }
}
