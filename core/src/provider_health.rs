/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark provider health abstraction layer.
 */

//! Provider health probing — types and lightweight utilities for LLM provider checks.
//!
//! Provides:
//! - `ProbeResult`: health probe outcome with reachability, latency, and discovered models
//! - `is_local_provider()`: detects Ollama/vLLM/LM Studio (no key required)
//! - `build_probe_url()`: constructs the correct health endpoint URL per provider
//! - `build_completion_url()`: constructs the correct chat/completions URL
//!
//! The actual HTTP probing (`probe_provider`, `probe_model`) requires an async HTTP
//! client (reqwest) and is left to the integration boundary. This module provides
//! only the types and URL construction logic.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ProbeResult
// ---------------------------------------------------------------------------

/// Result of probing a provider endpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Whether the provider responded successfully.
    pub reachable: bool,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
    /// Model IDs discovered from the provider's listing endpoint.
    pub discovered_models: Vec<String>,
    /// Error message if the probe failed.
    pub error: Option<String>,
}

impl ProbeResult {
    /// Create a successful probe result.
    pub fn success(latency_ms: u64, models: Vec<String>) -> Self {
        Self {
            reachable: true,
            latency_ms,
            discovered_models: models,
            error: None,
        }
    }

    /// Create a failed probe result.
    pub fn failure(latency_ms: u64, error: String) -> Self {
        Self {
            reachable: false,
            latency_ms,
            discovered_models: vec![],
            error: Some(error),
        }
    }
}

// ---------------------------------------------------------------------------
// Provider classification
// ---------------------------------------------------------------------------

/// Local providers that do not require API keys.
const LOCAL_PROVIDERS: &[&str] = &["ollama", "vllm", "lmstudio"];

/// Check if a provider is a local provider (no key required, localhost URL).
///
/// Returns true for `"ollama"`, `"vllm"`, `"lmstudio"`.
pub fn is_local_provider(provider: &str) -> bool {
    LOCAL_PROVIDERS.contains(&provider.to_lowercase().as_str())
}

/// Probe timeout for local provider health checks (seconds).
pub const PROBE_TIMEOUT_SECS: u64 = 5;

// ---------------------------------------------------------------------------
// URL construction
// ---------------------------------------------------------------------------

/// Build the health/model-listing URL for a given provider.
///
/// - **Ollama**: `GET {base_url_root}/api/tags` → parses `.models[].name`
/// - **OpenAI-compat** (vLLM, LM Studio, etc.): `GET {base_url}/models` → `.data[].id`
pub fn build_probe_url(provider: &str, base_url: &str) -> (String, bool) {
    let lower = provider.to_lowercase();
    if lower == "ollama" {
        let root = base_url
            .trim_end_matches('/')
            .trim_end_matches("/v1")
            .trim_end_matches("/v1/");
        (format!("{root}/api/tags"), true)
    } else {
        let trimmed = base_url.trim_end_matches('/');
        (format!("{trimmed}/models"), false)
    }
}

/// Build the chat/completions endpoint URL for a given base URL.
pub fn build_completion_url(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

/// Detect the correct authorization header name for a provider.
///
/// - Gemini uses `x-goog-api-key`
/// - Everyone else uses `Authorization: Bearer {key}`
pub fn auth_header_for_provider(provider: &str) -> &'static str {
    if provider.to_lowercase() == "gemini" {
        "x-goog-api-key"
    } else {
        "Authorization"
    }
}

/// Format the authorization header value.
///
/// - Gemini: raw key
/// - Everyone else: `Bearer {key}`
pub fn auth_value_for_provider(provider: &str, api_key: &str) -> String {
    if provider.to_lowercase() == "gemini" {
        api_key.to_string()
    } else {
        format!("Bearer {api_key}")
    }
}

// ---------------------------------------------------------------------------
// JSON parsing helpers (for use with probe responses)
// ---------------------------------------------------------------------------

/// Parse model IDs from an Ollama `/api/tags` response body.
pub fn parse_ollama_models(body: &serde_json::Value) -> Vec<String> {
    body.get("models")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse model IDs from an OpenAI-compatible `/models` response body.
pub fn parse_openai_models(body: &serde_json::Value) -> Vec<String> {
    body.get("data")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_local_provider_true() {
        assert!(is_local_provider("ollama"));
        assert!(is_local_provider("Ollama"));
        assert!(is_local_provider("OLLAMA"));
        assert!(is_local_provider("vllm"));
        assert!(is_local_provider("lmstudio"));
    }

    #[test]
    fn test_is_local_provider_false() {
        assert!(!is_local_provider("openai"));
        assert!(!is_local_provider("anthropic"));
        assert!(!is_local_provider("gemini"));
        assert!(!is_local_provider("groq"));
    }

    #[test]
    fn test_probe_result_default() {
        let result = ProbeResult::default();
        assert!(!result.reachable);
        assert_eq!(result.latency_ms, 0);
        assert!(result.discovered_models.is_empty());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_probe_result_success() {
        let result = ProbeResult::success(42, vec!["llama3.2".to_string()]);
        assert!(result.reachable);
        assert_eq!(result.latency_ms, 42);
        assert_eq!(result.discovered_models.len(), 1);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_probe_result_failure() {
        let result = ProbeResult::failure(100, "connection refused".to_string());
        assert!(!result.reachable);
        assert_eq!(result.latency_ms, 100);
        assert!(result.discovered_models.is_empty());
        assert_eq!(result.error.as_deref(), Some("connection refused"));
    }

    #[test]
    fn test_probe_timeout_value() {
        assert_eq!(PROBE_TIMEOUT_SECS, 5);
    }

    #[test]
    fn test_build_probe_url_ollama() {
        let (url, is_ollama) = build_probe_url("ollama", "http://localhost:11434/v1");
        assert_eq!(url, "http://localhost:11434/api/tags");
        assert!(is_ollama);
    }

    #[test]
    fn test_build_probe_url_openai_compat() {
        let (url, is_ollama) = build_probe_url("vllm", "http://localhost:8000/v1");
        assert_eq!(url, "http://localhost:8000/v1/models");
        assert!(!is_ollama);
    }

    #[test]
    fn test_build_completion_url() {
        let url = build_completion_url("http://localhost:8000/v1");
        assert_eq!(url, "http://localhost:8000/v1/chat/completions");

        let url2 = build_completion_url("http://localhost:8000/v1/");
        assert_eq!(url2, "http://localhost:8000/v1/chat/completions");
    }

    #[test]
    fn test_auth_header_gemini() {
        assert_eq!(auth_header_for_provider("gemini"), "x-goog-api-key");
        assert_eq!(auth_header_for_provider("Gemini"), "x-goog-api-key");
    }

    #[test]
    fn test_auth_header_bearer() {
        assert_eq!(auth_header_for_provider("openai"), "Authorization");
        assert_eq!(auth_header_for_provider("anthropic"), "Authorization");
    }

    #[test]
    fn test_auth_value_gemini() {
        let val = auth_value_for_provider("gemini", "my-key");
        assert_eq!(val, "my-key");
    }

    #[test]
    fn test_auth_value_bearer() {
        let val = auth_value_for_provider("openai", "sk-123");
        assert_eq!(val, "Bearer sk-123");
    }

    #[test]
    fn test_parse_ollama_models() {
        let body = serde_json::json!({
            "models": [
                {"name": "llama3.2:latest"},
                {"name": "mistral:latest"}
            ]
        });
        let models = parse_ollama_models(&body);
        assert_eq!(models, vec!["llama3.2:latest", "mistral:latest"]);
    }

    #[test]
    fn test_parse_openai_models() {
        let body = serde_json::json!({
            "data": [
                {"id": "gpt-4o"},
                {"id": "gpt-3.5-turbo"}
            ]
        });
        let models = parse_openai_models(&body);
        assert_eq!(models, vec!["gpt-4o", "gpt-3.5-turbo"]);
    }

    #[test]
    fn test_parse_empty_body() {
        let empty = serde_json::json!({});
        assert!(parse_ollama_models(&empty).is_empty());
        assert!(parse_openai_models(&empty).is_empty());
    }

    #[test]
    fn test_probe_result_serde() {
        let result = ProbeResult::success(55, vec!["model-a".to_string()]);
        let json = serde_json::to_string(&result).expect("operation failed");
        let back: ProbeResult = serde_json::from_str(&json).expect("operation failed");
        assert!(back.reachable);
        assert_eq!(back.latency_ms, 55);
        assert_eq!(back.discovered_models.len(), 1);
    }
}
