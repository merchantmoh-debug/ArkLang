/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Engine & Agent Integration — Model Registry, Prompt Builder, Gen Config.
 *
 * Selective port of Remember-Me-AI's integrations/engine.py (ModelRegistry).
 * Ports the orchestration/metadata layer (model registry, prompt construction,
 * generation config) without model I/O deps (transformers, CUDA, torch).
 *
 * Architecture:
 *   ModelSpec       → Model metadata (id, name, description, tier, params)
 *   ModelRegistry   → HashMap registry of available models
 *   PromptBuilder   → Prompt construction with system prompt + context injection
 *   GenerationConfig→ Sampling parameters (max_tokens, temperature, top_p, etc.)
 *   EngineState     → Current engine state (selected model, query count)
 */

use std::collections::HashMap;

// ===========================================================================
// Model Specification
// ===========================================================================

/// Model tier based on parameter count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelTier {
    Tiny,   // <1B params
    Small,  // 1-3B params
    Medium, // 3-7B params
    Large,  // 7-13B params
    Huge,   // 13B+ params
}

impl ModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTier::Tiny => "tiny",
            ModelTier::Small => "small",
            ModelTier::Medium => "medium",
            ModelTier::Large => "large",
            ModelTier::Huge => "huge",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "tiny" => ModelTier::Tiny,
            "small" => ModelTier::Small,
            "medium" => ModelTier::Medium,
            "large" => ModelTier::Large,
            "huge" => ModelTier::Huge,
            _ => ModelTier::Small,
        }
    }
}

/// Specification for a registered model.
#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub key: String,
    pub model_id: String,
    pub name: String,
    pub description: String,
    pub tier: ModelTier,
    pub param_count: f64,
    pub context_window: usize,
    pub recommended: bool,
}

impl ModelSpec {
    pub fn new(key: &str, model_id: &str, name: &str, desc: &str, tier: ModelTier) -> Self {
        Self {
            key: key.to_string(),
            model_id: model_id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            tier,
            param_count: 0.0,
            context_window: 4096,
            recommended: false,
        }
    }

    pub fn with_params(mut self, count: f64) -> Self {
        self.param_count = count;
        self
    }

    pub fn with_context(mut self, window: usize) -> Self {
        self.context_window = window;
        self
    }

    pub fn with_recommended(mut self) -> Self {
        self.recommended = true;
        self
    }
}

// ===========================================================================
// Model Registry — mirrors Python ModelRegistry.MODELS
// ===========================================================================

#[derive(Debug)]
pub struct ModelRegistry {
    models: HashMap<String, ModelSpec>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            models: HashMap::new(),
        };
        reg.register_defaults();
        reg
    }

    /// Register default models (mirrors Python MODELS dict).
    fn register_defaults(&mut self) {
        self.register(
            ModelSpec::new(
                "tiny",
                "Qwen/Qwen2.5-0.5B-Instruct",
                "Qwen 2.5 (0.5B) - Tiny",
                "Ultra-fast, low memory (Run anywhere)",
                ModelTier::Tiny,
            )
            .with_params(0.5)
            .with_context(4096),
        );
        self.register(
            ModelSpec::new(
                "small",
                "Qwen/Qwen2.5-1.5B-Instruct",
                "Qwen 2.5 (1.5B) - Small",
                "Balanced speed/intelligence (Recommended)",
                ModelTier::Small,
            )
            .with_params(1.5)
            .with_context(4096)
            .with_recommended(),
        );
        self.register(
            ModelSpec::new(
                "medium",
                "HuggingFaceTB/SmolLM2-1.7B-Instruct",
                "SmolLM2 (1.7B) - Medium",
                "High reasoning capability",
                ModelTier::Medium,
            )
            .with_params(1.7)
            .with_context(8192),
        );
    }

    pub fn register(&mut self, spec: ModelSpec) {
        self.models.insert(spec.key.clone(), spec);
    }

    pub fn get(&self, key: &str) -> Option<&ModelSpec> {
        self.models.get(key)
    }

    pub fn list(&self) -> Vec<&ModelSpec> {
        let mut models: Vec<&ModelSpec> = self.models.values().collect();
        models.sort_by(|a, b| {
            a.param_count
                .partial_cmp(&b.param_count)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        models
    }

    pub fn recommended(&self) -> Option<&ModelSpec> {
        self.models.values().find(|m| m.recommended)
    }

    pub fn count(&self) -> usize {
        self.models.len()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Generation Configuration
// ===========================================================================

/// Sampling parameters for text generation.
#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub max_tokens: usize,
    pub temperature: f64,
    pub top_p: f64,
    pub do_sample: bool,
    pub repetition_penalty: f64,
}

impl GenerationConfig {
    pub fn default_config() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.9,
            do_sample: true,
            repetition_penalty: 1.0,
        }
    }

    pub fn deterministic() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.0,
            top_p: 1.0,
            do_sample: false,
            repetition_penalty: 1.0,
        }
    }

    pub fn creative() -> Self {
        Self {
            max_tokens: 512,
            temperature: 1.0,
            top_p: 0.95,
            do_sample: true,
            repetition_penalty: 1.1,
        }
    }
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

// ===========================================================================
// Prompt Builder — mirrors Python generate_response() prompt construction
// ===========================================================================

/// Chat message role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

/// A single message in a chat prompt.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Prompt builder: constructs structured prompts with context injection.
#[derive(Debug)]
pub struct PromptBuilder {
    system_prompt: String,
    messages: Vec<Message>,
}

impl PromptBuilder {
    /// Default system prompt (mirrors Python SELF-AWARENESS injection).
    pub fn new() -> Self {
        Self {
            system_prompt: concat!(
                "You are a helpful AI assistant equipped with the Ark Sovereign Cognitive Kernel. ",
                "You have long-term memory via CSNP, and access to tools like Image Generation and Web Search. ",
                "Do not deny these capabilities. If the user refers to past conversations, assume your memory context is accurate. ",
                "Answer directly and helpfully."
            ).to_string(),
            messages: Vec::new(),
        }
    }

    /// Create with a custom system prompt.
    pub fn with_system(prompt: &str) -> Self {
        Self {
            system_prompt: prompt.to_string(),
            messages: Vec::new(),
        }
    }

    /// Build the final prompt from user input + optional context string.
    pub fn build(&self, user_input: &str, context: Option<&str>) -> Vec<Message> {
        let mut messages = Vec::with_capacity(3);

        // System message
        messages.push(Message {
            role: Role::System,
            content: self.system_prompt.clone(),
        });

        // User message with context injection
        let user_content = match context {
            Some(ctx) if !ctx.is_empty() => {
                format!(
                    "\n[RELEVANT LONG-TERM MEMORY]:\n{}\n\nUSER: {}",
                    ctx, user_input
                )
            }
            _ => format!("USER: {}", user_input),
        };

        messages.push(Message {
            role: Role::User,
            content: user_content,
        });

        messages
    }

    /// Append messages from previous conversation for context.
    pub fn add_history(&mut self, role: Role, content: String) {
        self.messages.push(Message { role, content });
    }

    /// Build with history included.
    pub fn build_with_history(&self, user_input: &str, context: Option<&str>) -> Vec<Message> {
        let mut messages = Vec::with_capacity(2 + self.messages.len());

        messages.push(Message {
            role: Role::System,
            content: self.system_prompt.clone(),
        });

        // Historical messages
        for msg in &self.messages {
            messages.push(msg.clone());
        }

        // Current user message
        let user_content = match context {
            Some(ctx) if !ctx.is_empty() => {
                format!(
                    "\n[RELEVANT LONG-TERM MEMORY]:\n{}\n\nUSER: {}",
                    ctx, user_input
                )
            }
            _ => format!("USER: {}", user_input),
        };

        messages.push(Message {
            role: Role::User,
            content: user_content,
        });

        messages
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Engine State — Runtime state of the engine
// ===========================================================================

#[derive(Debug)]
pub struct EngineState {
    pub registry: ModelRegistry,
    pub config: GenerationConfig,
    pub prompt_builder: PromptBuilder,
    pub selected_model: Option<String>,
    pub total_queries: u64,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            registry: ModelRegistry::new(),
            config: GenerationConfig::default(),
            prompt_builder: PromptBuilder::new(),
            selected_model: None,
            total_queries: 0,
        }
    }

    /// Select a model by key. Returns name if found.
    pub fn select_model(&mut self, key: &str) -> Option<String> {
        if let Some(spec) = self.registry.get(key) {
            let name = spec.name.clone();
            self.selected_model = Some(key.to_string());
            Some(name)
        } else {
            None
        }
    }

    /// Build a prompt for the current configuration.
    pub fn build_prompt(&mut self, input: &str, context: Option<&str>) -> Vec<Message> {
        self.total_queries += 1;
        self.prompt_builder.build(input, context)
    }

    /// Get engine stats.
    pub fn stats(&self) -> EngineStats {
        EngineStats {
            model_count: self.registry.count(),
            selected_model: self.selected_model.clone(),
            total_queries: self.total_queries,
            config_temperature: self.config.temperature,
            config_max_tokens: self.config.max_tokens,
        }
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct EngineStats {
    pub model_count: usize,
    pub selected_model: Option<String>,
    pub total_queries: u64,
    pub config_temperature: f64,
    pub config_max_tokens: usize,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_registry_defaults() {
        let reg = ModelRegistry::new();
        assert!(reg.count() >= 3, "should have at least 3 default models");
        assert!(reg.get("tiny").is_some());
        assert!(reg.get("small").is_some());
        assert!(reg.get("medium").is_some());
    }

    #[test]
    fn test_model_registry_recommended() {
        let reg = ModelRegistry::new();
        let rec = reg.recommended().expect("should have recommended model");
        assert_eq!(rec.key, "small");
    }

    #[test]
    fn test_model_registry_list_sorted() {
        let reg = ModelRegistry::new();
        let list = reg.list();
        for i in 1..list.len() {
            assert!(
                list[i - 1].param_count <= list[i].param_count,
                "models should be sorted by param count"
            );
        }
    }

    #[test]
    fn test_model_spec_builder() {
        let spec = ModelSpec::new(
            "test",
            "org/model",
            "Test Model",
            "A test",
            ModelTier::Small,
        )
        .with_params(1.5)
        .with_context(8192)
        .with_recommended();
        assert_eq!(spec.param_count, 1.5);
        assert_eq!(spec.context_window, 8192);
        assert!(spec.recommended);
    }

    #[test]
    fn test_generation_config_defaults() {
        let cfg = GenerationConfig::default_config();
        assert_eq!(cfg.max_tokens, 256);
        assert!((cfg.temperature - 0.7).abs() < 1e-6);
        assert!(cfg.do_sample);
    }

    #[test]
    fn test_generation_config_deterministic() {
        let cfg = GenerationConfig::deterministic();
        assert!((cfg.temperature - 0.0).abs() < 1e-6);
        assert!(!cfg.do_sample);
    }

    #[test]
    fn test_prompt_builder_simple() {
        let pb = PromptBuilder::new();
        let messages = pb.build("Hello", None);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
        assert!(messages[1].content.contains("Hello"));
    }

    #[test]
    fn test_prompt_builder_with_context() {
        let pb = PromptBuilder::new();
        let messages = pb.build("What happened?", Some("Last time we discussed AI safety."));
        assert!(messages[1].content.contains("RELEVANT LONG-TERM MEMORY"));
        assert!(messages[1].content.contains("AI safety"));
    }

    #[test]
    fn test_prompt_builder_custom_system() {
        let pb = PromptBuilder::with_system("You are a pirate.");
        let messages = pb.build("Hello", None);
        assert!(messages[0].content.contains("pirate"));
    }

    #[test]
    fn test_engine_state_select_model() {
        let mut state = EngineState::new();
        let name = state.select_model("small");
        assert!(name.is_some());
        assert!(name.unwrap().contains("Qwen"));
        assert_eq!(state.selected_model, Some("small".to_string()));
    }

    #[test]
    fn test_engine_state_unknown_model() {
        let mut state = EngineState::new();
        assert!(state.select_model("nonexistent").is_none());
        assert!(state.selected_model.is_none());
    }

    #[test]
    fn test_engine_stats() {
        let mut state = EngineState::new();
        state.select_model("tiny");
        state.build_prompt("test", None);
        state.build_prompt("test2", None);
        let stats = state.stats();
        assert_eq!(stats.total_queries, 2);
        assert!(stats.selected_model.is_some());
        assert!(stats.model_count >= 3);
    }
}
