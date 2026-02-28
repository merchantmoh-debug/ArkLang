/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark model routing layer.
 */

//! Model routing — auto-selects cheap/mid/expensive models by query complexity.
//!
//! The router scores a `RoutingRequest` based on heuristics (token count, tool
//! availability, code markers, conversation depth) and picks the cheapest model
//! that can handle the task.
//!
//! Decoupled from `CompletionRequest` — uses a lightweight `RoutingRequest`
//! so the routing logic can be used without the full LLM driver types.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// TaskComplexity
// ---------------------------------------------------------------------------

/// Task complexity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskComplexity {
    /// Quick lookup, greetings, simple Q&A — use the cheapest model.
    Simple,
    /// Standard conversational task — use a mid-tier model.
    Medium,
    /// Multi-step reasoning, code generation, complex analysis — use the best model.
    Complex,
}

impl std::fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskComplexity::Simple => write!(f, "simple"),
            TaskComplexity::Medium => write!(f, "medium"),
            TaskComplexity::Complex => write!(f, "complex"),
        }
    }
}

// ---------------------------------------------------------------------------
// ModelRoutingConfig
// ---------------------------------------------------------------------------

/// Configuration for the model router — maps complexity tiers to model IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingConfig {
    /// Model ID for simple tasks.
    pub simple_model: String,
    /// Model ID for medium tasks.
    pub medium_model: String,
    /// Model ID for complex tasks.
    pub complex_model: String,
    /// Score threshold below which a task is classified as Simple.
    pub simple_threshold: u32,
    /// Score threshold at or above which a task is classified as Complex.
    pub complex_threshold: u32,
}

impl Default for ModelRoutingConfig {
    fn default() -> Self {
        Self {
            simple_model: "llama-3.3-70b-versatile".to_string(),
            medium_model: "claude-sonnet-4-6".to_string(),
            complex_model: "claude-opus-4-6".to_string(),
            simple_threshold: 200,
            complex_threshold: 800,
        }
    }
}

// ---------------------------------------------------------------------------
// RoutingRequest (lightweight, decoupled from CompletionRequest)
// ---------------------------------------------------------------------------

/// A lightweight routing request — carries just enough info for scoring.
#[derive(Debug, Clone)]
pub struct RoutingRequest {
    /// Content of each message in the conversation (text only).
    pub message_texts: Vec<String>,
    /// Number of tools available.
    pub tool_count: usize,
    /// System prompt, if any.
    pub system: Option<String>,
}

impl RoutingRequest {
    /// Create a routing request from raw message texts.
    pub fn new(messages: Vec<String>, tool_count: usize, system: Option<String>) -> Self {
        Self {
            message_texts: messages,
            tool_count,
            system,
        }
    }

    /// Create from a single user message (common case).
    pub fn from_user_message(text: impl Into<String>) -> Self {
        Self {
            message_texts: vec![text.into()],
            tool_count: 0,
            system: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ModelRouter
// ---------------------------------------------------------------------------

/// Code markers that suggest a request involves programming tasks.
const CODE_MARKERS: &[&str] = &[
    "```",
    "fn ",
    "def ",
    "class ",
    "import ",
    "function ",
    "async ",
    "await ",
    "struct ",
    "impl ",
    "return ",
];

/// Model router that selects the appropriate model based on query complexity.
#[derive(Debug, Clone)]
pub struct ModelRouter {
    config: ModelRoutingConfig,
}

impl ModelRouter {
    /// Create a new model router with the given routing configuration.
    pub fn new(config: ModelRoutingConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ModelRoutingConfig::default())
    }

    /// Score a routing request and determine its complexity tier.
    ///
    /// Heuristics:
    /// - **Token count**: total characters in messages / 4
    /// - **Tool availability**: having tools suggests potential multi-step work
    /// - **Code markers**: backticks, `fn`, `def`, `class`, etc.
    /// - **Conversation depth**: more messages = more context = harder reasoning
    /// - **System prompt length**: longer prompts often imply complex tasks
    pub fn score(&self, request: &RoutingRequest) -> TaskComplexity {
        let mut score: u32 = 0;

        // 1. Total message content length (rough token proxy: ~4 chars per token)
        let total_chars: usize = request.message_texts.iter().map(|m| m.len()).sum();
        let approx_tokens = (total_chars / 4) as u32;
        score += approx_tokens;

        // 2. Tool availability adds complexity
        let tool_count = request.tool_count as u32;
        if tool_count > 0 {
            score += tool_count * 20;
        }

        // 3. Code markers in the last message
        if let Some(last_msg) = request.message_texts.last() {
            let text_lower = last_msg.to_lowercase();
            let code_score: u32 = CODE_MARKERS
                .iter()
                .filter(|marker| text_lower.contains(*marker))
                .count() as u32;
            score += code_score * 30;
        }

        // 4. Conversation depth
        let msg_count = request.message_texts.len() as u32;
        if msg_count > 10 {
            score += (msg_count - 10) * 15;
        }

        // 5. System prompt complexity
        if let Some(ref system) = request.system {
            let sys_len = system.len() as u32;
            if sys_len > 500 {
                score += (sys_len - 500) / 10;
            }
        }

        // Classify
        if score < self.config.simple_threshold {
            TaskComplexity::Simple
        } else if score >= self.config.complex_threshold {
            TaskComplexity::Complex
        } else {
            TaskComplexity::Medium
        }
    }

    /// Select the model name for a given complexity tier.
    pub fn model_for_complexity(&self, complexity: TaskComplexity) -> &str {
        match complexity {
            TaskComplexity::Simple => &self.config.simple_model,
            TaskComplexity::Medium => &self.config.medium_model,
            TaskComplexity::Complex => &self.config.complex_model,
        }
    }

    /// Score a request and return the selected model name + complexity.
    pub fn select_model(&self, request: &RoutingRequest) -> (TaskComplexity, String) {
        let complexity = self.score(request);
        let model = self.model_for_complexity(complexity).to_string();
        (complexity, model)
    }

    /// Validate that all configured models exist in the catalog.
    ///
    /// Returns a list of warning messages for models not found.
    pub fn validate_models(&self, catalog: &crate::model_catalog::ModelCatalog) -> Vec<String> {
        let mut warnings = vec![];
        for model in [
            &self.config.simple_model,
            &self.config.medium_model,
            &self.config.complex_model,
        ] {
            if catalog.find_model(model).is_none() {
                warnings.push(format!("Model '{}' not found in catalog", model));
            }
        }
        warnings
    }

    /// Resolve aliases in the routing config using the catalog.
    pub fn resolve_aliases(&mut self, catalog: &crate::model_catalog::ModelCatalog) {
        if let Some(resolved) = catalog.resolve_alias(&self.config.simple_model) {
            self.config.simple_model = resolved.to_string();
        }
        if let Some(resolved) = catalog.resolve_alias(&self.config.medium_model) {
            self.config.medium_model = resolved.to_string();
        }
        if let Some(resolved) = catalog.resolve_alias(&self.config.complex_model) {
            self.config.complex_model = resolved.to_string();
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> ModelRoutingConfig {
        ModelRoutingConfig::default()
    }

    #[test]
    fn test_simple_greeting_routes_to_simple() {
        let router = ModelRouter::new(default_config());
        let request = RoutingRequest::from_user_message("Hello!");
        let (complexity, model) = router.select_model(&request);
        assert_eq!(complexity, TaskComplexity::Simple);
        assert_eq!(model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn test_code_markers_increase_complexity() {
        let router = ModelRouter::new(default_config());
        let request = RoutingRequest::from_user_message(
            "Write a function that implements async file reading with struct and impl blocks:\n\
             ```rust\nfn main() { }\n```",
        );
        let complexity = router.score(&request);
        assert_ne!(complexity, TaskComplexity::Simple);
    }

    #[test]
    fn test_tools_increase_complexity() {
        let router = ModelRouter::new(default_config());
        let request = RoutingRequest::new(
            vec!["Use the available tools to solve this problem.".to_string()],
            15, // 15 tools
            None,
        );
        let complexity = router.score(&request);
        // 15 tools * 20 = 300 — should be at least Medium
        assert_ne!(complexity, TaskComplexity::Simple);
    }

    #[test]
    fn test_long_conversation_routes_higher() {
        let router = ModelRouter::new(default_config());
        let messages: Vec<String> = (0..20)
            .map(|i| {
                format!(
                    "This is message {} with enough content to add some token weight.",
                    i
                )
            })
            .collect();
        let request = RoutingRequest::new(messages, 0, None);
        let complexity = router.score(&request);
        assert_ne!(complexity, TaskComplexity::Simple);
    }

    #[test]
    fn test_model_for_complexity() {
        let router = ModelRouter::new(default_config());
        assert_eq!(
            router.model_for_complexity(TaskComplexity::Simple),
            "llama-3.3-70b-versatile"
        );
        assert_eq!(
            router.model_for_complexity(TaskComplexity::Medium),
            "claude-sonnet-4-6"
        );
        assert_eq!(
            router.model_for_complexity(TaskComplexity::Complex),
            "claude-opus-4-6"
        );
    }

    #[test]
    fn test_complexity_display() {
        assert_eq!(TaskComplexity::Simple.to_string(), "simple");
        assert_eq!(TaskComplexity::Medium.to_string(), "medium");
        assert_eq!(TaskComplexity::Complex.to_string(), "complex");
    }

    #[test]
    fn test_validate_models_all_found() {
        let catalog = crate::model_catalog::ModelCatalog::new();
        let router = ModelRouter::new(default_config());
        let warnings = router.validate_models(&catalog);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_models_unknown() {
        let catalog = crate::model_catalog::ModelCatalog::new();
        let mut config = default_config();
        config.simple_model = "unknown-model".to_string();
        let router = ModelRouter::new(config);
        let warnings = router.validate_models(&catalog);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unknown-model"));
    }

    #[test]
    fn test_resolve_aliases() {
        let catalog = crate::model_catalog::ModelCatalog::new();
        let config = ModelRoutingConfig {
            simple_model: "llama".to_string(),
            medium_model: "sonnet".to_string(),
            complex_model: "opus".to_string(),
            simple_threshold: 200,
            complex_threshold: 800,
        };
        let mut router = ModelRouter::new(config);
        router.resolve_aliases(&catalog);
        assert_eq!(
            router.model_for_complexity(TaskComplexity::Simple),
            "llama-3.3-70b-versatile"
        );
        assert_eq!(
            router.model_for_complexity(TaskComplexity::Medium),
            "claude-sonnet-4-6"
        );
        assert_eq!(
            router.model_for_complexity(TaskComplexity::Complex),
            "claude-opus-4-6"
        );
    }

    #[test]
    fn test_system_prompt_adds_complexity() {
        let router = ModelRouter::new(default_config());
        let req_long = RoutingRequest::new(vec!["Hi".to_string()], 0, Some("A".repeat(2000)));
        let complexity_long = router.score(&req_long);

        let req_short =
            RoutingRequest::new(vec!["Hi".to_string()], 0, Some("Be helpful.".to_string()));
        let complexity_short = router.score(&req_short);

        assert!(complexity_long as u32 >= complexity_short as u32);
    }

    #[test]
    fn test_default_config() {
        let config = ModelRoutingConfig::default();
        assert_eq!(config.simple_threshold, 200);
        assert_eq!(config.complex_threshold, 800);
    }

    #[test]
    fn test_with_defaults() {
        let router = ModelRouter::with_defaults();
        assert_eq!(
            router.model_for_complexity(TaskComplexity::Simple),
            "llama-3.3-70b-versatile"
        );
    }
}
