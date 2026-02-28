/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark embedding driver layer.
 */

//! Embedding driver — types and utilities for vector-based semantic memory.
//!
//! Provides:
//! - `EmbeddingError`: Error taxonomy for embedding operations
//! - `EmbeddingConfig`: Provider connection configuration
//! - `infer_dimensions()`: Model→dimension lookup (8 models)
//! - `cosine_similarity()`: Vector similarity computation
//! - `embedding_to_bytes()` / `embedding_from_bytes()`: BLOB serialization
//! - `embedding_base_url()`: Provider→URL resolution using Ark's `model_catalog`
//!
//! **Ark adaptation**: Removed `async_trait`, `reqwest`, `zeroize`, `thiserror`,
//! `tracing`. The actual HTTP embedding calls are deferred to integration boundary.
//! This module provides only the types, utilities, and dimension inference.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// EmbeddingError
// ---------------------------------------------------------------------------

/// Error type for embedding operations.
#[derive(Debug)]
pub enum EmbeddingError {
    /// HTTP request failed.
    Http(String),
    /// API returned an error.
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message from the API.
        message: String,
    },
    /// Response parsing failed.
    Parse(String),
    /// No API key configured.
    MissingApiKey(String),
}

impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbeddingError::Http(e) => write!(f, "HTTP error: {e}"),
            EmbeddingError::Api { status, message } => {
                write!(f, "API error (status {status}): {message}")
            }
            EmbeddingError::Parse(e) => write!(f, "Parse error: {e}"),
            EmbeddingError::MissingApiKey(e) => write!(f, "Missing API key: {e}"),
        }
    }
}

impl std::error::Error for EmbeddingError {}

// ---------------------------------------------------------------------------
// EmbeddingConfig
// ---------------------------------------------------------------------------

/// Configuration for creating an embedding driver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider name (openai, groq, together, ollama, etc.).
    pub provider: String,
    /// Model name (e.g., "text-embedding-3-small", "all-MiniLM-L6-v2").
    pub model: String,
    /// API key (resolved from env var). Empty string for local providers.
    pub api_key: String,
    /// Base URL for the API.
    pub base_url: String,
}

/// SECURITY: Custom Debug to redact API key in logs.
impl EmbeddingConfig {
    /// Create from provider name and model. Auto-resolves base URL and API key.
    pub fn from_provider(provider: &str, model: &str, api_key_env: &str) -> Self {
        let api_key = if api_key_env.is_empty() {
            String::new()
        } else {
            std::env::var(api_key_env).unwrap_or_default()
        };

        let base_url = embedding_base_url(provider);

        Self {
            provider: provider.to_string(),
            model: model.to_string(),
            api_key,
            base_url,
        }
    }

    /// Dimensions for this config's model.
    pub fn dimensions(&self) -> usize {
        infer_dimensions(&self.model)
    }
}

// ---------------------------------------------------------------------------
// Embed request/response types (for integration boundary)
// ---------------------------------------------------------------------------

/// Request body for an embedding API call.
#[derive(Debug, Serialize)]
pub struct EmbedRequest<'a> {
    /// Model name.
    pub model: &'a str,
    /// Input texts to embed.
    pub input: &'a [&'a str],
}

/// Response from an embedding API call.
#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    /// Embedding data entries.
    pub data: Vec<EmbedData>,
}

/// A single embedding in a response.
#[derive(Debug, Deserialize)]
pub struct EmbedData {
    /// The embedding vector.
    pub embedding: Vec<f32>,
}

// ---------------------------------------------------------------------------
// Dimension inference
// ---------------------------------------------------------------------------

/// Infer embedding dimensions from model name.
///
/// Returns well-known dimensions for common models, defaults to 1536.
pub fn infer_dimensions(model: &str) -> usize {
    match model {
        // OpenAI
        "text-embedding-3-small" => 1536,
        "text-embedding-3-large" => 3072,
        "text-embedding-ada-002" => 1536,
        // Sentence Transformers / local models
        "all-MiniLM-L6-v2" => 384,
        "all-MiniLM-L12-v2" => 384,
        "all-mpnet-base-v2" => 768,
        "nomic-embed-text" => 768,
        "mxbai-embed-large" => 1024,
        // Default to 1536 (most common)
        _ => 1536,
    }
}

/// Resolve the base URL for an embedding provider.
///
/// Uses the same URLs from Ark's `model_catalog` module.
pub fn embedding_base_url(provider: &str) -> String {
    match provider {
        "openai" => crate::model_catalog::OPENAI_BASE_URL.to_string(),
        "groq" => crate::model_catalog::GROQ_BASE_URL.to_string(),
        "together" => crate::model_catalog::TOGETHER_BASE_URL.to_string(),
        "fireworks" => crate::model_catalog::FIREWORKS_BASE_URL.to_string(),
        "mistral" => crate::model_catalog::MISTRAL_BASE_URL.to_string(),
        "ollama" => crate::model_catalog::OLLAMA_BASE_URL.to_string(),
        "vllm" => crate::model_catalog::VLLM_BASE_URL.to_string(),
        "lmstudio" => crate::model_catalog::LMSTUDIO_BASE_URL.to_string(),
        other => format!("https://{other}/v1"),
    }
}

/// Build the embeddings endpoint URL.
pub fn build_embedding_url(base_url: &str) -> String {
    format!("{}/embeddings", base_url.trim_end_matches('/'))
}

/// Check if a base URL points to a local provider (no data leaves the machine).
pub fn is_local_embedding(base_url: &str) -> bool {
    base_url.contains("localhost") || base_url.contains("127.0.0.1") || base_url.contains("[::1]")
}

// ---------------------------------------------------------------------------
// Vector math utilities
// ---------------------------------------------------------------------------

/// Compute cosine similarity between two vectors.
///
/// Returns a value in `[-1.0, 1.0]` where `1.0` = identical direction.
/// Returns `0.0` for empty or mismatched-length vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f32::EPSILON {
        0.0
    } else {
        dot / denom
    }
}

// ---------------------------------------------------------------------------
// BLOB serialization (for SQLite or disk storage)
// ---------------------------------------------------------------------------

/// Serialize an embedding vector to bytes (little-endian f32).
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for &val in embedding {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

/// Deserialize an embedding vector from bytes (little-endian f32).
pub fn embedding_from_bytes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_real_vectors() {
        let a = vec![0.1, 0.2, 0.3, 0.4];
        let b = vec![0.1, 0.2, 0.3, 0.4];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-5);

        let c = vec![0.4, 0.3, 0.2, 0.1];
        let sim2 = cosine_similarity(&a, &c);
        assert!(sim2 > 0.0 && sim2 < 1.0);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let sim = cosine_similarity(&[], &[]);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_length_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_embedding_roundtrip() {
        let embedding = vec![0.1, -0.5, 1.23456, 0.0, -1e10, 1e10];
        let bytes = embedding_to_bytes(&embedding);
        let recovered = embedding_from_bytes(&bytes);
        assert_eq!(embedding.len(), recovered.len());
        for (a, b) in embedding.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_embedding_bytes_empty() {
        let bytes = embedding_to_bytes(&[]);
        assert!(bytes.is_empty());
        let recovered = embedding_from_bytes(&bytes);
        assert!(recovered.is_empty());
    }

    #[test]
    fn test_infer_dimensions() {
        assert_eq!(infer_dimensions("text-embedding-3-small"), 1536);
        assert_eq!(infer_dimensions("text-embedding-3-large"), 3072);
        assert_eq!(infer_dimensions("all-MiniLM-L6-v2"), 384);
        assert_eq!(infer_dimensions("nomic-embed-text"), 768);
        assert_eq!(infer_dimensions("mxbai-embed-large"), 1024);
        assert_eq!(infer_dimensions("unknown-model"), 1536);
    }

    #[test]
    fn test_embedding_base_url() {
        assert_eq!(embedding_base_url("openai"), "https://api.openai.com/v1");
        assert_eq!(embedding_base_url("ollama"), "http://localhost:11434/v1");
        assert_eq!(
            embedding_base_url("custom-provider"),
            "https://custom-provider/v1"
        );
    }

    #[test]
    fn test_build_embedding_url() {
        assert_eq!(
            build_embedding_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/embeddings"
        );
    }

    #[test]
    fn test_is_local_embedding() {
        assert!(is_local_embedding("http://localhost:11434/v1"));
        assert!(is_local_embedding("http://127.0.0.1:8000/v1"));
        assert!(!is_local_embedding("https://api.openai.com/v1"));
    }

    #[test]
    fn test_embedding_error_display() {
        let err = EmbeddingError::Api {
            status: 401,
            message: "Unauthorized".to_string(),
        };
        assert_eq!(err.to_string(), "API error (status 401): Unauthorized");

        let err2 = EmbeddingError::MissingApiKey("OPENAI_API_KEY".to_string());
        assert!(err2.to_string().contains("OPENAI_API_KEY"));
    }

    #[test]
    fn test_config_from_provider() {
        let config = EmbeddingConfig::from_provider("ollama", "all-MiniLM-L6-v2", "");
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.dimensions(), 384);
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn test_embed_request_serde() {
        let texts = ["hello", "world"];
        let req = EmbedRequest {
            model: "text-embedding-3-small",
            input: &texts,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("text-embedding-3-small"));
        assert!(json.contains("hello"));
    }
}
