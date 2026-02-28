/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark agent message routing system.
 */

//! Agent router — routes incoming channel messages to the correct agent.
//!
//! Supports multiple routing layers with clear priority:
//! 1. **Binding rules** (most specific first, pattern-matching on channel/peer/guild/roles)
//! 2. **Direct routes** (exact `(channel, user)` → agent mapping)
//! 3. **User defaults** (per-user preferred agent)
//! 4. **System default** (global fallback)
//!
//! Also supports **broadcast routing** where a single message is fanned out
//! to multiple agents (parallel or sequential).

use crate::channel_types::ChannelType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Binding types (replacing openfang-types dependency)
// ---------------------------------------------------------------------------

/// Rule for matching incoming messages to agents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindingMatchRule {
    /// Channel type string (e.g., "telegram", "discord").
    pub channel: Option<String>,
    /// Account/bot ID within the channel.
    pub account_id: Option<String>,
    /// Peer/user ID.
    pub peer_id: Option<String>,
    /// Guild/server ID.
    pub guild_id: Option<String>,
    /// Required roles (user must have at least one).
    #[serde(default)]
    pub roles: Vec<String>,
}

impl BindingMatchRule {
    /// Calculate specificity score — more fields specified = higher score.
    /// Used to sort bindings so the most specific match wins.
    pub fn specificity(&self) -> u32 {
        let mut score = 0u32;
        if self.peer_id.is_some() {
            score += 8;
        }
        if self.guild_id.is_some() {
            score += 4;
        }
        if !self.roles.is_empty() {
            score += 2;
        }
        if self.channel.is_some() {
            score += 2;
        }
        if self.account_id.is_some() {
            score += 1;
        }
        score
    }
}

/// An agent binding — maps a match rule to an agent name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBinding {
    /// Agent name (resolved to ID via the name cache).
    pub agent: String,
    /// Match rule for this binding.
    pub match_rule: BindingMatchRule,
}

/// Broadcast strategy for multi-agent fan-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadcastStrategy {
    /// Send to all agents simultaneously.
    Parallel,
    /// Send to agents one at a time.
    Sequential,
}

impl Default for BroadcastStrategy {
    fn default() -> Self {
        Self::Parallel
    }
}

/// Broadcast configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BroadcastConfig {
    pub strategy: BroadcastStrategy,
    /// peer_id → list of agent names.
    pub routes: HashMap<String, Vec<String>>,
}

// ---------------------------------------------------------------------------
// BindingContext
// ---------------------------------------------------------------------------

/// Context for evaluating binding match rules against incoming messages.
#[derive(Debug, Default)]
pub struct BindingContext {
    pub channel: String,
    pub account_id: Option<String>,
    pub peer_id: String,
    pub guild_id: Option<String>,
    pub roles: Vec<String>,
}

// ---------------------------------------------------------------------------
// AgentRouter
// ---------------------------------------------------------------------------

/// Routes incoming messages to the correct agent.
///
/// Thread-safe via internal mutexes (no external `dashmap` dependency).
pub struct AgentRouter {
    /// Default agent per user (keyed by ark_user or platform_id).
    user_defaults: Mutex<HashMap<String, String>>,
    /// Direct routes: (channel_type_key, platform_user_id) → agent_id.
    direct_routes: Mutex<HashMap<(String, String), String>>,
    /// System-wide default agent.
    default_agent: Mutex<Option<String>>,
    /// Sorted bindings (most specific first).
    bindings: Mutex<Vec<(AgentBinding, String)>>,
    /// Broadcast configuration.
    broadcast: Mutex<BroadcastConfig>,
    /// Agent name → agent_id cache.
    agent_name_cache: Mutex<HashMap<String, String>>,
}

impl AgentRouter {
    /// Create a new router.
    pub fn new() -> Self {
        Self {
            user_defaults: Mutex::new(HashMap::new()),
            direct_routes: Mutex::new(HashMap::new()),
            default_agent: Mutex::new(None),
            bindings: Mutex::new(Vec::new()),
            broadcast: Mutex::new(BroadcastConfig::default()),
            agent_name_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Set the system-wide default agent.
    pub fn set_default(&self, agent_id: String) {
        *self.default_agent.lock().unwrap_or_else(|e| e.into_inner()) = Some(agent_id);
    }

    /// Set a user's default agent.
    pub fn set_user_default(&self, user_key: String, agent_id: String) {
        self.user_defaults
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(user_key, agent_id);
    }

    /// Set a direct route for a specific (channel, user) pair.
    pub fn set_direct_route(
        &self,
        channel_key: String,
        platform_user_id: String,
        agent_id: String,
    ) {
        self.direct_routes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((channel_key, platform_user_id), agent_id);
    }

    /// Load agent bindings from configuration. Sorts by specificity (most specific first).
    pub fn load_bindings(&self, bindings: &[AgentBinding]) {
        let mut sorted: Vec<(AgentBinding, String)> = bindings
            .iter()
            .map(|b| (b.clone(), b.agent.clone()))
            .collect();
        sorted.sort_by(|a, b| {
            b.0.match_rule
                .specificity()
                .cmp(&a.0.match_rule.specificity())
        });
        *self.bindings.lock().unwrap_or_else(|e| e.into_inner()) = sorted;
    }

    /// Load broadcast configuration.
    pub fn load_broadcast(&self, broadcast: BroadcastConfig) {
        *self.broadcast.lock().unwrap_or_else(|e| e.into_inner()) = broadcast;
    }

    /// Register an agent name → ID mapping for binding resolution.
    pub fn register_agent(&self, name: String, id: String) {
        self.agent_name_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name, id);
    }

    /// Resolve which agent should handle a message.
    ///
    /// Priority: bindings > direct route > user default > system default.
    pub fn resolve(
        &self,
        channel_type: &ChannelType,
        platform_user_id: &str,
        user_key: Option<&str>,
    ) -> Option<String> {
        let channel_key = format!("{:?}", channel_type);

        // 0. Check bindings (most specific first)
        let ctx = BindingContext {
            channel: channel_type.as_str().to_string(),
            account_id: None,
            peer_id: platform_user_id.to_string(),
            guild_id: None,
            roles: Vec::new(),
        };
        if let Some(agent_id) = self.resolve_binding(&ctx) {
            return Some(agent_id);
        }

        // 1. Check direct routes
        {
            let routes = self.direct_routes.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(agent) = routes.get(&(channel_key, platform_user_id.to_string())) {
                return Some(agent.clone());
            }
        }

        // 2. Check user defaults
        {
            let defaults = self.user_defaults.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(key) = user_key {
                if let Some(agent) = defaults.get(key) {
                    return Some(agent.clone());
                }
            }
            if let Some(agent) = defaults.get(platform_user_id) {
                return Some(agent.clone());
            }
        }

        // 3. System default
        self.default_agent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Resolve with full binding context (supports guild_id, roles, account_id).
    pub fn resolve_with_context(
        &self,
        channel_type: &ChannelType,
        platform_user_id: &str,
        user_key: Option<&str>,
        ctx: &BindingContext,
    ) -> Option<String> {
        if let Some(agent_id) = self.resolve_binding(ctx) {
            return Some(agent_id);
        }
        self.resolve(channel_type, platform_user_id, user_key)
    }

    /// Resolve broadcast: returns all agents that should receive a message.
    pub fn resolve_broadcast(&self, peer_id: &str) -> Vec<(String, Option<String>)> {
        let bc = self.broadcast.lock().unwrap_or_else(|e| e.into_inner());
        let cache = self
            .agent_name_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(agent_names) = bc.routes.get(peer_id) {
            agent_names
                .iter()
                .map(|name| {
                    let id = cache.get(name).cloned();
                    (name.clone(), id)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get broadcast strategy.
    pub fn broadcast_strategy(&self) -> BroadcastStrategy {
        self.broadcast
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .strategy
    }

    /// Check if a peer has broadcast routing configured.
    pub fn has_broadcast(&self, peer_id: &str) -> bool {
        self.broadcast
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .routes
            .contains_key(peer_id)
    }

    /// Get current bindings (read-only).
    pub fn get_bindings(&self) -> Vec<AgentBinding> {
        self.bindings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(b, _)| b.clone())
            .collect()
    }

    /// Add a single binding at runtime.
    pub fn add_binding(&self, binding: AgentBinding) {
        let name = binding.agent.clone();
        let mut bindings = self.bindings.lock().unwrap_or_else(|e| e.into_inner());
        bindings.push((binding, name));
        bindings.sort_by(|a, b| {
            b.0.match_rule
                .specificity()
                .cmp(&a.0.match_rule.specificity())
        });
    }

    /// Remove a binding by index.
    pub fn remove_binding(&self, index: usize) -> Option<AgentBinding> {
        let mut bindings = self.bindings.lock().unwrap_or_else(|e| e.into_inner());
        if index < bindings.len() {
            Some(bindings.remove(index).0)
        } else {
            None
        }
    }

    /// Evaluate bindings against a context, returning the first matching agent ID.
    fn resolve_binding(&self, ctx: &BindingContext) -> Option<String> {
        let bindings = self.bindings.lock().unwrap_or_else(|e| e.into_inner());
        let cache = self
            .agent_name_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for (binding, _agent_name) in bindings.iter() {
            if Self::binding_matches(binding, ctx) {
                if let Some(id) = cache.get(&binding.agent) {
                    return Some(id.clone());
                }
                // Binding matched but agent not found in cache — skip
            }
        }
        None
    }

    /// Check if a single binding's match_rule matches the context.
    fn binding_matches(binding: &AgentBinding, ctx: &BindingContext) -> bool {
        let rule = &binding.match_rule;

        if let Some(ref ch) = rule.channel {
            if ch != &ctx.channel {
                return false;
            }
        }
        if let Some(ref acc) = rule.account_id {
            if ctx.account_id.as_ref() != Some(acc) {
                return false;
            }
        }
        if let Some(ref pid) = rule.peer_id {
            if pid != &ctx.peer_id {
                return false;
            }
        }
        if let Some(ref gid) = rule.guild_id {
            if ctx.guild_id.as_ref() != Some(gid) {
                return false;
            }
        }
        if !rule.roles.is_empty() {
            let has_role = rule.roles.iter().any(|r| ctx.roles.contains(r));
            if !has_role {
                return false;
            }
        }
        true
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_priority() {
        let router = AgentRouter::new();
        let default_agent = "default-001".to_string();
        let user_agent = "user-001".to_string();
        let direct_agent = "direct-001".to_string();

        router.set_default(default_agent.clone());
        router.set_user_default("alice".to_string(), user_agent.clone());
        router.set_direct_route(
            "Telegram".to_string(),
            "tg_123".to_string(),
            direct_agent.clone(),
        );

        // Direct route wins
        let resolved = router.resolve(&ChannelType::Telegram, "tg_123", Some("alice"));
        assert_eq!(resolved, Some(direct_agent));

        // User default for non-direct-routed user
        let resolved = router.resolve(&ChannelType::WhatsApp, "wa_456", Some("alice"));
        assert_eq!(resolved, Some(user_agent));

        // System default for unknown user
        let resolved = router.resolve(&ChannelType::Discord, "dc_789", None);
        assert_eq!(resolved, Some(default_agent));
    }

    #[test]
    fn test_no_route() {
        let router = AgentRouter::new();
        let resolved = router.resolve(&ChannelType::CLI, "local", None);
        assert_eq!(resolved, None);
    }

    #[test]
    fn test_binding_channel_match() {
        let router = AgentRouter::new();
        router.register_agent("coder".to_string(), "coder-001".to_string());
        router.load_bindings(&[AgentBinding {
            agent: "coder".to_string(),
            match_rule: BindingMatchRule {
                channel: Some("telegram".to_string()),
                ..Default::default()
            },
        }]);

        let resolved = router.resolve(&ChannelType::Telegram, "user1", None);
        assert_eq!(resolved, Some("coder-001".to_string()));

        let resolved = router.resolve(&ChannelType::Discord, "user1", None);
        assert_eq!(resolved, None);
    }

    #[test]
    fn test_binding_peer_id_match() {
        let router = AgentRouter::new();
        router.register_agent("support".to_string(), "support-001".to_string());
        router.load_bindings(&[AgentBinding {
            agent: "support".to_string(),
            match_rule: BindingMatchRule {
                peer_id: Some("vip_user".to_string()),
                ..Default::default()
            },
        }]);

        let resolved = router.resolve(&ChannelType::Discord, "vip_user", None);
        assert_eq!(resolved, Some("support-001".to_string()));

        let resolved = router.resolve(&ChannelType::Discord, "other_user", None);
        assert_eq!(resolved, None);
    }

    #[test]
    fn test_binding_guild_and_role_match() {
        let router = AgentRouter::new();
        router.register_agent("admin-bot".to_string(), "admin-001".to_string());
        router.load_bindings(&[AgentBinding {
            agent: "admin-bot".to_string(),
            match_rule: BindingMatchRule {
                guild_id: Some("guild_123".to_string()),
                roles: vec!["admin".to_string()],
                ..Default::default()
            },
        }]);

        let ctx = BindingContext {
            channel: "discord".to_string(),
            peer_id: "user1".to_string(),
            guild_id: Some("guild_123".to_string()),
            roles: vec!["admin".to_string(), "user".to_string()],
            ..Default::default()
        };
        let resolved = router.resolve_with_context(&ChannelType::Discord, "user1", None, &ctx);
        assert_eq!(resolved, Some("admin-001".to_string()));

        // Wrong guild
        let ctx2 = BindingContext {
            channel: "discord".to_string(),
            peer_id: "user1".to_string(),
            guild_id: Some("guild_999".to_string()),
            roles: vec!["admin".to_string()],
            ..Default::default()
        };
        let resolved = router.resolve_with_context(&ChannelType::Discord, "user1", None, &ctx2);
        assert_eq!(resolved, None);
    }

    #[test]
    fn test_binding_specificity_ordering() {
        let router = AgentRouter::new();
        router.register_agent("general".to_string(), "general-001".to_string());
        router.register_agent("specific".to_string(), "specific-001".to_string());

        router.load_bindings(&[
            AgentBinding {
                agent: "general".to_string(),
                match_rule: BindingMatchRule {
                    channel: Some("discord".to_string()),
                    ..Default::default()
                },
            },
            AgentBinding {
                agent: "specific".to_string(),
                match_rule: BindingMatchRule {
                    channel: Some("discord".to_string()),
                    peer_id: Some("user1".to_string()),
                    guild_id: Some("guild_1".to_string()),
                    ..Default::default()
                },
            },
        ]);

        let ctx = BindingContext {
            channel: "discord".to_string(),
            peer_id: "user1".to_string(),
            guild_id: Some("guild_1".to_string()),
            ..Default::default()
        };
        let resolved = router.resolve_with_context(&ChannelType::Discord, "user1", None, &ctx);
        assert_eq!(resolved, Some("specific-001".to_string()));
    }

    #[test]
    fn test_broadcast_routing() {
        let router = AgentRouter::new();
        router.register_agent("agent-a".to_string(), "a-001".to_string());
        router.register_agent("agent-b".to_string(), "b-001".to_string());

        let mut routes = HashMap::new();
        routes.insert(
            "vip_user".to_string(),
            vec!["agent-a".to_string(), "agent-b".to_string()],
        );
        router.load_broadcast(BroadcastConfig {
            strategy: BroadcastStrategy::Parallel,
            routes,
        });

        assert!(router.has_broadcast("vip_user"));
        assert!(!router.has_broadcast("normal_user"));

        let targets = router.resolve_broadcast("vip_user");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].0, "agent-a");
        assert_eq!(targets[0].1, Some("a-001".to_string()));
        assert_eq!(targets[1].0, "agent-b");
        assert_eq!(targets[1].1, Some("b-001".to_string()));
    }

    #[test]
    fn test_empty_bindings_legacy_behavior() {
        let router = AgentRouter::new();
        router.set_default("default-001".to_string());
        router.load_bindings(&[]);

        let resolved = router.resolve(&ChannelType::Telegram, "user1", None);
        assert_eq!(resolved, Some("default-001".to_string()));
    }

    #[test]
    fn test_binding_nonexistent_agent() {
        let router = AgentRouter::new();
        router.load_bindings(&[AgentBinding {
            agent: "ghost-agent".to_string(),
            match_rule: BindingMatchRule {
                channel: Some("telegram".to_string()),
                ..Default::default()
            },
        }]);

        let resolved = router.resolve(&ChannelType::Telegram, "user1", None);
        assert_eq!(resolved, None);
    }

    #[test]
    fn test_add_remove_binding() {
        let router = AgentRouter::new();
        router.register_agent("test".to_string(), "test-001".to_string());

        assert!(router.get_bindings().is_empty());

        router.add_binding(AgentBinding {
            agent: "test".to_string(),
            match_rule: BindingMatchRule {
                channel: Some("slack".to_string()),
                ..Default::default()
            },
        });
        assert_eq!(router.get_bindings().len(), 1);

        let removed = router.remove_binding(0);
        assert!(removed.is_some());
        assert!(router.get_bindings().is_empty());
    }

    #[test]
    fn test_binding_specificity_scores() {
        let empty = BindingMatchRule::default();
        assert_eq!(empty.specificity(), 0);

        let channel_only = BindingMatchRule {
            channel: Some("discord".to_string()),
            ..Default::default()
        };
        assert_eq!(channel_only.specificity(), 2);

        let full = BindingMatchRule {
            channel: Some("discord".to_string()),
            peer_id: Some("user".to_string()),
            guild_id: Some("guild".to_string()),
            roles: vec!["admin".to_string()],
            account_id: Some("bot".to_string()),
        };
        assert_eq!(full.specificity(), 17); // 8+4+2+2+1
    }

    #[test]
    fn test_wire_channel_routing() {
        let router = AgentRouter::new();
        router.register_agent("wire-agent".to_string(), "wire-001".to_string());
        router.load_bindings(&[AgentBinding {
            agent: "wire-agent".to_string(),
            match_rule: BindingMatchRule {
                channel: Some("wire".to_string()),
                ..Default::default()
            },
        }]);

        let resolved = router.resolve(&ChannelType::Wire, "peer-abc", None);
        assert_eq!(resolved, Some("wire-001".to_string()));
    }
}
