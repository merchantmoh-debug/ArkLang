/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark agent context budget system.
 */

//! Dynamic context budget for tool result truncation.
//!
//! Replaces hardcoded `MAX_TOOL_RESULT_CHARS` with a two-layer system:
//! - **Layer 1:** Per-result cap based on context window size (30% of window)
//! - **Layer 2:** Context guard that scans all tool results before LLM calls
//!   and compacts oldest results when total exceeds 75% headroom.
//!
//! This module is model-agnostic — the context window size is injected at
//! construction time from the model catalog.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ContextBudget
// ---------------------------------------------------------------------------

/// Budget parameters derived from the model's context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    /// Total context window size in tokens.
    pub context_window_tokens: usize,
    /// Estimated characters per token for tool results (denser content).
    pub tool_chars_per_token: f64,
    /// Estimated characters per token for general content.
    pub general_chars_per_token: f64,
}

impl ContextBudget {
    /// Create a new budget from a context window size.
    pub fn new(context_window_tokens: usize) -> Self {
        Self {
            context_window_tokens,
            tool_chars_per_token: 2.0,
            general_chars_per_token: 4.0,
        }
    }

    /// Per-result character cap: 30% of context window converted to chars.
    pub fn per_result_cap(&self) -> usize {
        let tokens_for_tool = (self.context_window_tokens as f64 * 0.30) as usize;
        (tokens_for_tool as f64 * self.tool_chars_per_token) as usize
    }

    /// Single result absolute max: 50% of context window.
    pub fn single_result_max(&self) -> usize {
        let tokens = (self.context_window_tokens as f64 * 0.50) as usize;
        (tokens as f64 * self.tool_chars_per_token) as usize
    }

    /// Total tool result headroom: 75% of context window in chars.
    pub fn total_tool_headroom_chars(&self) -> usize {
        let tokens = (self.context_window_tokens as f64 * 0.75) as usize;
        (tokens as f64 * self.tool_chars_per_token) as usize
    }

    /// Estimate token count from character count using general ratio.
    pub fn estimate_tokens_from_chars(&self, chars: usize) -> usize {
        (chars as f64 / self.general_chars_per_token).ceil() as usize
    }

    /// Check if estimated token usage exceeds a percentage threshold.
    pub fn exceeds_threshold(&self, estimated_tokens: usize, threshold_pct: f64) -> bool {
        estimated_tokens > (self.context_window_tokens as f64 * threshold_pct) as usize
    }
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self::new(200_000)
    }
}

// ---------------------------------------------------------------------------
// Layer 1: Single-result truncation
// ---------------------------------------------------------------------------

/// Layer 1: Truncate a single tool result dynamically based on context budget.
///
/// Breaks at newline boundaries when possible to avoid mid-line truncation.
pub fn truncate_tool_result_dynamic(content: &str, budget: &ContextBudget) -> String {
    let cap = budget.per_result_cap();
    if content.len() <= cap {
        return content.to_string();
    }

    // Find last newline before the cap to break cleanly
    let search_start = cap.saturating_sub(200);
    let break_point = content[search_start..cap]
        .rfind('\n')
        .map(|pos| search_start + pos)
        .unwrap_or(cap.saturating_sub(100));

    format!(
        "{}\n\n[TRUNCATED: result was {} chars, showing first {} (budget: {}% of {}K context window)]",
        &content[..break_point],
        content.len(),
        break_point,
        30,
        budget.context_window_tokens / 1000
    )
}

/// Truncate content to `max_chars` with a marker.
pub fn truncate_to(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        return content.to_string();
    }
    let keep = max_chars.saturating_sub(80);
    let break_point = content[keep.saturating_sub(100)..keep]
        .rfind('\n')
        .map(|pos| keep.saturating_sub(100) + pos)
        .unwrap_or(keep);
    format!(
        "{}\n\n[COMPACTED: {} → {} chars by context guard]",
        &content[..break_point],
        content.len(),
        break_point
    )
}

// ---------------------------------------------------------------------------
// Layer 2: Batch context guard
// ---------------------------------------------------------------------------

/// A tool result entry for context guard processing.
#[derive(Debug)]
pub struct ToolResultEntry {
    /// Index into the parent collection.
    pub index: usize,
    /// The tool result content.
    pub content: String,
}

/// Layer 2: Context guard — compact tool results when total exceeds headroom.
///
/// Takes a mutable slice of tool result strings, compacts oldest first.
/// Returns the number of results compacted.
pub fn compact_tool_results(results: &mut [ToolResultEntry], budget: &ContextBudget) -> usize {
    let headroom = budget.total_tool_headroom_chars();
    let single_max = budget.single_result_max();

    let total_chars: usize = results.iter().map(|r| r.content.len()).sum();
    if total_chars <= headroom {
        return 0;
    }

    let mut current_total = total_chars;
    let mut compacted = 0;

    // Pass 1: Cap any single result > 50% of context
    for entry in results.iter_mut() {
        if entry.content.len() > single_max {
            let old_len = entry.content.len();
            entry.content = truncate_to(&entry.content, single_max);
            current_total -= old_len;
            current_total += entry.content.len();
            compacted += 1;
        }
    }

    // Pass 2: Compact oldest results until under headroom
    let compact_target = 2000;
    for entry in results.iter_mut() {
        if current_total <= headroom {
            break;
        }
        if entry.content.len() <= compact_target {
            continue;
        }
        let old_len = entry.content.len();
        entry.content = truncate_to(&entry.content, compact_target);
        current_total -= old_len;
        current_total += entry.content.len();
        compacted += 1;
    }

    compacted
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_defaults() {
        let budget = ContextBudget::default();
        assert_eq!(budget.context_window_tokens, 200_000);
        // 30% of 200K * 2.0 chars/token = 120K chars
        assert_eq!(budget.per_result_cap(), 120_000);
    }

    #[test]
    fn test_small_model_budget() {
        let budget = ContextBudget::new(8_000);
        // 30% of 8K * 2.0 = 4800 chars
        assert_eq!(budget.per_result_cap(), 4_800);
    }

    #[test]
    fn test_single_result_max() {
        let budget = ContextBudget::new(100_000);
        // 50% of 100K * 2.0 = 100K chars
        assert_eq!(budget.single_result_max(), 100_000);
    }

    #[test]
    fn test_total_headroom() {
        let budget = ContextBudget::new(200_000);
        // 75% of 200K * 2.0 = 300K chars
        assert_eq!(budget.total_tool_headroom_chars(), 300_000);
    }

    #[test]
    fn test_truncate_within_limit() {
        let budget = ContextBudget::default();
        let short = "Hello world";
        assert_eq!(truncate_tool_result_dynamic(short, &budget), short);
    }

    #[test]
    fn test_truncate_exceeds_limit() {
        let budget = ContextBudget::new(100); // cap = 60 chars
        let content =
            "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\nline12";
        let result = truncate_tool_result_dynamic(content, &budget);
        assert!(result.contains("[TRUNCATED:"));
    }

    #[test]
    fn test_truncate_to_short() {
        let short = "hello";
        assert_eq!(truncate_to(short, 100), "hello");
    }

    #[test]
    fn test_truncate_to_long() {
        let long = "x".repeat(5000);
        let result = truncate_to(&long, 2000);
        assert!(result.contains("[COMPACTED:"));
        assert!(result.len() < 5000);
    }

    #[test]
    fn test_compact_no_compaction() {
        let budget = ContextBudget::default();
        let mut results = vec![ToolResultEntry {
            index: 0,
            content: "small result".to_string(),
        }];
        let compacted = compact_tool_results(&mut results, &budget);
        assert_eq!(compacted, 0);
    }

    #[test]
    fn test_compact_oversized_results() {
        let budget = ContextBudget::new(100); // headroom = 150 chars
        let mut results = vec![
            ToolResultEntry {
                index: 0,
                content: "x".repeat(500),
            },
            ToolResultEntry {
                index: 1,
                content: "y".repeat(500),
            },
        ];
        let compacted = compact_tool_results(&mut results, &budget);
        assert!(compacted > 0);
        assert!(results[0].content.len() < 500);
    }

    #[test]
    fn test_estimate_tokens() {
        let budget = ContextBudget::default();
        // 4 chars per token
        assert_eq!(budget.estimate_tokens_from_chars(400), 100);
    }

    #[test]
    fn test_exceeds_threshold() {
        let budget = ContextBudget::new(1000);
        assert!(budget.exceeds_threshold(800, 0.70)); // 800 > 700
        assert!(!budget.exceeds_threshold(600, 0.70)); // 600 < 700
    }

    #[test]
    fn test_budget_serde_roundtrip() {
        let budget = ContextBudget::new(128_000);
        let json = serde_json::to_string(&budget).unwrap();
        let back: ContextBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(back.context_window_tokens, 128_000);
    }
}
