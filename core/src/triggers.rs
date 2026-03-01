//! Event-driven agent triggers — agents auto-activate when events match patterns.
//!
//! Architecture informed by OpenFang (MIT/Apache-2.0).
//! Ark-native implementation: Mutex<HashMap> concurrency, epoch timestamps,
//! counter-based IDs, zero external dependencies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

// ── ID Generation ─────────────────────────────────────────────────

static TRIGGER_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_trigger_id() -> String {
    format!("trg-{}", TRIGGER_COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Inlined Types ────────────────────────────────────────────────

/// Agent identifier.
pub type AgentId = String;

/// Unique identifier for a trigger.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TriggerId(pub String);

impl TriggerId {
    pub fn new() -> Self {
        Self(next_trigger_id())
    }
}

impl Default for TriggerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TriggerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Event System (inlined from openfang_types::event) ─────────

/// Target for an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventTarget {
    /// Specific agent.
    Agent(AgentId),
    /// All agents.
    Broadcast,
    /// System-level handler.
    System,
}

/// Lifecycle events for agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent {
    Spawned { agent_id: AgentId, name: String },
    Started { agent_id: AgentId },
    Suspended { agent_id: AgentId },
    Resumed { agent_id: AgentId },
    Terminated { agent_id: AgentId, reason: String },
    Crashed { agent_id: AgentId, error: String },
}

/// System events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    KernelStarted,
    KernelStopping,
    QuotaWarning {
        agent_id: AgentId,
        resource: String,
        usage_percent: f64,
    },
    HealthCheck {
        status: String,
    },
    QuotaEnforced {
        agent_id: AgentId,
        spent: f64,
        limit: f64,
    },
    ModelRouted {
        agent_id: AgentId,
        complexity: String,
        model: String,
    },
    UserAction {
        user_id: String,
        action: String,
        result: String,
    },
    HealthCheckFailed {
        agent_id: AgentId,
        unresponsive_secs: u64,
    },
}

/// Message role in conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// A message payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub role: MessageRole,
    pub content: String,
}

/// A tool result payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPayload {
    pub tool_id: String,
    pub success: bool,
    pub content: String,
    pub execution_time_ms: u64,
}

/// A memory update delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDelta {
    pub agent_id: AgentId,
    pub key: String,
    pub operation: MemoryOp,
}

/// Memory operation type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryOp {
    Set,
    Delete,
    Update,
}

/// Network event placeholder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEvent {
    pub description: String,
}

/// The payload of an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    Message(MessagePayload),
    ToolResult(ToolResultPayload),
    MemoryUpdate(MemoryDelta),
    Lifecycle(LifecycleEvent),
    Network(NetworkEvent),
    System(SystemEvent),
    Custom(Vec<u8>),
}

/// A full event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub source: AgentId,
    pub target: EventTarget,
    pub payload: EventPayload,
    pub timestamp: u64,
}

impl Event {
    pub fn new(source: AgentId, target: EventTarget, payload: EventPayload) -> Self {
        Self {
            source,
            target,
            payload,
            timestamp: now_epoch(),
        }
    }
}

// ── Trigger Pattern ──────────────────────────────────────────────

/// What kind of events a trigger matches on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerPattern {
    /// Match any lifecycle event.
    Lifecycle,
    /// Match when a specific agent name is spawned.
    AgentSpawned { name_pattern: String },
    /// Match when any agent is terminated or crashed.
    AgentTerminated,
    /// Match any system event.
    System,
    /// Match a specific system event by keyword.
    SystemKeyword { keyword: String },
    /// Match any memory update event.
    MemoryUpdate,
    /// Match memory updates for a specific key pattern.
    MemoryKeyPattern { key_pattern: String },
    /// Match all events (wildcard).
    All,
    /// Match custom events by content substring.
    ContentMatch { substring: String },
}

/// A registered trigger definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// Unique trigger ID.
    pub id: TriggerId,
    /// Which agent owns this trigger.
    pub agent_id: AgentId,
    /// The event pattern to match.
    pub pattern: TriggerPattern,
    /// Prompt template to send when triggered. Use `{{event}}` for event description.
    pub prompt_template: String,
    /// Whether this trigger is currently active.
    pub enabled: bool,
    /// When this trigger was created (epoch seconds).
    pub created_at: u64,
    /// How many times this trigger has fired.
    pub fire_count: u64,
    /// Maximum number of times this trigger can fire (0 = unlimited).
    pub max_fires: u64,
}

// ── Trigger Engine ───────────────────────────────────────────────

/// The trigger engine manages event-to-agent routing.
pub struct TriggerEngine {
    triggers: Mutex<HashMap<String, Trigger>>,
    agent_triggers: Mutex<HashMap<AgentId, Vec<String>>>,
}

impl TriggerEngine {
    /// Create a new trigger engine.
    pub fn new() -> Self {
        Self {
            triggers: Mutex::new(HashMap::new()),
            agent_triggers: Mutex::new(HashMap::new()),
        }
    }

    /// Register a new trigger. Returns the trigger ID.
    pub fn register(
        &self,
        agent_id: AgentId,
        pattern: TriggerPattern,
        prompt_template: String,
        max_fires: u64,
    ) -> TriggerId {
        let trigger = Trigger {
            id: TriggerId::new(),
            agent_id: agent_id.clone(),
            pattern,
            prompt_template,
            enabled: true,
            created_at: now_epoch(),
            fire_count: 0,
            max_fires,
        };
        let id = trigger.id.clone();
        let id_str = id.0.clone();

        self.triggers
            .lock()
            .expect("unexpected failure")
            .insert(id_str.clone(), trigger);
        self.agent_triggers
            .lock()
            .expect("unexpected failure")
            .entry(agent_id.clone())
            .or_default()
            .push(id_str);

        eprintln!("[triggers] registered {} for agent {}", id, agent_id);
        id
    }

    /// Remove a trigger. Returns true if found.
    pub fn remove(&self, trigger_id: &TriggerId) -> bool {
        let mut triggers = self.triggers.lock().expect("mutex poisoned");
        if let Some(trigger) = triggers.remove(&trigger_id.0) {
            let mut at = self.agent_triggers.lock().expect("mutex poisoned");
            if let Some(list) = at.get_mut(&trigger.agent_id) {
                list.retain(|id| *id != trigger_id.0);
            }
            true
        } else {
            false
        }
    }

    /// Remove all triggers for an agent.
    pub fn remove_agent_triggers(&self, agent_id: &str) {
        let mut at = self.agent_triggers.lock().expect("mutex poisoned");
        if let Some(trigger_ids) = at.remove(agent_id) {
            let mut triggers = self.triggers.lock().expect("mutex poisoned");
            for id in trigger_ids {
                triggers.remove(&id);
            }
        }
    }

    /// Enable or disable a trigger. Returns true if found.
    pub fn set_enabled(&self, trigger_id: &TriggerId, enabled: bool) -> bool {
        let mut triggers = self.triggers.lock().expect("mutex poisoned");
        if let Some(t) = triggers.get_mut(&trigger_id.0) {
            t.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// List all triggers for an agent.
    pub fn list_agent_triggers(&self, agent_id: &str) -> Vec<Trigger> {
        let at = self.agent_triggers.lock().expect("mutex poisoned");
        let triggers = self.triggers.lock().expect("mutex poisoned");
        at.get(agent_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| triggers.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List all registered triggers.
    pub fn list_all(&self) -> Vec<Trigger> {
        self.triggers.lock().expect("mutex poisoned").values().cloned().collect()
    }

    /// Evaluate an event against all triggers. Returns a list of
    /// `(agent_id, message_to_send)` pairs for matching triggers.
    pub fn evaluate(&self, event: &Event) -> Vec<(AgentId, String)> {
        let event_description = describe_event(event);
        let mut matches = Vec::new();
        let mut triggers = self.triggers.lock().expect("mutex poisoned");

        for trigger in triggers.values_mut() {
            if !trigger.enabled {
                continue;
            }
            if trigger.max_fires > 0 && trigger.fire_count >= trigger.max_fires {
                trigger.enabled = false;
                continue;
            }
            if matches_pattern(&trigger.pattern, event, &event_description) {
                let message = trigger
                    .prompt_template
                    .replace("{{event}}", &event_description);
                matches.push((trigger.agent_id.clone(), message));
                trigger.fire_count += 1;
            }
        }

        matches
    }

    /// Get a trigger by ID.
    pub fn get(&self, trigger_id: &TriggerId) -> Option<Trigger> {
        self.triggers.lock().expect("mutex poisoned").get(&trigger_id.0).cloned()
    }

    /// Total number of registered triggers.
    pub fn count(&self) -> usize {
        self.triggers.lock().expect("mutex poisoned").len()
    }
}

impl Default for TriggerEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Pattern Matching ─────────────────────────────────────────────

fn matches_pattern(pattern: &TriggerPattern, event: &Event, description: &str) -> bool {
    match pattern {
        TriggerPattern::All => true,
        TriggerPattern::Lifecycle => {
            matches!(event.payload, EventPayload::Lifecycle(_))
        }
        TriggerPattern::AgentSpawned { name_pattern } => {
            if let EventPayload::Lifecycle(LifecycleEvent::Spawned { name, .. }) = &event.payload {
                name.contains(name_pattern.as_str()) || name_pattern == "*"
            } else {
                false
            }
        }
        TriggerPattern::AgentTerminated => matches!(
            event.payload,
            EventPayload::Lifecycle(LifecycleEvent::Terminated { .. })
                | EventPayload::Lifecycle(LifecycleEvent::Crashed { .. })
        ),
        TriggerPattern::System => {
            matches!(event.payload, EventPayload::System(_))
        }
        TriggerPattern::SystemKeyword { keyword } => {
            if let EventPayload::System(se) = &event.payload {
                let se_str = format!("{:?}", se).to_lowercase();
                se_str.contains(&keyword.to_lowercase())
            } else {
                false
            }
        }
        TriggerPattern::MemoryUpdate => {
            matches!(event.payload, EventPayload::MemoryUpdate(_))
        }
        TriggerPattern::MemoryKeyPattern { key_pattern } => {
            if let EventPayload::MemoryUpdate(delta) = &event.payload {
                delta.key.contains(key_pattern.as_str()) || key_pattern == "*"
            } else {
                false
            }
        }
        TriggerPattern::ContentMatch { substring } => description
            .to_lowercase()
            .contains(&substring.to_lowercase()),
    }
}

// ── Event Description ────────────────────────────────────────────

/// Create a human-readable description of an event for use in prompts.
fn describe_event(event: &Event) -> String {
    match &event.payload {
        EventPayload::Message(msg) => {
            format!("Message from {:?}: {}", msg.role, msg.content)
        }
        EventPayload::ToolResult(tr) => {
            let content_preview = if tr.content.len() > 200 {
                &tr.content[..200]
            } else {
                &tr.content
            };
            format!(
                "Tool '{}' {} ({}ms): {}",
                tr.tool_id,
                if tr.success { "succeeded" } else { "failed" },
                tr.execution_time_ms,
                content_preview
            )
        }
        EventPayload::MemoryUpdate(delta) => {
            format!(
                "Memory {:?} on key '{}' for agent {}",
                delta.operation, delta.key, delta.agent_id
            )
        }
        EventPayload::Lifecycle(le) => match le {
            LifecycleEvent::Spawned { agent_id, name } => {
                format!("Agent '{name}' (id: {agent_id}) was spawned")
            }
            LifecycleEvent::Started { agent_id } => {
                format!("Agent {agent_id} started")
            }
            LifecycleEvent::Suspended { agent_id } => {
                format!("Agent {agent_id} suspended")
            }
            LifecycleEvent::Resumed { agent_id } => {
                format!("Agent {agent_id} resumed")
            }
            LifecycleEvent::Terminated { agent_id, reason } => {
                format!("Agent {agent_id} terminated: {reason}")
            }
            LifecycleEvent::Crashed { agent_id, error } => {
                format!("Agent {agent_id} crashed: {error}")
            }
        },
        EventPayload::Network(ne) => {
            format!("Network event: {}", ne.description)
        }
        EventPayload::System(se) => match se {
            SystemEvent::KernelStarted => "Kernel started".to_string(),
            SystemEvent::KernelStopping => "Kernel stopping".to_string(),
            SystemEvent::QuotaWarning {
                agent_id,
                resource,
                usage_percent,
            } => format!("Quota warning: agent {agent_id}, {resource} at {usage_percent:.1}%"),
            SystemEvent::HealthCheck { status } => format!("Health check: {status}"),
            SystemEvent::QuotaEnforced {
                agent_id,
                spent,
                limit,
            } => format!("Quota enforced: agent {agent_id}, spent ${spent:.4} / ${limit:.4}"),
            SystemEvent::ModelRouted {
                agent_id,
                complexity,
                model,
            } => format!("Model routed: agent {agent_id}, complexity={complexity}, model={model}"),
            SystemEvent::UserAction {
                user_id,
                action,
                result,
            } => format!("User action: {user_id} {action} -> {result}"),
            SystemEvent::HealthCheckFailed {
                agent_id,
                unresponsive_secs,
            } => format!(
                "Health check failed: agent {agent_id}, unresponsive for {unresponsive_secs}s"
            ),
        },
        EventPayload::Custom(data) => {
            format!("Custom event ({} bytes)", data.len())
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_trigger() {
        let engine = TriggerEngine::new();
        let id = engine.register(
            "agent-1".to_string(),
            TriggerPattern::All,
            "Event occurred: {{event}}".to_string(),
            0,
        );
        assert!(engine.get(&id).is_some());
    }

    #[test]
    fn test_evaluate_lifecycle() {
        let engine = TriggerEngine::new();
        let watcher = "watcher".to_string();
        engine.register(
            watcher.clone(),
            TriggerPattern::Lifecycle,
            "Lifecycle: {{event}}".to_string(),
            0,
        );

        let event = Event::new(
            "spawner".to_string(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: "new-agent-id".to_string(),
                name: "new-agent".to_string(),
            }),
        );

        let matches = engine.evaluate(&event);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, watcher);
        assert!(matches[0].1.contains("new-agent"));
    }

    #[test]
    fn test_evaluate_agent_spawned_pattern() {
        let engine = TriggerEngine::new();
        engine.register(
            "watcher".to_string(),
            TriggerPattern::AgentSpawned {
                name_pattern: "coder".to_string(),
            },
            "Coder spawned: {{event}}".to_string(),
            0,
        );

        // This should match
        let event = Event::new(
            "sys".to_string(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: "c1".to_string(),
                name: "coder".to_string(),
            }),
        );
        assert_eq!(engine.evaluate(&event).len(), 1);

        // This should NOT match
        let event2 = Event::new(
            "sys".to_string(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: "r1".to_string(),
                name: "researcher".to_string(),
            }),
        );
        assert_eq!(engine.evaluate(&event2).len(), 0);
    }

    #[test]
    fn test_max_fires() {
        let engine = TriggerEngine::new();
        engine.register(
            "agent-1".to_string(),
            TriggerPattern::All,
            "Event: {{event}}".to_string(),
            2, // max 2 fires
        );

        let event = Event::new(
            "sys".to_string(),
            EventTarget::Broadcast,
            EventPayload::System(SystemEvent::HealthCheck {
                status: "ok".to_string(),
            }),
        );

        // First two should match
        assert_eq!(engine.evaluate(&event).len(), 1);
        assert_eq!(engine.evaluate(&event).len(), 1);
        // Third should not
        assert_eq!(engine.evaluate(&event).len(), 0);
    }

    #[test]
    fn test_remove_trigger() {
        let engine = TriggerEngine::new();
        let id = engine.register("a".to_string(), TriggerPattern::All, "m".to_string(), 0);
        assert!(engine.remove(&id));
        assert!(engine.get(&id).is_none());
    }

    #[test]
    fn test_remove_agent_triggers() {
        let engine = TriggerEngine::new();
        engine.register(
            "agent-x".to_string(),
            TriggerPattern::All,
            "a".to_string(),
            0,
        );
        engine.register(
            "agent-x".to_string(),
            TriggerPattern::System,
            "b".to_string(),
            0,
        );
        assert_eq!(engine.list_agent_triggers("agent-x").len(), 2);

        engine.remove_agent_triggers("agent-x");
        assert_eq!(engine.list_agent_triggers("agent-x").len(), 0);
    }

    #[test]
    fn test_content_match() {
        let engine = TriggerEngine::new();
        engine.register(
            "agent-1".to_string(),
            TriggerPattern::ContentMatch {
                substring: "quota".to_string(),
            },
            "Alert: {{event}}".to_string(),
            0,
        );

        let event = Event::new(
            "sys".to_string(),
            EventTarget::System,
            EventPayload::System(SystemEvent::QuotaWarning {
                agent_id: "a".to_string(),
                resource: "tokens".to_string(),
                usage_percent: 85.0,
            }),
        );
        assert_eq!(engine.evaluate(&event).len(), 1);
    }

    #[test]
    fn test_system_keyword_match() {
        let engine = TriggerEngine::new();
        engine.register(
            "agent-1".to_string(),
            TriggerPattern::SystemKeyword {
                keyword: "kernel".to_string(),
            },
            "Kernel event: {{event}}".to_string(),
            0,
        );

        let event = Event::new(
            "sys".to_string(),
            EventTarget::System,
            EventPayload::System(SystemEvent::KernelStarted),
        );
        assert_eq!(engine.evaluate(&event).len(), 1);
    }

    #[test]
    fn test_memory_key_pattern() {
        let engine = TriggerEngine::new();
        engine.register(
            "agent-1".to_string(),
            TriggerPattern::MemoryKeyPattern {
                key_pattern: "user_pref".to_string(),
            },
            "Memory changed: {{event}}".to_string(),
            0,
        );

        let event = Event::new(
            "mem".to_string(),
            EventTarget::Broadcast,
            EventPayload::MemoryUpdate(MemoryDelta {
                agent_id: "a".to_string(),
                key: "user_preferences_theme".to_string(),
                operation: MemoryOp::Set,
            }),
        );
        assert_eq!(engine.evaluate(&event).len(), 1);

        // Key that doesn't match
        let event2 = Event::new(
            "mem".to_string(),
            EventTarget::Broadcast,
            EventPayload::MemoryUpdate(MemoryDelta {
                agent_id: "a".to_string(),
                key: "system_config".to_string(),
                operation: MemoryOp::Set,
            }),
        );
        assert_eq!(engine.evaluate(&event2).len(), 0);
    }

    #[test]
    fn test_wildcard_agent_spawned() {
        let engine = TriggerEngine::new();
        engine.register(
            "watcher".to_string(),
            TriggerPattern::AgentSpawned {
                name_pattern: "*".to_string(),
            },
            "Any agent spawned: {{event}}".to_string(),
            0,
        );

        let event = Event::new(
            "sys".to_string(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: "x".to_string(),
                name: "any-name-at-all".to_string(),
            }),
        );
        assert_eq!(engine.evaluate(&event).len(), 1);
    }

    #[test]
    fn test_agent_terminated_matches_crash() {
        let engine = TriggerEngine::new();
        engine.register(
            "watcher".to_string(),
            TriggerPattern::AgentTerminated,
            "Agent down: {{event}}".to_string(),
            0,
        );

        let event = Event::new(
            "sys".to_string(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Crashed {
                agent_id: "x".to_string(),
                error: "OOM".to_string(),
            }),
        );
        assert_eq!(engine.evaluate(&event).len(), 1);
    }

    #[test]
    fn test_disabled_trigger_skipped() {
        let engine = TriggerEngine::new();
        let id = engine.register(
            "agent-1".to_string(),
            TriggerPattern::All,
            "Event: {{event}}".to_string(),
            0,
        );
        engine.set_enabled(&id, false);

        let event = Event::new(
            "sys".to_string(),
            EventTarget::Broadcast,
            EventPayload::System(SystemEvent::KernelStarted),
        );
        assert_eq!(engine.evaluate(&event).len(), 0);
    }

    #[test]
    fn test_list_all() {
        let engine = TriggerEngine::new();
        engine.register("a".to_string(), TriggerPattern::All, "x".to_string(), 0);
        engine.register("b".to_string(), TriggerPattern::System, "y".to_string(), 0);
        assert_eq!(engine.list_all().len(), 2);
    }

    #[test]
    fn test_count() {
        let engine = TriggerEngine::new();
        assert_eq!(engine.count(), 0);
        engine.register("a".to_string(), TriggerPattern::All, "x".to_string(), 0);
        assert_eq!(engine.count(), 1);
    }

    #[test]
    fn test_describe_event_message() {
        let event = Event::new(
            "user".to_string(),
            EventTarget::Agent("a".to_string()),
            EventPayload::Message(MessagePayload {
                role: MessageRole::User,
                content: "Hello agent".to_string(),
            }),
        );
        let desc = describe_event(&event);
        assert!(desc.contains("Hello agent"));
    }

    #[test]
    fn test_describe_event_tool_result() {
        let event = Event::new(
            "sys".to_string(),
            EventTarget::Agent("a".to_string()),
            EventPayload::ToolResult(ToolResultPayload {
                tool_id: "file_read".to_string(),
                success: true,
                content: "file contents here".to_string(),
                execution_time_ms: 42,
            }),
        );
        let desc = describe_event(&event);
        assert!(desc.contains("file_read"));
        assert!(desc.contains("succeeded"));
        assert!(desc.contains("42ms"));
    }
}
