/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark A2A interoperability layer.
 */

//! A2A (Agent-to-Agent) Protocol — cross-framework agent interoperability.
//!
//! Google's A2A protocol enables cross-framework agent interoperability via
//! **Agent Cards** (JSON capability manifests) and **Task-based coordination**.
//!
//! This module provides:
//! - `AgentCard` — describes an agent's capabilities to external systems
//! - `A2aTask` / `A2aTaskStore` — task lifecycle management
//! - `A2aConfig` / `ExternalAgent` — configuration types (inlined)
//! - `AgentManifestSummary` — lightweight Ark-native manifest for card building
//!
//! **Ark adaptation**: Removed `openfang_types::agent::AgentManifest`,
//! `openfang_types::config::*`, `reqwest`, and `tracing`. The `A2aClient` HTTP
//! methods are replaced with URL builders — actual HTTP calls are deferred to
//! the integration boundary.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// A2A Configuration types (inlined from openfang_types::config)
// ---------------------------------------------------------------------------

/// Configuration for A2A protocol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct A2aConfig {
    /// Whether A2A is enabled.
    pub enabled: bool,
    /// HTTP path to listen on (e.g. "/a2a").
    pub listen_path: String,
    /// List of external agents to discover at boot.
    #[serde(default)]
    pub external_agents: Vec<ExternalAgent>,
}

/// An external agent to discover via A2A protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAgent {
    /// Display name for the external agent.
    pub name: String,
    /// Base URL of the external agent.
    pub url: String,
}

// ---------------------------------------------------------------------------
// AgentManifestSummary (Ark-native replacement for OpenFang's AgentManifest)
// ---------------------------------------------------------------------------

/// Lightweight manifest summary used for building A2A Agent Cards.
///
/// This replaces `openfang_types::agent::AgentManifest` with only the
/// fields needed for A2A card generation.
#[derive(Debug, Clone, Default)]
pub struct AgentManifestSummary {
    /// Agent display name.
    pub name: String,
    /// Agent description.
    pub description: String,
    /// List of tool names the agent can use.
    pub tools: Vec<String>,
}

// ---------------------------------------------------------------------------
// A2A Agent Card
// ---------------------------------------------------------------------------

/// A2A Agent Card — describes an agent's capabilities to external systems.
///
/// Served at `/.well-known/agent.json` per the A2A specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    /// Agent display name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Agent endpoint URL.
    pub url: String,
    /// Protocol version.
    pub version: String,
    /// Agent capabilities.
    pub capabilities: AgentCapabilities,
    /// Skills this agent can perform (A2A skill descriptors).
    pub skills: Vec<AgentSkill>,
    /// Supported input content types.
    #[serde(default)]
    pub default_input_modes: Vec<String>,
    /// Supported output content types.
    #[serde(default)]
    pub default_output_modes: Vec<String>,
}

/// A2A agent capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    /// Whether this agent supports streaming responses.
    pub streaming: bool,
    /// Whether this agent supports push notifications.
    pub push_notifications: bool,
    /// Whether task status history is available.
    pub state_transition_history: bool,
}

/// A2A skill descriptor — describes a capability for discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    /// Unique skill identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description of what this skill does.
    pub description: String,
    /// Tags for discovery.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Example prompts that trigger this skill.
    #[serde(default)]
    pub examples: Vec<String>,
}

// ---------------------------------------------------------------------------
// A2A Task
// ---------------------------------------------------------------------------

/// A2A Task — unit of work exchanged between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aTask {
    /// Unique task identifier.
    pub id: String,
    /// Optional session identifier for conversation continuity.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Current task status.
    pub status: A2aTaskStatus,
    /// Messages exchanged during the task.
    #[serde(default)]
    pub messages: Vec<A2aMessage>,
    /// Artifacts produced by the task.
    #[serde(default)]
    pub artifacts: Vec<A2aArtifact>,
}

/// A2A task status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum A2aTaskStatus {
    /// Task has been received but not started.
    Submitted,
    /// Task is being processed.
    Working,
    /// Agent needs more input from the caller.
    InputRequired,
    /// Task completed successfully.
    Completed,
    /// Task was cancelled.
    Cancelled,
    /// Task failed.
    Failed,
}

/// A2A message in a task conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aMessage {
    /// Message role ("user" or "agent").
    pub role: String,
    /// Message content parts.
    pub parts: Vec<A2aPart>,
}

/// A2A message content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum A2aPart {
    /// Text content.
    Text { text: String },
    /// File content (base64-encoded).
    File {
        name: String,
        mime_type: String,
        data: String,
    },
    /// Structured data.
    Data {
        mime_type: String,
        data: serde_json::Value,
    },
}

/// A2A artifact produced by a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aArtifact {
    /// Artifact name.
    pub name: String,
    /// Artifact content parts.
    pub parts: Vec<A2aPart>,
}

// ---------------------------------------------------------------------------
// A2A Task Store — tracks task lifecycle
// ---------------------------------------------------------------------------

/// In-memory store for tracking A2A task lifecycle.
///
/// Tasks are created by `tasks/send`, polled by `tasks/get`, and cancelled
/// by `tasks/cancel`. The store is bounded to prevent memory exhaustion.
#[derive(Debug)]
pub struct A2aTaskStore {
    tasks: Mutex<HashMap<String, A2aTask>>,
    /// Maximum number of tasks to retain (FIFO eviction).
    max_tasks: usize,
}

impl A2aTaskStore {
    /// Create a new task store with a capacity limit.
    pub fn new(max_tasks: usize) -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            max_tasks,
        }
    }

    /// Insert a task. If the store is at capacity, the oldest completed task is evicted.
    pub fn insert(&self, task: A2aTask) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        // Evict oldest completed/failed/cancelled tasks if at capacity
        if tasks.len() >= self.max_tasks {
            let evict_key = tasks
                .iter()
                .filter(|(_, t)| {
                    matches!(
                        t.status,
                        A2aTaskStatus::Completed | A2aTaskStatus::Failed | A2aTaskStatus::Cancelled
                    )
                })
                .map(|(k, _)| k.clone())
                .next();
            if let Some(key) = evict_key {
                tasks.remove(&key);
            }
        }
        tasks.insert(task.id.clone(), task);
    }

    /// Get a task by ID.
    pub fn get(&self, task_id: &str) -> Option<A2aTask> {
        self.tasks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(task_id)
            .cloned()
    }

    /// Update a task's status.
    pub fn update_status(&self, task_id: &str, status: A2aTaskStatus) -> bool {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = status;
            true
        } else {
            false
        }
    }

    /// Complete a task with a response message and optional artifacts.
    pub fn complete(&self, task_id: &str, response: A2aMessage, artifacts: Vec<A2aArtifact>) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(task) = tasks.get_mut(task_id) {
            task.messages.push(response);
            task.artifacts.extend(artifacts);
            task.status = A2aTaskStatus::Completed;
        }
    }

    /// Fail a task with an error message.
    pub fn fail(&self, task_id: &str, error_message: A2aMessage) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(task) = tasks.get_mut(task_id) {
            task.messages.push(error_message);
            task.status = A2aTaskStatus::Failed;
        }
    }

    /// Cancel a task.
    pub fn cancel(&self, task_id: &str) -> bool {
        self.update_status(task_id, A2aTaskStatus::Cancelled)
    }

    /// Count of tracked tasks.
    pub fn len(&self) -> usize {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for A2aTaskStore {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ---------------------------------------------------------------------------
// A2A Card Builder
// ---------------------------------------------------------------------------

/// Build an A2A Agent Card from an Ark agent manifest summary.
pub fn build_agent_card(manifest: &AgentManifestSummary, base_url: &str) -> AgentCard {
    // Convert tool names to A2A skill descriptors
    let skills: Vec<AgentSkill> = manifest
        .tools
        .iter()
        .map(|tool| AgentSkill {
            id: tool.clone(),
            name: tool.replace('_', " "),
            description: format!("Can use the {tool} tool"),
            tags: vec!["tool".to_string()],
            examples: vec![],
        })
        .collect();

    AgentCard {
        name: manifest.name.clone(),
        description: manifest.description.clone(),
        url: format!("{base_url}/a2a"),
        version: "0.1.0".to_string(),
        capabilities: AgentCapabilities {
            streaming: true,
            push_notifications: false,
            state_transition_history: true,
        },
        skills,
        default_input_modes: vec!["text".to_string()],
        default_output_modes: vec!["text".to_string()],
    }
}

// ---------------------------------------------------------------------------
// A2A URL Builders (replaces reqwest-based A2aClient)
// ---------------------------------------------------------------------------

/// Build the A2A discovery URL for a given agent base URL.
///
/// Per the A2A spec, Agent Cards are served at `/.well-known/agent.json`.
pub fn build_discovery_url(base_url: &str) -> String {
    format!("{}/.well-known/agent.json", base_url.trim_end_matches('/'))
}

/// Build a JSON-RPC 2.0 request body for `tasks/send`.
pub fn build_send_task_body(message: &str, session_id: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tasks/send",
        "params": {
            "message": {
                "role": "user",
                "parts": [{"type": "text", "text": message}]
            },
            "sessionId": session_id,
        }
    })
}

/// Build a JSON-RPC 2.0 request body for `tasks/get`.
pub fn build_get_task_body(task_id: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tasks/get",
        "params": {
            "id": task_id,
        }
    })
}

/// Parse a JSON-RPC 2.0 response into an A2aTask.
pub fn parse_task_response(body: &serde_json::Value) -> Result<A2aTask, String> {
    if let Some(result) = body.get("result") {
        serde_json::from_value(result.clone())
            .map_err(|e| format!("Invalid A2A task response: {e}"))
    } else if let Some(error) = body.get("error") {
        Err(format!("A2A error: {}", error))
    } else {
        Err("Empty A2A response".to_string())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_card_from_manifest() {
        let manifest = AgentManifestSummary {
            name: "test-agent".to_string(),
            description: "A test agent".to_string(),
            tools: vec!["web_search".to_string(), "file_read".to_string()],
        };

        let card = build_agent_card(&manifest, "https://example.com");
        assert_eq!(card.name, "test-agent");
        assert_eq!(card.description, "A test agent");
        assert!(card.url.contains("/a2a"));
        assert!(card.capabilities.streaming);
        assert_eq!(card.default_input_modes, vec!["text"]);
        assert_eq!(card.skills.len(), 2);
        assert_eq!(card.skills[0].name, "web search");
    }

    #[test]
    fn test_agent_card_serde() {
        let card = AgentCard {
            name: "test".to_string(),
            description: "test desc".to_string(),
            url: "https://example.com/a2a".to_string(),
            version: "0.1.0".to_string(),
            capabilities: AgentCapabilities::default(),
            skills: vec![],
            default_input_modes: vec!["text".to_string()],
            default_output_modes: vec!["text".to_string()],
        };
        let json = serde_json::to_string(&card).unwrap();
        let back: AgentCard = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test");
        assert!(json.contains("defaultInputModes")); // camelCase
    }

    #[test]
    fn test_a2a_task_status_transitions() {
        let task = A2aTask {
            id: "task-1".to_string(),
            session_id: None,
            status: A2aTaskStatus::Submitted,
            messages: vec![],
            artifacts: vec![],
        };
        assert_eq!(task.status, A2aTaskStatus::Submitted);

        let working = A2aTask {
            status: A2aTaskStatus::Working,
            ..task.clone()
        };
        assert_eq!(working.status, A2aTaskStatus::Working);

        let completed = A2aTask {
            status: A2aTaskStatus::Completed,
            ..task.clone()
        };
        assert_eq!(completed.status, A2aTaskStatus::Completed);

        let cancelled = A2aTask {
            status: A2aTaskStatus::Cancelled,
            ..task.clone()
        };
        assert_eq!(cancelled.status, A2aTaskStatus::Cancelled);

        let failed = A2aTask {
            status: A2aTaskStatus::Failed,
            ..task
        };
        assert_eq!(failed.status, A2aTaskStatus::Failed);
    }

    #[test]
    fn test_a2a_message_serde() {
        let msg = A2aMessage {
            role: "user".to_string(),
            parts: vec![
                A2aPart::Text {
                    text: "Hello".to_string(),
                },
                A2aPart::Data {
                    mime_type: "application/json".to_string(),
                    data: serde_json::json!({"key": "value"}),
                },
            ],
        };

        let json = serde_json::to_string(&msg).unwrap();
        let back: A2aMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.role, "user");
        assert_eq!(back.parts.len(), 2);

        match &back.parts[0] {
            A2aPart::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected Text part"),
        }
    }

    #[test]
    fn test_task_store_insert_and_get() {
        let store = A2aTaskStore::new(10);
        let task = A2aTask {
            id: "t-1".to_string(),
            session_id: None,
            status: A2aTaskStatus::Working,
            messages: vec![],
            artifacts: vec![],
        };
        store.insert(task);
        assert_eq!(store.len(), 1);

        let got = store.get("t-1").unwrap();
        assert_eq!(got.status, A2aTaskStatus::Working);
    }

    #[test]
    fn test_task_store_complete_and_fail() {
        let store = A2aTaskStore::new(10);
        let task = A2aTask {
            id: "t-2".to_string(),
            session_id: None,
            status: A2aTaskStatus::Working,
            messages: vec![],
            artifacts: vec![],
        };
        store.insert(task);

        store.complete(
            "t-2",
            A2aMessage {
                role: "agent".to_string(),
                parts: vec![A2aPart::Text {
                    text: "Done".to_string(),
                }],
            },
            vec![],
        );

        let completed = store.get("t-2").unwrap();
        assert_eq!(completed.status, A2aTaskStatus::Completed);
        assert_eq!(completed.messages.len(), 1);
    }

    #[test]
    fn test_task_store_cancel() {
        let store = A2aTaskStore::new(10);
        let task = A2aTask {
            id: "t-3".to_string(),
            session_id: None,
            status: A2aTaskStatus::Working,
            messages: vec![],
            artifacts: vec![],
        };
        store.insert(task);
        assert!(store.cancel("t-3"));
        assert_eq!(store.get("t-3").unwrap().status, A2aTaskStatus::Cancelled);
        assert!(!store.cancel("t-999"));
    }

    #[test]
    fn test_task_store_eviction() {
        let store = A2aTaskStore::new(2);
        for i in 0..2 {
            let task = A2aTask {
                id: format!("t-{i}"),
                session_id: None,
                status: A2aTaskStatus::Completed,
                messages: vec![],
                artifacts: vec![],
            };
            store.insert(task);
        }
        assert_eq!(store.len(), 2);

        let task = A2aTask {
            id: "t-2".to_string(),
            session_id: None,
            status: A2aTaskStatus::Working,
            messages: vec![],
            artifacts: vec![],
        };
        store.insert(task);
        assert!(store.len() <= 2);
    }

    #[test]
    fn test_a2a_config_serde() {
        let config = A2aConfig {
            enabled: true,
            listen_path: "/a2a".to_string(),
            external_agents: vec![ExternalAgent {
                name: "other-agent".to_string(),
                url: "https://other.example.com".to_string(),
            }],
        };

        let json = serde_json::to_string(&config).unwrap();
        let back: A2aConfig = serde_json::from_str(&json).unwrap();
        assert!(back.enabled);
        assert_eq!(back.listen_path, "/a2a");
        assert_eq!(back.external_agents.len(), 1);
        assert_eq!(back.external_agents[0].name, "other-agent");
    }

    #[test]
    fn test_build_discovery_url() {
        let url = build_discovery_url("https://agent.example.com");
        assert_eq!(url, "https://agent.example.com/.well-known/agent.json");

        let url2 = build_discovery_url("https://agent.example.com/");
        assert_eq!(url2, "https://agent.example.com/.well-known/agent.json");
    }

    #[test]
    fn test_build_send_task_body() {
        let body = build_send_task_body("Hello", Some("session-1"));
        assert_eq!(body["method"], "tasks/send");
        assert_eq!(body["params"]["sessionId"], "session-1");
    }

    #[test]
    fn test_build_get_task_body() {
        let body = build_get_task_body("task-123");
        assert_eq!(body["method"], "tasks/get");
        assert_eq!(body["params"]["id"], "task-123");
    }

    #[test]
    fn test_parse_task_response_success() {
        let body = serde_json::json!({
            "result": {
                "id": "t-1",
                "status": "completed",
                "messages": [],
                "artifacts": []
            }
        });
        let task = parse_task_response(&body).unwrap();
        assert_eq!(task.id, "t-1");
        assert_eq!(task.status, A2aTaskStatus::Completed);
    }

    #[test]
    fn test_parse_task_response_error() {
        let body = serde_json::json!({
            "error": {"code": -32600, "message": "Invalid request"}
        });
        let result = parse_task_response(&body);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("A2A error"));
    }

    #[test]
    fn test_task_store_default() {
        let store = A2aTaskStore::default();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_task_store_fail() {
        let store = A2aTaskStore::new(10);
        let task = A2aTask {
            id: "t-fail".to_string(),
            session_id: None,
            status: A2aTaskStatus::Working,
            messages: vec![],
            artifacts: vec![],
        };
        store.insert(task);
        store.fail(
            "t-fail",
            A2aMessage {
                role: "agent".to_string(),
                parts: vec![A2aPart::Text {
                    text: "Error occurred".to_string(),
                }],
            },
        );
        let failed = store.get("t-fail").unwrap();
        assert_eq!(failed.status, A2aTaskStatus::Failed);
        assert_eq!(failed.messages.len(), 1);
    }
}
