/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark semantic memory layer.
 */

//! Semantic memory — fragment storage, knowledge graph, and consolidation.
//!
//! Provides:
//! - `MemoryFragment` — a unit of semantic memory with embedding, source, confidence
//! - `MemoryFilter` — criteria for filtering memory recall
//! - `MemorySource` — where a memory came from (conversation, document, etc.)
//! - `ConsolidationReport` / `ImportReport` — operation result types
//! - `ConsolidationEngine` — in-memory confidence decay + merge engine
//! - Knowledge graph types: `Entity`, `Relation`, `EntityType`, `RelationType`,
//!   `GraphPattern`, `GraphMatch`
//!
//! **Ark adaptation**: Removed `async_trait` (all sync), `chrono` (→ `u64`
//! epoch seconds), `uuid` (→ `String` IDs), `rusqlite` (→ in-memory store),
//! `OpenFangResult` (→ `Result<T, String>`), `AgentId` (→ `String`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// MemoryId — simple string-based ID (replaces uuid::Uuid)
// ---------------------------------------------------------------------------

/// Unique identifier for a memory fragment.
///
/// Uses a simple incrementing counter in Ark (no `uuid` dep).
/// For production, the kernel provides a UUID allocator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub String);

impl MemoryId {
    /// Create a new MemoryId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Create a MemoryId from a numeric counter.
    pub fn from_counter(n: u64) -> Self {
        Self(format!("mem-{n:016x}"))
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self("mem-0000000000000000".to_string())
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// MemorySource
// ---------------------------------------------------------------------------

/// Where a memory came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    /// From a conversation/interaction.
    Conversation,
    /// From a document that was processed.
    Document,
    /// From an observation (tool output, web page, etc.).
    Observation,
    /// Inferred by the agent from existing knowledge.
    Inference,
    /// Explicitly provided by the user.
    UserProvided,
    /// From a system event.
    System,
}

// ---------------------------------------------------------------------------
// MemoryFragment
// ---------------------------------------------------------------------------

/// A single unit of memory stored in the semantic store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFragment {
    /// Unique ID.
    pub id: MemoryId,
    /// Which agent owns this memory (agent ID string).
    pub agent_id: String,
    /// The textual content of this memory.
    pub content: String,
    /// Vector embedding (populated by the semantic store).
    pub embedding: Option<Vec<f32>>,
    /// Arbitrary metadata.
    pub metadata: HashMap<String, serde_json::Value>,
    /// How this memory was created.
    pub source: MemorySource,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// When this memory was created (Unix epoch seconds).
    pub created_at: u64,
    /// When this memory was last accessed (Unix epoch seconds).
    pub accessed_at: u64,
    /// How many times this memory has been accessed.
    pub access_count: u64,
    /// Memory scope/collection name.
    pub scope: String,
    /// Soft-delete flag.
    pub deleted: bool,
}

impl Default for MemoryFragment {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: MemoryId::default(),
            agent_id: String::new(),
            content: String::new(),
            embedding: None,
            metadata: HashMap::new(),
            source: MemorySource::Conversation,
            confidence: 1.0,
            created_at: now,
            accessed_at: now,
            access_count: 0,
            scope: "episodic".to_string(),
            deleted: false,
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryFilter
// ---------------------------------------------------------------------------

/// Filter criteria for memory recall.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryFilter {
    /// Filter by agent ID.
    pub agent_id: Option<String>,
    /// Filter by source type.
    pub source: Option<MemorySource>,
    /// Filter by scope.
    pub scope: Option<String>,
    /// Minimum confidence threshold.
    pub min_confidence: Option<f32>,
    /// Only memories created after this time (Unix epoch seconds).
    pub after: Option<u64>,
    /// Only memories created before this time (Unix epoch seconds).
    pub before: Option<u64>,
    /// Metadata key-value filters.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MemoryFilter {
    /// Create a filter for a specific agent.
    pub fn agent(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: Some(agent_id.into()),
            ..Default::default()
        }
    }

    /// Create a filter for a specific scope.
    pub fn scope(scope: impl Into<String>) -> Self {
        Self {
            scope: Some(scope.into()),
            ..Default::default()
        }
    }

    /// Test whether a memory fragment matches this filter.
    pub fn matches(&self, fragment: &MemoryFragment) -> bool {
        if fragment.deleted {
            return false;
        }
        if let Some(ref aid) = self.agent_id {
            if &fragment.agent_id != aid {
                return false;
            }
        }
        if let Some(ref src) = self.source {
            if &fragment.source != src {
                return false;
            }
        }
        if let Some(ref scp) = self.scope {
            if &fragment.scope != scp {
                return false;
            }
        }
        if let Some(min_conf) = self.min_confidence {
            if fragment.confidence < min_conf {
                return false;
            }
        }
        if let Some(after) = self.after {
            if fragment.created_at < after {
                return false;
            }
        }
        if let Some(before) = self.before {
            if fragment.created_at > before {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Knowledge Graph types
// ---------------------------------------------------------------------------

/// Types of entities in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Organization,
    Project,
    Concept,
    Event,
    Location,
    Document,
    Tool,
    Custom(String),
}

/// An entity in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique entity ID.
    pub id: String,
    /// Entity type.
    pub entity_type: EntityType,
    /// Display name.
    pub name: String,
    /// Arbitrary properties.
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    /// When created (Unix epoch seconds).
    pub created_at: u64,
    /// When last updated (Unix epoch seconds).
    pub updated_at: u64,
}

/// Types of relations in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    WorksAt,
    KnowsAbout,
    RelatedTo,
    DependsOn,
    OwnedBy,
    CreatedBy,
    LocatedIn,
    PartOf,
    Uses,
    Produces,
    Custom(String),
}

/// A relation between two entities in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Source entity ID.
    pub source: String,
    /// Relation type.
    pub relation: RelationType,
    /// Target entity ID.
    pub target: String,
    /// Arbitrary properties on the relation.
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// When created (Unix epoch seconds).
    pub created_at: u64,
}

/// A pattern for querying the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPattern {
    /// Optional source entity filter.
    pub source: Option<String>,
    /// Optional relation type filter.
    pub relation: Option<RelationType>,
    /// Optional target entity filter.
    pub target: Option<String>,
    /// Maximum traversal depth.
    pub max_depth: u32,
}

impl Default for GraphPattern {
    fn default() -> Self {
        Self {
            source: None,
            relation: None,
            target: None,
            max_depth: 2,
        }
    }
}

/// A result from a graph query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMatch {
    /// The source entity.
    pub source: Entity,
    /// The relation.
    pub relation: Relation,
    /// The target entity.
    pub target: Entity,
}

// ---------------------------------------------------------------------------
// Consolidation & export/import types
// ---------------------------------------------------------------------------

/// Report from memory consolidation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidationReport {
    /// Number of memories merged.
    pub memories_merged: u64,
    /// Number of memories whose confidence decayed.
    pub memories_decayed: u64,
    /// How long the consolidation took (ms).
    pub duration_ms: u64,
}

/// Format for memory export/import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format.
    Json,
    /// MessagePack binary format.
    MessagePack,
}

/// Report from memory import.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportReport {
    /// Number of entities imported.
    pub entities_imported: u64,
    /// Number of relations imported.
    pub relations_imported: u64,
    /// Number of memories imported.
    pub memories_imported: u64,
    /// Errors encountered during import.
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// ConsolidationEngine — in-memory confidence decay
// ---------------------------------------------------------------------------

/// Memory consolidation engine.
///
/// Performs confidence decay on old, unaccessed memories.
/// In Ark, this operates on an in-memory store (no `rusqlite` dep).
/// The kernel can wire this to Fernet's persistent memory layer.
pub struct ConsolidationEngine {
    /// Decay rate: how much to reduce confidence per cycle (0.0–1.0).
    decay_rate: f32,
    /// Memories considered "stale" after this many seconds without access.
    stale_threshold_secs: u64,
    /// Minimum confidence floor (memories never drop below this).
    min_confidence: f32,
}

impl ConsolidationEngine {
    /// Create a new consolidation engine.
    pub fn new(decay_rate: f32) -> Self {
        Self {
            decay_rate: decay_rate.clamp(0.0, 1.0),
            stale_threshold_secs: 7 * 24 * 3600, // 7 days
            min_confidence: 0.1,
        }
    }

    /// Set the stale threshold in seconds.
    pub fn with_stale_threshold(mut self, secs: u64) -> Self {
        self.stale_threshold_secs = secs;
        self
    }

    /// Set the minimum confidence floor.
    pub fn with_min_confidence(mut self, min: f32) -> Self {
        self.min_confidence = min.clamp(0.0, 1.0);
        self
    }

    /// Run a consolidation cycle on a mutable slice of memories.
    ///
    /// Returns a report of how many memories were decayed.
    pub fn consolidate(&self, memories: &mut [MemoryFragment]) -> ConsolidationReport {
        let start = std::time::Instant::now();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cutoff = now.saturating_sub(self.stale_threshold_secs);
        let decay_factor = 1.0 - self.decay_rate;
        let mut decayed = 0u64;

        for mem in memories.iter_mut() {
            if mem.deleted {
                continue;
            }
            if mem.accessed_at < cutoff && mem.confidence > self.min_confidence {
                mem.confidence = (mem.confidence * decay_factor).max(self.min_confidence);
                decayed += 1;
            }
        }

        ConsolidationReport {
            memories_merged: 0, // Phase 1: no merging logic
            memories_decayed: decayed,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

impl Default for ConsolidationEngine {
    fn default() -> Self {
        Self::new(0.1)
    }
}

// ---------------------------------------------------------------------------
// In-memory store (simple container for testing / kernel integration)
// ---------------------------------------------------------------------------

/// Simple in-memory semantic memory store.
///
/// Thread-safe via `Mutex`. For production, the kernel can replace this
/// with Fernet's persistent store.
pub struct MemoryStore {
    fragments: Mutex<Vec<MemoryFragment>>,
    counter: Mutex<u64>,
}

impl MemoryStore {
    /// Create a new empty memory store.
    pub fn new() -> Self {
        Self {
            fragments: Mutex::new(Vec::new()),
            counter: Mutex::new(0),
        }
    }

    /// Store a new memory fragment. Returns the assigned ID.
    pub fn remember(
        &self,
        agent_id: &str,
        content: &str,
        source: MemorySource,
        scope: &str,
    ) -> MemoryId {
        let mut counter = self.counter.lock().unwrap_or_else(|e| e.into_inner());
        let id = MemoryId::from_counter(*counter);
        *counter += 1;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let fragment = MemoryFragment {
            id: id.clone(),
            agent_id: agent_id.to_string(),
            content: content.to_string(),
            embedding: None,
            metadata: HashMap::new(),
            source,
            confidence: 1.0,
            created_at: now,
            accessed_at: now,
            access_count: 0,
            scope: scope.to_string(),
            deleted: false,
        };

        self.fragments
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(fragment);
        id
    }

    /// Recall memories matching a filter.
    pub fn recall(&self, filter: &MemoryFilter, limit: usize) -> Vec<MemoryFragment> {
        let frags = self.fragments.lock().unwrap_or_else(|e| e.into_inner());
        frags
            .iter()
            .filter(|f| filter.matches(f))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Soft-delete a memory by ID.
    pub fn forget(&self, id: &MemoryId) -> bool {
        let mut frags = self.fragments.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(frag) = frags.iter_mut().find(|f| &f.id == id) {
            frag.deleted = true;
            true
        } else {
            false
        }
    }

    /// Run consolidation on all memories.
    pub fn consolidate(&self, engine: &ConsolidationEngine) -> ConsolidationReport {
        let mut frags = self.fragments.lock().unwrap_or_else(|e| e.into_inner());
        engine.consolidate(&mut frags)
    }

    /// Count of non-deleted memories.
    pub fn len(&self) -> usize {
        let frags = self.fragments.lock().unwrap_or_else(|e| e.into_inner());
        frags.iter().filter(|f| !f.deleted).count()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for MemoryStore {
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

    #[test]
    fn test_memory_id_display() {
        let id = MemoryId::from_counter(42);
        assert_eq!(id.to_string(), "mem-000000000000002a");
    }

    #[test]
    fn test_memory_id_default() {
        let id = MemoryId::default();
        assert_eq!(id.0, "mem-0000000000000000");
    }

    #[test]
    fn test_memory_fragment_default() {
        let frag = MemoryFragment::default();
        assert_eq!(frag.confidence, 1.0);
        assert_eq!(frag.scope, "episodic");
        assert!(!frag.deleted);
    }

    #[test]
    fn test_memory_fragment_serde() {
        let frag = MemoryFragment {
            id: MemoryId::new("test-1"),
            agent_id: "agent-1".to_string(),
            content: "Test memory".to_string(),
            embedding: Some(vec![0.1, 0.2, 0.3]),
            metadata: HashMap::new(),
            source: MemorySource::Conversation,
            confidence: 0.95,
            created_at: 1000,
            accessed_at: 1000,
            access_count: 0,
            scope: "episodic".to_string(),
            deleted: false,
        };
        let json = serde_json::to_string(&frag).unwrap();
        let back: MemoryFragment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.content, "Test memory");
        assert_eq!(back.confidence, 0.95);
    }

    #[test]
    fn test_memory_filter_agent() {
        let filter = MemoryFilter::agent("agent-1");
        assert_eq!(filter.agent_id, Some("agent-1".to_string()));
        assert!(filter.source.is_none());
    }

    #[test]
    fn test_memory_filter_scope() {
        let filter = MemoryFilter::scope("semantic");
        assert_eq!(filter.scope, Some("semantic".to_string()));
    }

    #[test]
    fn test_filter_matches() {
        let frag = MemoryFragment {
            agent_id: "a1".to_string(),
            scope: "episodic".to_string(),
            confidence: 0.8,
            source: MemorySource::Conversation,
            ..Default::default()
        };

        // Agent match
        assert!(MemoryFilter::agent("a1").matches(&frag));
        assert!(!MemoryFilter::agent("a2").matches(&frag));

        // Scope match
        assert!(MemoryFilter::scope("episodic").matches(&frag));
        assert!(!MemoryFilter::scope("semantic").matches(&frag));

        // Confidence threshold
        let filter = MemoryFilter {
            min_confidence: Some(0.9),
            ..Default::default()
        };
        assert!(!filter.matches(&frag));

        // Deleted fragments never match
        let mut deleted = frag.clone();
        deleted.deleted = true;
        assert!(!MemoryFilter::default().matches(&deleted));
    }

    #[test]
    fn test_memory_source_serde() {
        let src = MemorySource::Observation;
        let json = serde_json::to_string(&src).unwrap();
        assert_eq!(json, "\"observation\"");
        let back: MemorySource = serde_json::from_str(&json).unwrap();
        assert_eq!(back, MemorySource::Observation);
    }

    #[test]
    fn test_entity_type_serde() {
        let et = EntityType::Custom("workflow".to_string());
        let json = serde_json::to_string(&et).unwrap();
        let back: EntityType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, EntityType::Custom("workflow".to_string()));
    }

    #[test]
    fn test_relation_type_serde() {
        let rt = RelationType::DependsOn;
        let json = serde_json::to_string(&rt).unwrap();
        assert_eq!(json, "\"depends_on\"");
    }

    #[test]
    fn test_graph_pattern_default() {
        let pat = GraphPattern::default();
        assert_eq!(pat.max_depth, 2);
        assert!(pat.source.is_none());
    }

    #[test]
    fn test_consolidation_engine_default() {
        let engine = ConsolidationEngine::default();
        assert!((engine.decay_rate - 0.1).abs() < f32::EPSILON);
        assert_eq!(engine.stale_threshold_secs, 7 * 24 * 3600);
    }

    #[test]
    fn test_consolidation_empty() {
        let engine = ConsolidationEngine::new(0.1);
        let mut memories: Vec<MemoryFragment> = vec![];
        let report = engine.consolidate(&mut memories);
        assert_eq!(report.memories_decayed, 0);
    }

    #[test]
    fn test_consolidation_decays_stale() {
        let engine = ConsolidationEngine::new(0.1).with_stale_threshold(1); // 1 second

        let old_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 10; // 10 seconds ago

        let mut memories = vec![MemoryFragment {
            id: MemoryId::new("m1"),
            confidence: 0.9,
            accessed_at: old_time,
            ..Default::default()
        }];

        let report = engine.consolidate(&mut memories);
        assert_eq!(report.memories_decayed, 1);
        // 0.9 * 0.9 = 0.81
        assert!((memories[0].confidence - 0.81).abs() < 0.01);
    }

    #[test]
    fn test_consolidation_respects_min_confidence() {
        let engine = ConsolidationEngine::new(0.99)
            .with_stale_threshold(0)
            .with_min_confidence(0.1);

        let old_time = 0; // Very old

        let mut memories = vec![MemoryFragment {
            id: MemoryId::new("m1"),
            confidence: 0.15,
            accessed_at: old_time,
            ..Default::default()
        }];

        engine.consolidate(&mut memories);
        assert!(memories[0].confidence >= 0.1);
    }

    #[test]
    fn test_consolidation_skips_deleted() {
        let engine = ConsolidationEngine::new(0.1).with_stale_threshold(0);
        let mut memories = vec![MemoryFragment {
            deleted: true,
            confidence: 0.9,
            accessed_at: 0,
            ..Default::default()
        }];
        let report = engine.consolidate(&mut memories);
        assert_eq!(report.memories_decayed, 0);
    }

    #[test]
    fn test_memory_store_remember_and_recall() {
        let store = MemoryStore::new();
        let id = store.remember("a1", "hello world", MemorySource::Conversation, "episodic");

        let results = store.recall(&MemoryFilter::agent("a1"), 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
        assert_eq!(results[0].content, "hello world");
    }

    #[test]
    fn test_memory_store_forget() {
        let store = MemoryStore::new();
        let id = store.remember("a1", "secret", MemorySource::UserProvided, "episodic");
        assert_eq!(store.len(), 1);

        assert!(store.forget(&id));
        assert_eq!(store.len(), 0); // Soft-deleted, doesn't count

        // Recall should return empty now
        let results = store.recall(&MemoryFilter::agent("a1"), 10);
        assert!(results.is_empty());

        // Forgetting again returns false
        assert!(!store.forget(&MemoryId::new("nonexistent")));
    }

    #[test]
    fn test_memory_store_consolidate() {
        let store = MemoryStore::new();
        store.remember("a1", "old memory", MemorySource::Conversation, "test");

        // Manually set accessed_at to old
        {
            let mut frags = store.fragments.lock().unwrap();
            frags[0].accessed_at = 0; // Very old
        }

        let engine = ConsolidationEngine::new(0.1).with_stale_threshold(1);
        let report = store.consolidate(&engine);
        assert_eq!(report.memories_decayed, 1);
    }

    #[test]
    fn test_memory_store_empty() {
        let store = MemoryStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_consolidation_report_serde() {
        let report = ConsolidationReport {
            memories_merged: 5,
            memories_decayed: 10,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: ConsolidationReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.memories_decayed, 10);
    }

    #[test]
    fn test_import_report_default() {
        let report = ImportReport::default();
        assert_eq!(report.entities_imported, 0);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_export_format_serde() {
        let fmt = ExportFormat::Json;
        let json = serde_json::to_string(&fmt).unwrap();
        let back: ExportFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ExportFormat::Json);
    }
}
