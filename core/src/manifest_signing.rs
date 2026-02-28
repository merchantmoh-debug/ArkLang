/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for Ark's agent manifest supply chain integrity.
 */

//! Ed25519 manifest signing for agent supply chain integrity.
//!
//! Agent manifests (HAND.toml, agent.json, etc.) define capabilities and
//! permissions. A tampered manifest = privilege escalation. This module
//! uses SHA-256 content hashing + Ed25519 digital signatures to create
//! a tamper-evident envelope around agent manifests.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A signed manifest envelope containing the original content,
/// the Ed25519 signature, and the signer's public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedManifest {
    /// The raw manifest content (e.g., TOML, JSON).
    pub content: String,
    /// SHA-256 hash of the content (hex-encoded).
    pub content_hash: String,
    /// Ed25519 signature of the content hash (hex-encoded).
    pub signature: String,
    /// The signer's Ed25519 public key (hex-encoded).
    pub public_key: String,
}

/// Error type for manifest signing operations.
#[derive(Debug, Clone)]
pub enum ManifestSignError {
    /// The Ed25519 signature does not match the content hash.
    InvalidSignature(String),
    /// The content hash does not match the actual content.
    ContentTampered { expected: String, actual: String },
    /// Failed to decode hex-encoded key or signature.
    DecodingError(String),
}

impl std::fmt::Display for ManifestSignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestSignError::InvalidSignature(msg) => {
                write!(f, "Invalid manifest signature: {}", msg)
            }
            ManifestSignError::ContentTampered { expected, actual } => {
                write!(
                    f,
                    "Manifest content tampered: expected hash {}, got {}",
                    expected, actual
                )
            }
            ManifestSignError::DecodingError(msg) => {
                write!(f, "Manifest decoding error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ManifestSignError {}

/// Compute the SHA-256 hash of content, returned as hex string.
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Sign a manifest with the given Ed25519 signing key.
///
/// Returns a `SignedManifest` envelope containing the content, hash, signature,
/// and the signer's public key.
pub fn sign_manifest(content: &str, signing_key: &SigningKey) -> SignedManifest {
    let content_hash = hash_content(content);
    let signature = signing_key.sign(content_hash.as_bytes());
    let public_key = signing_key.verifying_key();

    SignedManifest {
        content: content.to_string(),
        content_hash,
        signature: hex::encode(signature.to_bytes()),
        public_key: hex::encode(public_key.to_bytes()),
    }
}

/// Verify a signed manifest's integrity and authenticity.
///
/// Checks:
/// 1. Content hash matches the actual content (tamper detection)
/// 2. Ed25519 signature matches the content hash (authenticity)
pub fn verify_manifest(manifest: &SignedManifest) -> Result<(), ManifestSignError> {
    // Step 1: Verify content hash
    let actual_hash = hash_content(&manifest.content);
    if actual_hash != manifest.content_hash {
        return Err(ManifestSignError::ContentTampered {
            expected: manifest.content_hash.clone(),
            actual: actual_hash,
        });
    }

    // Step 2: Decode public key
    let pub_key_bytes: [u8; 32] = hex::decode(&manifest.public_key)
        .map_err(|e| ManifestSignError::DecodingError(format!("public key: {}", e)))?
        .try_into()
        .map_err(|_| ManifestSignError::DecodingError("public key: wrong length".to_string()))?;

    let verifying_key = VerifyingKey::from_bytes(&pub_key_bytes)
        .map_err(|e| ManifestSignError::DecodingError(format!("public key: {}", e)))?;

    // Step 3: Decode signature
    let sig_bytes: [u8; 64] = hex::decode(&manifest.signature)
        .map_err(|e| ManifestSignError::DecodingError(format!("signature: {}", e)))?
        .try_into()
        .map_err(|_| ManifestSignError::DecodingError("signature: wrong length".to_string()))?;

    let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);

    // Step 4: Verify signature
    verifying_key
        .verify(manifest.content_hash.as_bytes(), &signature)
        .map_err(|e| ManifestSignError::InvalidSignature(e.to_string()))
}

/// Verify that a manifest was signed by a specific public key.
///
/// This is used to check that an agent manifest was signed by the
/// expected authority (e.g., the Sovereign Systems root key).
pub fn verify_manifest_by_key(
    manifest: &SignedManifest,
    expected_public_key: &str,
) -> Result<(), ManifestSignError> {
    if manifest.public_key != expected_public_key {
        return Err(ManifestSignError::InvalidSignature(format!(
            "Manifest signed by {}, expected {}",
            manifest.public_key, expected_public_key
        )));
    }
    verify_manifest(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn test_keypair() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    #[test]
    fn test_sign_and_verify() {
        let key = test_keypair();
        let content = r#"
[agent]
name = "test-agent"
capabilities = ["file_read", "net_connect"]
"#;
        let signed = sign_manifest(content, &key);
        assert!(verify_manifest(&signed).is_ok());
    }

    #[test]
    fn test_tampered_content_detected() {
        let key = test_keypair();
        let signed = sign_manifest("original content", &key);

        let mut tampered = signed.clone();
        tampered.content = "tampered content".to_string();

        let result = verify_manifest(&tampered);
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestSignError::ContentTampered { .. } => {}
            other => panic!("Expected ContentTampered, got: {:?}", other),
        }
    }

    #[test]
    fn test_wrong_signature_rejected() {
        let key1 = test_keypair();
        let key2 = test_keypair();

        let signed = sign_manifest("content", &key1);

        // Create a new signed manifest with key2's signature but key1's public key
        let mut forged = signed.clone();
        let forged_sig = key2.sign(signed.content_hash.as_bytes());
        forged.signature = hex::encode(forged_sig.to_bytes());

        let result = verify_manifest(&forged);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_by_specific_key() {
        let key = test_keypair();
        let expected_pk = hex::encode(key.verifying_key().to_bytes());
        let signed = sign_manifest("content", &key);

        assert!(verify_manifest_by_key(&signed, &expected_pk).is_ok());

        let wrong_key = test_keypair();
        let wrong_pk = hex::encode(wrong_key.verifying_key().to_bytes());
        assert!(verify_manifest_by_key(&signed, &wrong_pk).is_err());
    }

    #[test]
    fn test_hash_deterministic() {
        let h1 = hash_content("test");
        let h2 = hash_content("test");
        assert_eq!(h1, h2);
        assert_ne!(hash_content("test"), hash_content("test2"));
    }

    #[test]
    fn test_empty_content_signs() {
        let key = test_keypair();
        let signed = sign_manifest("", &key);
        assert!(verify_manifest(&signed).is_ok());
    }

    #[test]
    fn test_large_content_signs() {
        let key = test_keypair();
        let large = "x".repeat(100_000);
        let signed = sign_manifest(&large, &key);
        assert!(verify_manifest(&signed).is_ok());
    }
}
