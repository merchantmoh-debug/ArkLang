/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark kernel handle abstraction.
 */

//! Trait abstraction for kernel operations needed by the agent runtime.
//!
//! This trait allows the agent runtime to call back into the kernel for
//! inter-agent operations (spawn, send, list, kill) without creating
//! a circular dependency. The kernel implements this trait and passes
//! it into the agent loop.
//!
//! **Ark adaptation**: Removed `async_trait` — all methods are synchronous.
//! Removed `openfang_types::capability::Capability` (uses Ark's own
//! `crate::capability::Capability`). Inlined `Entity`, `Relation`,
//! `GraphPattern`, `GraphMatch` from `openfang_types::memory`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// AgentInfo
// ---------------------------------------------------------------------------

/// Agent info returned by list and discovery operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique agent identifier.
    pub id: String,
    /// Agent display name.
    pub name: String,
    /// Current agent state (e.g. "running", "idle", "stopped").
    pub state: String,
    /// LLM provider name.
    pub model_provider: String,
    /// LLM model name.
    pub model_name: String,
    /// Agent description.
    pub description: String,
    /// Tags for discovery.
    pub tags: Vec<String>,
    /// Tool names available to this agent.
    pub tools: Vec<String>,
}

impl Default for AgentInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            state: "idle".to_string(),
            model_provider: String::new(),
            model_name: String::new(),
            description: String::new(),
            tags: Vec::new(),
            tools: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Knowledge Graph types (inlined from openfang_types::memory)
// ---------------------------------------------------------------------------

/// An entity in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Entity name/label.
    pub name: String,
    /// Entity type (e.g. "person", "concept", "file").
    pub entity_type: String,
    /// Freeform properties.
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
}

/// A directed relation between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Source entity name.
    pub from: String,
    /// Relation type (e.g. "depends_on", "created_by").
    pub relation_type: String,
    /// Target entity name.
    pub to: String,
    /// Freeform properties on the relation.
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
}

/// A pattern for querying the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPattern {
    /// Entity name pattern (substring match, case-insensitive).
    #[serde(default)]
    pub entity_name: Option<String>,
    /// Entity type filter.
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Relation type filter.
    #[serde(default)]
    pub relation_type: Option<String>,
    /// Maximum number of results.
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_max_results() -> usize {
    50
}

impl Default for GraphPattern {
    fn default() -> Self {
        Self {
            entity_name: None,
            entity_type: None,
            relation_type: None,
            max_results: default_max_results(),
        }
    }
}

/// A match result from a knowledge graph query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMatch {
    /// Matching entities.
    pub entities: Vec<Entity>,
    /// Matching relations.
    pub relations: Vec<Relation>,
}

// ---------------------------------------------------------------------------
// KernelHandle trait
// ---------------------------------------------------------------------------

/// Handle to kernel operations, passed into the agent loop so agents
/// can interact with each other via tools.
///
/// All methods are synchronous (Ark adaptation — removed `async_trait`).
/// Default implementations return "not available" errors to allow
/// incremental kernel feature rollout.
pub trait KernelHandle: Send + Sync {
    /// Spawn a new agent from a TOML manifest string.
    /// `parent_id` is the UUID string of the spawning agent (for lineage tracking).
    /// Returns (agent_id, agent_name) on success.
    fn spawn_agent(
        &self,
        manifest_toml: &str,
        parent_id: Option<&str>,
    ) -> Result<(String, String), String>;

    /// Send a message to another agent and get the response.
    fn send_to_agent(&self, agent_id: &str, message: &str) -> Result<String, String>;

    /// List all running agents.
    fn list_agents(&self) -> Vec<AgentInfo>;

    /// Kill an agent by ID.
    fn kill_agent(&self, agent_id: &str) -> Result<(), String>;

    /// Store a value in shared memory (cross-agent accessible).
    fn memory_store(&self, key: &str, value: serde_json::Value) -> Result<(), String>;

    /// Recall a value from shared memory.
    fn memory_recall(&self, key: &str) -> Result<Option<serde_json::Value>, String>;

    /// Find agents by query (matches on name substring, tag, or tool name; case-insensitive).
    fn find_agents(&self, query: &str) -> Vec<AgentInfo>;

    /// Post a task to the shared task queue. Returns the task ID.
    fn task_post(
        &self,
        title: &str,
        description: &str,
        assigned_to: Option<&str>,
        created_by: Option<&str>,
    ) -> Result<String, String> {
        let _ = (title, description, assigned_to, created_by);
        Err("Task queue not available".to_string())
    }

    /// Claim the next available task. Returns task JSON or None.
    fn task_claim(&self, agent_id: &str) -> Result<Option<serde_json::Value>, String> {
        let _ = agent_id;
        Err("Task queue not available".to_string())
    }

    /// Mark a task as completed with a result string.
    fn task_complete(&self, task_id: &str, result: &str) -> Result<(), String> {
        let _ = (task_id, result);
        Err("Task queue not available".to_string())
    }

    /// List tasks, optionally filtered by status.
    fn task_list(&self, status: Option<&str>) -> Result<Vec<serde_json::Value>, String> {
        let _ = status;
        Err("Task queue not available".to_string())
    }

    /// Publish a custom event that can trigger proactive agents.
    fn publish_event(&self, event_type: &str, payload: serde_json::Value) -> Result<(), String> {
        let _ = (event_type, payload);
        Err("Event bus not available".to_string())
    }

    /// Add an entity to the knowledge graph.
    fn knowledge_add_entity(&self, entity: Entity) -> Result<String, String> {
        let _ = entity;
        Err("Knowledge graph not available".to_string())
    }

    /// Add a relation to the knowledge graph.
    fn knowledge_add_relation(&self, relation: Relation) -> Result<String, String> {
        let _ = relation;
        Err("Knowledge graph not available".to_string())
    }

    /// Query the knowledge graph with a pattern.
    fn knowledge_query(&self, pattern: GraphPattern) -> Result<Vec<GraphMatch>, String> {
        let _ = pattern;
        Err("Knowledge graph not available".to_string())
    }

    /// Create a cron job for the calling agent.
    fn cron_create(&self, agent_id: &str, job_json: serde_json::Value) -> Result<String, String> {
        let _ = (agent_id, job_json);
        Err("Cron scheduler not available".to_string())
    }

    /// List cron jobs for the calling agent.
    fn cron_list(&self, agent_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let _ = agent_id;
        Err("Cron scheduler not available".to_string())
    }

    /// Cancel a cron job by ID.
    fn cron_cancel(&self, job_id: &str) -> Result<(), String> {
        let _ = job_id;
        Err("Cron scheduler not available".to_string())
    }

    /// Check if a tool requires approval based on current policy.
    fn requires_approval(&self, tool_name: &str) -> bool {
        let _ = tool_name;
        false
    }

    /// Request approval for a tool execution.
    /// Returns `Ok(true)` if approved, `Ok(false)` if denied or timed out.
    fn request_approval(
        &self,
        agent_id: &str,
        tool_name: &str,
        action_summary: &str,
    ) -> Result<bool, String> {
        let _ = (agent_id, tool_name, action_summary);
        Ok(true) // Default: auto-approve
    }

    /// List available Hands and their activation status.
    fn hand_list(&self) -> Result<Vec<serde_json::Value>, String> {
        Err("Hands system not available".to_string())
    }

    /// Activate a Hand — spawns a specialized autonomous agent.
    fn hand_activate(
        &self,
        hand_id: &str,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let _ = (hand_id, config);
        Err("Hands system not available".to_string())
    }

    /// Check the status and dashboard metrics of an active Hand.
    fn hand_status(&self, hand_id: &str) -> Result<serde_json::Value, String> {
        let _ = hand_id;
        Err("Hands system not available".to_string())
    }

    /// Deactivate a running Hand and stop its agent.
    fn hand_deactivate(&self, instance_id: &str) -> Result<(), String> {
        let _ = instance_id;
        Err("Hands system not available".to_string())
    }

    /// List discovered external A2A agents as (name, url) pairs.
    fn list_a2a_agents(&self) -> Vec<(String, String)> {
        vec![]
    }

    /// Get the URL of a discovered external A2A agent by name.
    fn get_a2a_agent_url(&self, name: &str) -> Option<String> {
        let _ = name;
        None
    }

    /// Send a message to a user on a named channel adapter.
    fn send_channel_message(
        &self,
        channel: &str,
        recipient: &str,
        message: &str,
    ) -> Result<String, String> {
        let _ = (channel, recipient, message);
        Err("Channel send not available".to_string())
    }

    /// Spawn an agent with capability inheritance enforcement.
    /// `parent_caps` are the parent's granted capabilities. The kernel MUST verify
    /// that every capability in the child manifest is covered by `parent_caps`.
    fn spawn_agent_checked(
        &self,
        manifest_toml: &str,
        parent_id: Option<&str>,
        parent_caps: &[crate::capability::Capability],
    ) -> Result<(String, String), String> {
        // Default: delegate to spawn_agent (no enforcement)
        let _ = parent_caps;
        self.spawn_agent(manifest_toml, parent_id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal mock kernel for testing.
    struct MockKernel {
        agents: Vec<AgentInfo>,
        memory: std::sync::Mutex<HashMap<String, serde_json::Value>>,
    }

    impl MockKernel {
        fn new() -> Self {
            Self {
                agents: vec![AgentInfo {
                    id: "agent-1".to_string(),
                    name: "TestAgent".to_string(),
                    state: "running".to_string(),
                    ..Default::default()
                }],
                memory: std::sync::Mutex::new(HashMap::new()),
            }
        }
    }

    impl KernelHandle for MockKernel {
        fn spawn_agent(
            &self,
            _manifest_toml: &str,
            _parent_id: Option<&str>,
        ) -> Result<(String, String), String> {
            Ok(("new-agent-id".to_string(), "NewAgent".to_string()))
        }

        fn send_to_agent(&self, agent_id: &str, message: &str) -> Result<String, String> {
            Ok(format!("Echo from {agent_id}: {message}"))
        }

        fn list_agents(&self) -> Vec<AgentInfo> {
            self.agents.clone()
        }

        fn kill_agent(&self, agent_id: &str) -> Result<(), String> {
            if agent_id == "agent-1" {
                Ok(())
            } else {
                Err("Agent not found".to_string())
            }
        }

        fn memory_store(&self, key: &str, value: serde_json::Value) -> Result<(), String> {
            let mut mem = self.memory.lock().unwrap();
            mem.insert(key.to_string(), value);
            Ok(())
        }

        fn memory_recall(&self, key: &str) -> Result<Option<serde_json::Value>, String> {
            let mem = self.memory.lock().unwrap();
            Ok(mem.get(key).cloned())
        }

        fn find_agents(&self, query: &str) -> Vec<AgentInfo> {
            let lower = query.to_lowercase();
            self.agents
                .iter()
                .filter(|a| a.name.to_lowercase().contains(&lower))
                .cloned()
                .collect()
        }
    }

    #[test]
    fn test_spawn_agent() {
        let kernel = MockKernel::new();
        let result = kernel.spawn_agent("[agent]\nname = \"child\"", None);
        assert!(result.is_ok());
        let (id, name) = result.unwrap();
        assert_eq!(id, "new-agent-id");
        assert_eq!(name, "NewAgent");
    }

    #[test]
    fn test_send_to_agent() {
        let kernel = MockKernel::new();
        let result = kernel.send_to_agent("agent-1", "hello");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Echo from agent-1"));
    }

    #[test]
    fn test_list_agents() {
        let kernel = MockKernel::new();
        let agents = kernel.list_agents();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "TestAgent");
    }

    #[test]
    fn test_kill_agent() {
        let kernel = MockKernel::new();
        assert!(kernel.kill_agent("agent-1").is_ok());
        assert!(kernel.kill_agent("nonexistent").is_err());
    }

    #[test]
    fn test_memory_store_and_recall() {
        let kernel = MockKernel::new();
        kernel
            .memory_store("key1", serde_json::json!("value1"))
            .unwrap();
        let value = kernel.memory_recall("key1").unwrap();
        assert_eq!(value, Some(serde_json::json!("value1")));

        let missing = kernel.memory_recall("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_find_agents() {
        let kernel = MockKernel::new();
        let found = kernel.find_agents("test");
        assert_eq!(found.len(), 1);
        let not_found = kernel.find_agents("nonexistent");
        assert!(not_found.is_empty());
    }

    #[test]
    fn test_default_implementations_return_errors() {
        let kernel = MockKernel::new();
        assert!(kernel.task_post("t", "d", None, None).is_err());
        assert!(kernel.task_claim("agent-1").is_err());
        assert!(kernel.task_complete("t-1", "done").is_err());
        assert!(kernel.task_list(None).is_err());
        assert!(kernel.publish_event("evt", serde_json::json!({})).is_err());
        assert!(kernel.cron_create("a", serde_json::json!({})).is_err());
        assert!(kernel.cron_list("a").is_err());
        assert!(kernel.cron_cancel("j").is_err());
        assert!(kernel.hand_list().is_err());
        assert!(kernel.hand_status("h").is_err());
        assert!(kernel.hand_deactivate("h").is_err());
        assert!(kernel.send_channel_message("email", "bob", "hi").is_err());
    }

    #[test]
    fn test_default_approval_auto_approves() {
        let kernel = MockKernel::new();
        assert!(!kernel.requires_approval("any_tool"));
        assert!(kernel.request_approval("a", "t", "summary").unwrap());
    }

    #[test]
    fn test_default_a2a_empty() {
        let kernel = MockKernel::new();
        assert!(kernel.list_a2a_agents().is_empty());
        assert!(kernel.get_a2a_agent_url("any").is_none());
    }

    #[test]
    fn test_agent_info_default() {
        let info = AgentInfo::default();
        assert_eq!(info.state, "idle");
        assert!(info.id.is_empty());
    }

    #[test]
    fn test_agent_info_serde() {
        let info = AgentInfo {
            id: "a1".to_string(),
            name: "Test".to_string(),
            state: "running".to_string(),
            model_provider: "anthropic".to_string(),
            model_name: "claude-sonnet-4".to_string(),
            description: "A test agent".to_string(),
            tags: vec!["test".to_string()],
            tools: vec!["web_search".to_string()],
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: AgentInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Test");
        assert_eq!(back.tags.len(), 1);
    }

    #[test]
    fn test_entity_serde() {
        let entity = Entity {
            name: "Rust".to_string(),
            entity_type: "language".to_string(),
            properties: HashMap::from([("year".to_string(), serde_json::json!(2010))]),
        };
        let json = serde_json::to_string(&entity).unwrap();
        let back: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Rust");
    }

    #[test]
    fn test_relation_serde() {
        let rel = Relation {
            from: "Ark".to_string(),
            relation_type: "written_in".to_string(),
            to: "Rust".to_string(),
            properties: HashMap::new(),
        };
        let json = serde_json::to_string(&rel).unwrap();
        let back: Relation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.relation_type, "written_in");
    }

    #[test]
    fn test_graph_pattern_default() {
        let pattern = GraphPattern::default();
        assert_eq!(pattern.max_results, 50);
        assert!(pattern.entity_name.is_none());
    }

    #[test]
    fn test_knowledge_default_errors() {
        let kernel = MockKernel::new();
        let entity = Entity {
            name: "Test".to_string(),
            entity_type: "concept".to_string(),
            properties: HashMap::new(),
        };
        assert!(kernel.knowledge_add_entity(entity).is_err());
    }

    #[test]
    fn test_spawn_agent_checked_delegates() {
        let kernel = MockKernel::new();
        let result = kernel.spawn_agent_checked("[agent]\nname=\"child\"", None, &[]);
        assert!(result.is_ok());
    }
}
