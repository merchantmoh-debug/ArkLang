/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark unified channel messaging system.
 */

//! Core channel bridge types for the Ark agent messaging layer.
//!
//! Provides platform-agnostic message types for routing messages between
//! agents and external channels (Telegram, Discord, Slack, etc.). The types
//! defined here form the shared vocabulary for all channel adapters.
//!
//! All types are synchronous and serializable — async transport is handled
//! by whatever runtime hosts the adapter (WASM, native, etc.).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ChannelType
// ---------------------------------------------------------------------------

/// The type of messaging channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    Telegram,
    WhatsApp,
    Slack,
    Discord,
    Signal,
    Matrix,
    Email,
    Teams,
    Mattermost,
    WebChat,
    CLI,
    /// Ark-specific: Wire P2P protocol
    Wire,
    /// Ark-specific: GCD governance channel
    Gcd,
    Custom(String),
}

impl ChannelType {
    /// Convert to lowercase string for routing/matching.
    pub fn as_str(&self) -> &str {
        match self {
            ChannelType::Telegram => "telegram",
            ChannelType::WhatsApp => "whatsapp",
            ChannelType::Slack => "slack",
            ChannelType::Discord => "discord",
            ChannelType::Signal => "signal",
            ChannelType::Matrix => "matrix",
            ChannelType::Email => "email",
            ChannelType::Teams => "teams",
            ChannelType::Mattermost => "mattermost",
            ChannelType::WebChat => "webchat",
            ChannelType::CLI => "cli",
            ChannelType::Wire => "wire",
            ChannelType::Gcd => "gcd",
            ChannelType::Custom(s) => s.as_str(),
        }
    }
}

// ---------------------------------------------------------------------------
// ChannelUser
// ---------------------------------------------------------------------------

/// A user on a messaging platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelUser {
    /// Platform-specific user ID.
    pub platform_id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Optional mapping to an Ark agent identity.
    pub ark_user: Option<String>,
}

// ---------------------------------------------------------------------------
// ChannelContent
// ---------------------------------------------------------------------------

/// Content types that can be received from a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelContent {
    Text(String),
    Image {
        url: String,
        caption: Option<String>,
    },
    File {
        url: String,
        filename: String,
    },
    Voice {
        url: String,
        duration_seconds: u32,
    },
    Location {
        lat: f64,
        lon: f64,
    },
    Command {
        name: String,
        args: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// ChannelMessage
// ---------------------------------------------------------------------------

/// A unified message from any channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Which channel this came from.
    pub channel: ChannelType,
    /// Platform-specific message identifier.
    pub platform_message_id: String,
    /// Who sent this message.
    pub sender: ChannelUser,
    /// The message content.
    pub content: ChannelContent,
    /// Optional target agent ID (if routed directly).
    pub target_agent: Option<String>,
    /// When the message was sent.
    pub timestamp: DateTime<Utc>,
    /// Whether this message is from a group chat (vs DM).
    #[serde(default)]
    pub is_group: bool,
    /// Thread ID for threaded conversations (platform-specific).
    #[serde(default)]
    pub thread_id: Option<String>,
    /// Arbitrary platform metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// AgentPhase (lifecycle UX indicators)
// ---------------------------------------------------------------------------

/// Agent lifecycle phase for UX indicators.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentPhase {
    /// Message is queued, waiting for agent.
    Queued,
    /// Agent is calling the LLM.
    Thinking,
    /// Agent is executing a tool.
    ToolUse { tool_name: String },
    /// Agent is streaming tokens.
    Streaming,
    /// Agent finished successfully.
    Done,
    /// Agent encountered an error.
    Error,
}

impl AgentPhase {
    /// Sanitize a tool name for display (truncate to 64 chars, strip control chars).
    pub fn tool_use(name: &str) -> Self {
        let sanitized: String = name.chars().filter(|c| !c.is_control()).take(64).collect();
        Self::ToolUse {
            tool_name: sanitized,
        }
    }
}

/// Reaction to show in a channel (emoji-based).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleReaction {
    pub phase: AgentPhase,
    pub emoji: String,
    pub remove_previous: bool,
}

/// Hardcoded emoji allowlist for lifecycle reactions.
pub const ALLOWED_REACTION_EMOJI: &[&str] = &[
    "\u{1F914}",        // 🤔 thinking
    "\u{2699}\u{FE0F}", // ⚙️ tool_use
    "\u{270D}\u{FE0F}", // ✍️ streaming
    "\u{2705}",         // ✅ done
    "\u{274C}",         // ❌ error
    "\u{23F3}",         // ⏳ queued
    "\u{1F504}",        // 🔄 processing
    "\u{1F440}",        // 👀 looking
];

/// Get the default emoji for a given agent phase.
pub fn default_phase_emoji(phase: &AgentPhase) -> &'static str {
    match phase {
        AgentPhase::Queued => "\u{23F3}",                 // ⏳
        AgentPhase::Thinking => "\u{1F914}",              // 🤔
        AgentPhase::ToolUse { .. } => "\u{2699}\u{FE0F}", // ⚙️
        AgentPhase::Streaming => "\u{270D}\u{FE0F}",      // ✍️
        AgentPhase::Done => "\u{2705}",                   // ✅
        AgentPhase::Error => "\u{274C}",                  // ❌
    }
}

// ---------------------------------------------------------------------------
// DeliveryReceipt
// ---------------------------------------------------------------------------

/// Delivery status for outbound messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Sent,
    Delivered,
    Failed,
    BestEffort,
}

/// Receipt tracking outbound message delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    pub message_id: String,
    pub channel: String,
    pub recipient: String,
    pub status: DeliveryStatus,
    pub timestamp: DateTime<Utc>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// ChannelStatus (health)
// ---------------------------------------------------------------------------

/// Health status for a channel adapter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelStatus {
    pub connected: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub last_error: Option<String>,
}

// ---------------------------------------------------------------------------
// OutputFormat (for formatter)
// ---------------------------------------------------------------------------

/// How to format outbound messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// Standard Markdown (passthrough).
    Markdown,
    /// Telegram HTML subset.
    TelegramHtml,
    /// Slack mrkdwn format.
    SlackMrkdwn,
    /// Plain text (all formatting stripped).
    PlainText,
}

// ---------------------------------------------------------------------------
// split_message utility
// ---------------------------------------------------------------------------

/// Split a message into chunks of at most `max_len` bytes,
/// preferring to split at newline boundaries.
pub fn split_message(text: &str, max_len: usize) -> Vec<&str> {
    if text.len() <= max_len {
        return vec![text];
    }
    let mut chunks = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining);
            break;
        }
        // Safe truncation on char boundary
        let mut safe_end = max_len;
        while safe_end > 0 && !remaining.is_char_boundary(safe_end) {
            safe_end -= 1;
        }
        let split_at = remaining[..safe_end].rfind('\n').unwrap_or(safe_end);
        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk);
        remaining = rest
            .strip_prefix("\r\n")
            .or_else(|| rest.strip_prefix('\n'))
            .unwrap_or(rest);
    }
    chunks
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_message_serialization() {
        let msg = ChannelMessage {
            channel: ChannelType::Telegram,
            platform_message_id: "123".to_string(),
            sender: ChannelUser {
                platform_id: "user1".to_string(),
                display_name: "Alice".to_string(),
                ark_user: None,
            },
            content: ChannelContent::Text("Hello!".to_string()),
            target_agent: None,
            timestamp: Utc::now(),
            is_group: false,
            thread_id: None,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&msg).expect("operation failed");
        let deserialized: ChannelMessage = serde_json::from_str(&json).expect("operation failed");
        assert_eq!(deserialized.channel, ChannelType::Telegram);
    }

    #[test]
    fn test_split_message_short() {
        assert_eq!(split_message("hello", 100), vec!["hello"]);
    }

    #[test]
    fn test_split_message_at_newlines() {
        let text = "line1\nline2\nline3";
        let chunks = split_message(text, 10);
        assert_eq!(chunks, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_channel_type_serde() {
        for ct in [
            ChannelType::Telegram,
            ChannelType::Discord,
            ChannelType::Matrix,
            ChannelType::Email,
            ChannelType::Wire,
            ChannelType::Gcd,
        ] {
            let json = serde_json::to_string(&ct).expect("operation failed");
            let back: ChannelType = serde_json::from_str(&json).expect("operation failed");
            assert_eq!(back, ct);
        }
    }

    #[test]
    fn test_channel_type_as_str() {
        assert_eq!(ChannelType::Wire.as_str(), "wire");
        assert_eq!(ChannelType::Gcd.as_str(), "gcd");
        assert_eq!(ChannelType::Telegram.as_str(), "telegram");
        assert_eq!(ChannelType::Custom("mybot".into()).as_str(), "mybot");
    }

    #[test]
    fn test_channel_content_variants() {
        let text = ChannelContent::Text("hello".to_string());
        let cmd = ChannelContent::Command {
            name: "status".into(),
            args: vec![],
        };
        let loc = ChannelContent::Location {
            lat: 40.7128,
            lon: -74.0060,
        };
        serde_json::to_string(&text).expect("operation failed");
        serde_json::to_string(&cmd).expect("operation failed");
        serde_json::to_string(&loc).expect("operation failed");
    }

    #[test]
    fn test_agent_phase_serde_roundtrip() {
        let phases = vec![
            AgentPhase::Queued,
            AgentPhase::Thinking,
            AgentPhase::tool_use("web_fetch"),
            AgentPhase::Streaming,
            AgentPhase::Done,
            AgentPhase::Error,
        ];
        for phase in &phases {
            let json = serde_json::to_string(phase).expect("operation failed");
            let back: AgentPhase = serde_json::from_str(&json).expect("operation failed");
            assert_eq!(*phase, back);
        }
    }

    #[test]
    fn test_agent_phase_tool_use_sanitizes() {
        let phase = AgentPhase::tool_use("hello\x00world\x01test");
        if let AgentPhase::ToolUse { tool_name } = phase {
            assert!(!tool_name.contains('\x00'));
            assert!(!tool_name.contains('\x01'));
            assert!(tool_name.contains("hello"));
        } else {
            panic!("Expected ToolUse variant");
        }
    }

    #[test]
    fn test_agent_phase_tool_use_truncates() {
        let long_name = "a".repeat(200);
        let phase = AgentPhase::tool_use(&long_name);
        if let AgentPhase::ToolUse { tool_name } = phase {
            assert!(tool_name.len() <= 64);
        }
    }

    #[test]
    fn test_default_phase_emoji() {
        assert_eq!(default_phase_emoji(&AgentPhase::Thinking), "\u{1F914}");
        assert_eq!(default_phase_emoji(&AgentPhase::Done), "\u{2705}");
        assert_eq!(default_phase_emoji(&AgentPhase::Error), "\u{274C}");
    }

    #[test]
    fn test_delivery_status_serde() {
        for status in [
            DeliveryStatus::Sent,
            DeliveryStatus::Delivered,
            DeliveryStatus::Failed,
            DeliveryStatus::BestEffort,
        ] {
            let json = serde_json::to_string(&status).expect("operation failed");
            let back: DeliveryStatus = serde_json::from_str(&json).expect("operation failed");
            assert_eq!(status, back);
        }
    }

    #[test]
    fn test_delivery_receipt_serde() {
        let receipt = DeliveryReceipt {
            message_id: "msg-123".to_string(),
            channel: "telegram".to_string(),
            recipient: "user-456".to_string(),
            status: DeliveryStatus::Sent,
            timestamp: Utc::now(),
            error: None,
        };
        let json = serde_json::to_string(&receipt).expect("operation failed");
        let back: DeliveryReceipt = serde_json::from_str(&json).expect("operation failed");
        assert_eq!(back.message_id, "msg-123");
        assert_eq!(back.status, DeliveryStatus::Sent);
    }

    #[test]
    fn test_output_format_serde() {
        for fmt in [
            OutputFormat::Markdown,
            OutputFormat::TelegramHtml,
            OutputFormat::SlackMrkdwn,
            OutputFormat::PlainText,
        ] {
            let json = serde_json::to_string(&fmt).expect("operation failed");
            let back: OutputFormat = serde_json::from_str(&json).expect("operation failed");
            assert_eq!(fmt, back);
        }
    }
}
