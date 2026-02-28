/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark plugin lifecycle hooks layer.
 */

//! Plugin lifecycle hooks — intercept points at key moments in agent execution.
//!
//! Provides a callback-based hook system (not dynamic loading) for safe
//! extensibility. Four hook types:
//! - `BeforeToolCall`: Fires before tool execution. Can block the call by returning Err.
//! - `AfterToolCall`: Fires after tool execution. Observe-only.
//! - `BeforePromptBuild`: Fires before system prompt construction. Observe-only.
//! - `AgentLoopEnd`: Fires after the agent loop completes. Observe-only.
//!
//! **Ark adaptation**: Removed `dashmap` (replaced with `std::sync::Mutex<HashMap>`),
//! inlined `HookEvent` enum (removed `openfang_types::agent::HookEvent` dep),
//! removed `tracing` (replaced with eprintln for error logging).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// HookEvent (inlined from openfang_types::agent)
// ---------------------------------------------------------------------------

/// Hook event types — identifies when a hook fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    /// Before a tool is called. Handlers can block execution.
    BeforeToolCall,
    /// After a tool call completes. Observe-only.
    AfterToolCall,
    /// Before system prompt is constructed. Observe-only.
    BeforePromptBuild,
    /// After the agent loop completes. Observe-only.
    AgentLoopEnd,
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookEvent::BeforeToolCall => write!(f, "before_tool_call"),
            HookEvent::AfterToolCall => write!(f, "after_tool_call"),
            HookEvent::BeforePromptBuild => write!(f, "before_prompt_build"),
            HookEvent::AgentLoopEnd => write!(f, "agent_loop_end"),
        }
    }
}

// ---------------------------------------------------------------------------
// HookContext
// ---------------------------------------------------------------------------

/// Context passed to hook handlers.
pub struct HookContext<'a> {
    /// Agent display name.
    pub agent_name: &'a str,
    /// Agent ID string.
    pub agent_id: &'a str,
    /// Which hook event triggered this call.
    pub event: HookEvent,
    /// Event-specific payload (tool name, input, result, etc.).
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// HookHandler trait
// ---------------------------------------------------------------------------

/// Hook handler trait. Implementations must be thread-safe.
pub trait HookHandler: Send + Sync {
    /// Called when the hook fires.
    ///
    /// For `BeforeToolCall`: returning `Err(reason)` blocks the tool call.
    /// For all other events: return value is ignored (observe-only).
    fn on_event(&self, ctx: &HookContext) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// HookRegistry
// ---------------------------------------------------------------------------

/// Registry of hook handlers, keyed by event type.
///
/// Thread-safe via `Mutex<HashMap>`. Handlers fire in registration order.
pub struct HookRegistry {
    handlers: Mutex<HashMap<HookEvent, Vec<Arc<dyn HookHandler>>>>,
}

impl HookRegistry {
    /// Create an empty hook registry.
    pub fn new() -> Self {
        Self {
            handlers: Mutex::new(HashMap::new()),
        }
    }

    /// Register a handler for a specific event type.
    pub fn register(&self, event: HookEvent, handler: Arc<dyn HookHandler>) {
        let mut map = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(event).or_default().push(handler);
    }

    /// Fire all handlers for an event. Returns Err if any handler blocks.
    ///
    /// For `BeforeToolCall`, the first Err stops execution and returns the reason.
    /// For other events, errors are logged but don't propagate.
    pub fn fire(&self, ctx: &HookContext) -> Result<(), String> {
        let map = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(handlers) = map.get(&ctx.event) {
            for handler in handlers.iter() {
                if let Err(reason) = handler.on_event(ctx) {
                    if ctx.event == HookEvent::BeforeToolCall {
                        return Err(reason);
                    }
                    // For non-blocking hooks, log and continue
                    eprintln!(
                        "[hooks] {:?} handler error for agent '{}': {}",
                        ctx.event, ctx.agent_name, reason
                    );
                }
            }
        }
        Ok(())
    }

    /// Check if any handlers are registered for a given event.
    pub fn has_handlers(&self, event: HookEvent) -> bool {
        let map = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        map.get(&event).map(|v| !v.is_empty()).unwrap_or(false)
    }

    /// Count of registered handlers across all events.
    pub fn handler_count(&self) -> usize {
        let map = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        map.values().map(|v| v.len()).sum()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A test handler that always succeeds.
    struct OkHandler;
    impl HookHandler for OkHandler {
        fn on_event(&self, _ctx: &HookContext) -> Result<(), String> {
            Ok(())
        }
    }

    /// A test handler that always blocks.
    struct BlockHandler {
        reason: String,
    }
    impl HookHandler for BlockHandler {
        fn on_event(&self, _ctx: &HookContext) -> Result<(), String> {
            Err(self.reason.clone())
        }
    }

    /// A test handler that records calls.
    struct RecordHandler {
        calls: Mutex<Vec<String>>,
    }
    impl RecordHandler {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }
    impl HookHandler for RecordHandler {
        fn on_event(&self, ctx: &HookContext) -> Result<(), String> {
            self.calls.lock().unwrap().push(format!("{:?}", ctx.event));
            Ok(())
        }
    }

    fn make_ctx(event: HookEvent) -> HookContext<'static> {
        HookContext {
            agent_name: "test-agent",
            agent_id: "abc-123",
            event,
            data: serde_json::json!({}),
        }
    }

    #[test]
    fn test_empty_registry_is_noop() {
        let registry = HookRegistry::new();
        let ctx = make_ctx(HookEvent::BeforeToolCall);
        assert!(registry.fire(&ctx).is_ok());
    }

    #[test]
    fn test_before_tool_call_can_block() {
        let registry = HookRegistry::new();
        registry.register(
            HookEvent::BeforeToolCall,
            Arc::new(BlockHandler {
                reason: "Not allowed".to_string(),
            }),
        );
        let ctx = make_ctx(HookEvent::BeforeToolCall);
        let result = registry.fire(&ctx);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not allowed");
    }

    #[test]
    fn test_after_tool_call_receives_result() {
        let recorder = Arc::new(RecordHandler::new());
        let registry = HookRegistry::new();
        registry.register(HookEvent::AfterToolCall, recorder.clone());

        let ctx = HookContext {
            agent_name: "test-agent",
            agent_id: "abc-123",
            event: HookEvent::AfterToolCall,
            data: serde_json::json!({"tool_name": "file_read", "result": "ok"}),
        };
        assert!(registry.fire(&ctx).is_ok());
        assert_eq!(recorder.call_count(), 1);
    }

    #[test]
    fn test_multiple_handlers_all_fire() {
        let r1 = Arc::new(RecordHandler::new());
        let r2 = Arc::new(RecordHandler::new());
        let registry = HookRegistry::new();
        registry.register(HookEvent::AgentLoopEnd, r1.clone());
        registry.register(HookEvent::AgentLoopEnd, r2.clone());

        let ctx = make_ctx(HookEvent::AgentLoopEnd);
        assert!(registry.fire(&ctx).is_ok());
        assert_eq!(r1.call_count(), 1);
        assert_eq!(r2.call_count(), 1);
    }

    #[test]
    fn test_hook_errors_dont_crash_non_blocking() {
        let registry = HookRegistry::new();
        registry.register(
            HookEvent::AfterToolCall,
            Arc::new(BlockHandler {
                reason: "oops".to_string(),
            }),
        );
        let ctx = make_ctx(HookEvent::AfterToolCall);
        // AfterToolCall is non-blocking, so error should be swallowed
        assert!(registry.fire(&ctx).is_ok());
    }

    #[test]
    fn test_all_four_events_fire() {
        let recorder = Arc::new(RecordHandler::new());
        let registry = HookRegistry::new();
        registry.register(HookEvent::BeforeToolCall, recorder.clone());
        registry.register(HookEvent::AfterToolCall, recorder.clone());
        registry.register(HookEvent::BeforePromptBuild, recorder.clone());
        registry.register(HookEvent::AgentLoopEnd, recorder.clone());

        for event in [
            HookEvent::BeforeToolCall,
            HookEvent::AfterToolCall,
            HookEvent::BeforePromptBuild,
            HookEvent::AgentLoopEnd,
        ] {
            let ctx = make_ctx(event);
            let _ = registry.fire(&ctx);
        }
        assert_eq!(recorder.call_count(), 4);
    }

    #[test]
    fn test_has_handlers() {
        let registry = HookRegistry::new();
        assert!(!registry.has_handlers(HookEvent::BeforeToolCall));
        registry.register(HookEvent::BeforeToolCall, Arc::new(OkHandler));
        assert!(registry.has_handlers(HookEvent::BeforeToolCall));
        assert!(!registry.has_handlers(HookEvent::AfterToolCall));
    }

    #[test]
    fn test_handler_count() {
        let registry = HookRegistry::new();
        assert_eq!(registry.handler_count(), 0);
        registry.register(HookEvent::BeforeToolCall, Arc::new(OkHandler));
        registry.register(HookEvent::AfterToolCall, Arc::new(OkHandler));
        assert_eq!(registry.handler_count(), 2);
    }

    #[test]
    fn test_hook_event_display() {
        assert_eq!(HookEvent::BeforeToolCall.to_string(), "before_tool_call");
        assert_eq!(HookEvent::AgentLoopEnd.to_string(), "agent_loop_end");
    }

    #[test]
    fn test_hook_event_serde() {
        let event = HookEvent::BeforePromptBuild;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"before_prompt_build\"");
        let back: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HookEvent::BeforePromptBuild);
    }
}
