/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark LLM driver abstraction layer.
 */

//! LLM driver types and error definitions.
//!
//! Provides the Ark-native abstraction layer for LLM interactions:
//! - `LlmError`: Comprehensive error taxonomy (HTTP, API, rate-limit, parse, overload)
//! - `CompletionRequest/Response`: Model-agnostic request/response types
//! - `StreamEvent`: Incremental streaming event types for UX
//! - `DriverConfig`: Provider connection configuration with API key redaction
//!
//! This module does NOT use `async_trait` or `tokio`. The actual `LlmDriver` trait
//! is defined at the integration boundary (e.g., in the agent runtime) where async
//! is available. This module provides only the data types.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// LlmError
// ---------------------------------------------------------------------------

/// Error type for LLM driver operations.
#[derive(Debug)]
pub enum LlmError {
    /// HTTP request failed.
    Http(String),
    /// API returned an error.
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message from the API.
        message: String,
    },
    /// Rate limited — should retry after delay.
    RateLimited {
        /// How long to wait before retrying (ms).
        retry_after_ms: u64,
    },
    /// Response parsing failed.
    Parse(String),
    /// No API key configured.
    MissingApiKey(String),
    /// Model overloaded.
    Overloaded {
        /// How long to wait before retrying (ms).
        retry_after_ms: u64,
    },
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Http(e) => write!(f, "HTTP error: {e}"),
            LlmError::Api { status, message } => write!(f, "API error ({status}): {message}"),
            LlmError::RateLimited { retry_after_ms } => {
                write!(f, "Rate limited, retry after {retry_after_ms}ms")
            }
            LlmError::Parse(e) => write!(f, "Parse error: {e}"),
            LlmError::MissingApiKey(e) => write!(f, "Missing API key: {e}"),
            LlmError::Overloaded { retry_after_ms } => {
                write!(f, "Model overloaded, retry after {retry_after_ms}ms")
            }
        }
    }
}

impl std::error::Error for LlmError {}

impl LlmError {
    /// Whether this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LlmError::Http(_) | LlmError::RateLimited { .. } | LlmError::Overloaded { .. }
        )
    }

    /// Suggested retry delay in milliseconds, if any.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            LlmError::RateLimited { retry_after_ms } => Some(*retry_after_ms),
            LlmError::Overloaded { retry_after_ms } => Some(*retry_after_ms),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Token usage
// ---------------------------------------------------------------------------

/// Token usage statistics from an LLM response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input/prompt tokens consumed.
    pub input_tokens: u64,
    /// Number of output/completion tokens generated.
    pub output_tokens: u64,
}

impl TokenUsage {
    /// Total tokens (input + output).
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

// ---------------------------------------------------------------------------
// Stop reason
// ---------------------------------------------------------------------------

/// Reason the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Model reached a natural stopping point.
    EndTurn,
    /// Hit the max_tokens limit.
    MaxTokens,
    /// Model wants to use a tool.
    ToolUse,
    /// Stop sequence encountered.
    StopSequence,
}

impl Default for StopReason {
    fn default() -> Self {
        StopReason::EndTurn
    }
}

// ---------------------------------------------------------------------------
// Content blocks
// ---------------------------------------------------------------------------

/// A content block in a completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content.
    Text { text: String },
    /// Extended thinking content.
    Thinking { thinking: String },
}

// ---------------------------------------------------------------------------
// Tool types
// ---------------------------------------------------------------------------

/// A tool/function definition for the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
}

/// A tool call extracted from a model response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool use ID for correlation.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Parsed input arguments.
    pub input: serde_json::Value,
}

// ---------------------------------------------------------------------------
// CompletionRequest / CompletionResponse
// ---------------------------------------------------------------------------

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role.
    pub role: MessageRole,
    /// Message content as text.
    pub content: String,
}

/// Role of a message sender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A request to an LLM for completion.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// Model identifier.
    pub model: String,
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// Available tools the model can use.
    pub tools: Vec<ToolDefinition>,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// System prompt (extracted from messages for APIs that need it separately).
    pub system: Option<String>,
}

/// A response from an LLM completion.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// The content blocks in the response.
    pub content: Vec<ContentBlock>,
    /// Why the model stopped generating.
    pub stop_reason: StopReason,
    /// Tool calls extracted from the response.
    pub tool_calls: Vec<ToolCall>,
    /// Token usage statistics.
    pub usage: TokenUsage,
}

impl CompletionResponse {
    /// Extract text content from the response.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                ContentBlock::Thinking { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

// ---------------------------------------------------------------------------
// StreamEvent
// ---------------------------------------------------------------------------

/// Events emitted during streaming LLM completion.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Incremental text content.
    TextDelta { text: String },
    /// A tool use block has started.
    ToolUseStart { id: String, name: String },
    /// Incremental JSON input for an in-progress tool use.
    ToolInputDelta { text: String },
    /// A tool use block is complete with parsed input.
    ToolUseEnd {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Incremental thinking/reasoning text.
    ThinkingDelta { text: String },
    /// The entire response is complete.
    ContentComplete {
        stop_reason: StopReason,
        usage: TokenUsage,
    },
    /// Agent lifecycle phase change (for UX indicators).
    PhaseChange {
        phase: String,
        detail: Option<String>,
    },
    /// Tool execution completed with result.
    ToolExecutionResult {
        name: String,
        result_preview: String,
        is_error: bool,
    },
}

// ---------------------------------------------------------------------------
// DriverConfig
// ---------------------------------------------------------------------------

/// Configuration for creating an LLM driver.
#[derive(Clone, Serialize, Deserialize)]
pub struct DriverConfig {
    /// Provider name.
    pub provider: String,
    /// API key.
    pub api_key: Option<String>,
    /// Base URL override.
    pub base_url: Option<String>,
}

/// SECURITY: Custom Debug impl redacts the API key.
impl fmt::Debug for DriverConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DriverConfig")
            .field("provider", &self.provider)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("base_url", &self.base_url)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_response_text() {
        let response = CompletionResponse {
            content: vec![
                ContentBlock::Text {
                    text: "Hello ".to_string(),
                },
                ContentBlock::Text {
                    text: "world!".to_string(),
                },
            ],
            stop_reason: StopReason::EndTurn,
            tool_calls: vec![],
            usage: TokenUsage::default(),
        };
        assert_eq!(response.text(), "Hello world!");
    }

    #[test]
    fn test_completion_response_filters_thinking() {
        let response = CompletionResponse {
            content: vec![
                ContentBlock::Thinking {
                    thinking: "Let me think...".to_string(),
                },
                ContentBlock::Text {
                    text: "Answer".to_string(),
                },
            ],
            stop_reason: StopReason::EndTurn,
            tool_calls: vec![],
            usage: TokenUsage::default(),
        };
        assert_eq!(response.text(), "Answer");
    }

    #[test]
    fn test_stream_event_clone() {
        let event = StreamEvent::TextDelta {
            text: "hello".to_string(),
        };
        let cloned = event.clone();
        assert!(matches!(cloned, StreamEvent::TextDelta { text } if text == "hello"));
    }

    #[test]
    fn test_stream_event_variants() {
        let events: Vec<StreamEvent> = vec![
            StreamEvent::TextDelta {
                text: "hi".to_string(),
            },
            StreamEvent::ToolUseStart {
                id: "t1".to_string(),
                name: "web_search".to_string(),
            },
            StreamEvent::ToolInputDelta {
                text: "{\"q".to_string(),
            },
            StreamEvent::ToolUseEnd {
                id: "t1".to_string(),
                name: "web_search".to_string(),
                input: serde_json::json!({"query": "rust"}),
            },
            StreamEvent::ContentComplete {
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
            },
        ];
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_driver_config_debug_redacts() {
        let config = DriverConfig {
            provider: "anthropic".to_string(),
            api_key: Some("sk-secret-key-123".to_string()),
            base_url: None,
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sk-secret-key-123"));
    }

    #[test]
    fn test_llm_error_display() {
        let err = LlmError::Api {
            status: 429,
            message: "Too many requests".to_string(),
        };
        assert_eq!(err.to_string(), "API error (429): Too many requests");
    }

    #[test]
    fn test_llm_error_retryable() {
        assert!(
            LlmError::RateLimited {
                retry_after_ms: 1000
            }
            .is_retryable()
        );
        assert!(
            LlmError::Overloaded {
                retry_after_ms: 5000
            }
            .is_retryable()
        );
        assert!(LlmError::Http("timeout".to_string()).is_retryable());
        assert!(!LlmError::Parse("bad json".to_string()).is_retryable());
        assert!(!LlmError::MissingApiKey("OPENAI_API_KEY".to_string()).is_retryable());
    }

    #[test]
    fn test_llm_error_retry_after() {
        assert_eq!(
            LlmError::RateLimited {
                retry_after_ms: 1000
            }
            .retry_after_ms(),
            Some(1000)
        );
        assert_eq!(LlmError::Http("err".to_string()).retry_after_ms(), None);
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
        };
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn test_token_usage_serde() {
        let usage = TokenUsage {
            input_tokens: 42,
            output_tokens: 13,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let back: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.input_tokens, 42);
        assert_eq!(back.output_tokens, 13);
    }

    #[test]
    fn test_stop_reason_serde() {
        let reason = StopReason::ToolUse;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, "\"tool_use\"");
        let back: StopReason = serde_json::from_str(&json).unwrap();
        assert_eq!(back, StopReason::ToolUse);
    }

    #[test]
    fn test_message_serde() {
        let msg = Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(back.role, MessageRole::User);
        assert_eq!(back.content, "Hello");
    }

    #[test]
    fn test_tool_definition_serde() {
        let tool = ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let back: ToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "web_search");
    }

    #[test]
    fn test_driver_config_serde() {
        let config = DriverConfig {
            provider: "openai".to_string(),
            api_key: Some("key".to_string()),
            base_url: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: DriverConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider, "openai");
        assert_eq!(back.api_key, Some("key".to_string()));
    }
}
