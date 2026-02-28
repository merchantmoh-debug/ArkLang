/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark context overflow recovery pipeline.
 */

//! Context overflow recovery pipeline.
//!
//! Provides a 4-stage recovery pipeline that replaces brute-force
//! message trimming with structured, progressive recovery:
//!
//! 1. Auto-compact via message trimming (keep recent, drop old)
//! 2. Aggressive overflow compaction (drop all but last N)
//! 3. Truncate historical tool results
//! 4. Return error suggesting /reset or /compact
//!
//! This module operates on a generic "message" abstraction —
//! `Vec<ContextEntry>` — to avoid coupling to openfang_types.

use crate::context_budget::ContextBudget;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ContextEntry (Ark-native message abstraction)
// ---------------------------------------------------------------------------

/// Role of a context entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single entry in the agent's context window.
///
/// This is a simplified abstraction over LLM messages, designed to be
/// model-agnostic and serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub role: ContextRole,
    pub content: String,
    /// If this is a tool result, the tool use ID.
    pub tool_use_id: Option<String>,
    /// If true, this entry is a tool result.
    #[serde(default)]
    pub is_tool_result: bool,
}

impl ContextEntry {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ContextRole::User,
            content: content.into(),
            tool_use_id: None,
            is_tool_result: false,
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ContextRole::Assistant,
            content: content.into(),
            tool_use_id: None,
            is_tool_result: false,
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ContextRole::System,
            content: content.into(),
            tool_use_id: None,
            is_tool_result: false,
        }
    }

    /// Create a tool result entry.
    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: ContextRole::Tool,
            content: content.into(),
            tool_use_id: Some(tool_use_id.into()),
            is_tool_result: true,
        }
    }

    /// Estimate token count using chars/4 heuristic.
    pub fn estimated_tokens(&self) -> usize {
        self.content.len() / 4 + 1
    }
}

// ---------------------------------------------------------------------------
// RecoveryStage
// ---------------------------------------------------------------------------

/// Recovery stage that was applied.
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStage {
    /// No recovery needed.
    None,
    /// Stage 1: moderate trim (keep last 10).
    AutoCompaction { removed: usize },
    /// Stage 2: aggressive trim (keep last 4).
    OverflowCompaction { removed: usize },
    /// Stage 3: truncated tool results.
    ToolResultTruncation { truncated: usize },
    /// Stage 4: unrecoverable — suggest /reset.
    FinalError,
}

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

/// Estimate total token count for a context window.
fn estimate_total_tokens(entries: &[ContextEntry], system_prompt: &str) -> usize {
    let system_tokens = system_prompt.len() / 4 + 1;
    let entry_tokens: usize = entries.iter().map(|e| e.estimated_tokens()).sum();
    system_tokens + entry_tokens
}

// ---------------------------------------------------------------------------
// Recovery pipeline
// ---------------------------------------------------------------------------

/// Run the 4-stage overflow recovery pipeline.
///
/// Returns the recovery stage applied and modifies `entries` in-place.
pub fn recover_from_overflow(
    entries: &mut Vec<ContextEntry>,
    system_prompt: &str,
    budget: &ContextBudget,
) -> RecoveryStage {
    let estimated = estimate_total_tokens(entries, system_prompt);
    let threshold_70 = (budget.context_window_tokens as f64 * 0.70) as usize;
    let threshold_90 = (budget.context_window_tokens as f64 * 0.90) as usize;

    // No recovery needed
    if estimated <= threshold_70 {
        return RecoveryStage::None;
    }

    // Stage 1: Moderate trim — keep last 10 messages
    if estimated <= threshold_90 {
        let keep = 10.min(entries.len());
        let remove = entries.len() - keep;
        if remove > 0 {
            entries.drain(..remove);
            let new_est = estimate_total_tokens(entries, system_prompt);
            if new_est <= threshold_70 {
                return RecoveryStage::AutoCompaction { removed: remove };
            }
        }
    }

    // Stage 2: Aggressive trim — keep last 4 + summary marker
    {
        let keep = 4.min(entries.len());
        let remove = entries.len() - keep;
        if remove > 0 {
            let summary = ContextEntry::system(format!(
                "[System: {} earlier messages were removed due to context overflow. \
                 The conversation continues from here. Use /compact for smarter summarization.]",
                remove
            ));
            entries.drain(..remove);
            entries.insert(0, summary);

            let new_est = estimate_total_tokens(entries, system_prompt);
            if new_est <= threshold_90 {
                return RecoveryStage::OverflowCompaction { removed: remove };
            }
        }
    }

    // Stage 3: Truncate all historical tool results to 2K chars
    let tool_truncation_limit = 2000;
    let mut truncated = 0;
    for entry in entries.iter_mut() {
        if entry.is_tool_result && entry.content.len() > tool_truncation_limit {
            let keep = tool_truncation_limit.saturating_sub(80);
            let safe_keep = keep.min(entry.content.len());
            entry.content = format!(
                "{}\n\n[OVERFLOW RECOVERY: truncated from {} to {} chars]",
                &entry.content[..safe_keep],
                entry.content.len(),
                safe_keep
            );
            truncated += 1;
        }
    }

    if truncated > 0 {
        let new_est = estimate_total_tokens(entries, system_prompt);
        if new_est <= threshold_90 {
            return RecoveryStage::ToolResultTruncation { truncated };
        }
    }

    // Stage 4: Final error — nothing more we can do automatically
    RecoveryStage::FinalError
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entries(count: usize, size_each: usize) -> Vec<ContextEntry> {
        (0..count)
            .map(|i| {
                let text = format!("msg{}: {}", i, "x".repeat(size_each));
                if i % 2 == 0 {
                    ContextEntry::user(text)
                } else {
                    ContextEntry::assistant(text)
                }
            })
            .collect()
    }

    #[test]
    fn test_no_recovery_needed() {
        let budget = ContextBudget::new(200_000);
        let mut entries = make_entries(2, 100);
        let stage = recover_from_overflow(&mut entries, "sys", &budget);
        assert_eq!(stage, RecoveryStage::None);
    }

    #[test]
    fn test_stage1_moderate_trim() {
        let budget = ContextBudget::new(1000);
        let mut entries = make_entries(20, 150);
        let stage = recover_from_overflow(&mut entries, "system", &budget);
        match stage {
            RecoveryStage::AutoCompaction { removed } => {
                assert!(removed > 0);
                assert!(entries.len() <= 10);
            }
            RecoveryStage::OverflowCompaction { .. } => {}
            _ => {}
        }
    }

    #[test]
    fn test_stage2_aggressive_trim() {
        let budget = ContextBudget::new(1000);
        let mut entries = make_entries(30, 200);
        let stage = recover_from_overflow(&mut entries, "system", &budget);
        match stage {
            RecoveryStage::OverflowCompaction { removed } => {
                assert!(removed > 0);
            }
            RecoveryStage::ToolResultTruncation { .. } | RecoveryStage::FinalError => {}
            _ => {}
        }
    }

    #[test]
    fn test_stage3_tool_truncation() {
        let budget = ContextBudget::new(500);
        let big_result = "x".repeat(5000);
        let mut entries = vec![
            ContextEntry::user("hi"),
            ContextEntry::tool_result("t1", big_result.clone()),
            ContextEntry::tool_result("t2", big_result),
        ];
        let stage = recover_from_overflow(&mut entries, "system", &budget);
        match stage {
            RecoveryStage::ToolResultTruncation { truncated } => {
                assert!(truncated > 0);
            }
            RecoveryStage::OverflowCompaction { .. } | RecoveryStage::FinalError => {}
            _ => {}
        }
    }

    #[test]
    fn test_cascading_stages() {
        let budget = ContextBudget::new(2000);
        let mut entries = make_entries(50, 500);
        let stage = recover_from_overflow(&mut entries, "system prompt", &budget);
        assert_ne!(stage, RecoveryStage::None);
    }

    #[test]
    fn test_context_entry_constructors() {
        let user = ContextEntry::user("hello");
        assert_eq!(user.role, ContextRole::User);
        assert!(!user.is_tool_result);

        let assistant = ContextEntry::assistant("world");
        assert_eq!(assistant.role, ContextRole::Assistant);

        let system = ContextEntry::system("init");
        assert_eq!(system.role, ContextRole::System);

        let tool = ContextEntry::tool_result("t1", "result");
        assert_eq!(tool.role, ContextRole::Tool);
        assert!(tool.is_tool_result);
        assert_eq!(tool.tool_use_id.as_deref(), Some("t1"));
    }

    #[test]
    fn test_context_entry_serde() {
        let entry = ContextEntry::tool_result("abc", "hello world");
        let json = serde_json::to_string(&entry).unwrap();
        let back: ContextEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.role, ContextRole::Tool);
        assert!(back.is_tool_result);
        assert_eq!(back.tool_use_id.as_deref(), Some("abc"));
    }

    #[test]
    fn test_estimated_tokens() {
        let entry = ContextEntry::user("x".repeat(400));
        assert_eq!(entry.estimated_tokens(), 101); // 400/4 + 1
    }

    #[test]
    fn test_estimate_total_tokens() {
        let entries = vec![
            ContextEntry::user("x".repeat(100)),
            ContextEntry::assistant("y".repeat(200)),
        ];
        let total = estimate_total_tokens(&entries, "system prompt");
        // system: 13/4+1 = 4, user: 100/4+1 = 26, assistant: 200/4+1 = 51
        assert_eq!(total, 4 + 26 + 51);
    }

    #[test]
    fn test_recovery_stage_equality() {
        assert_eq!(RecoveryStage::None, RecoveryStage::None);
        assert_ne!(RecoveryStage::None, RecoveryStage::FinalError);
        assert_eq!(
            RecoveryStage::AutoCompaction { removed: 5 },
            RecoveryStage::AutoCompaction { removed: 5 }
        );
    }
}
