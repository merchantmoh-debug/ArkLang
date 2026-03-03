/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Research & Tools Module — Intent Detection, Tool Registry, Research Engine.
 *
 * Selective port of Remember-Me-AI's integrations/tools.py + agent.py.
 * Ports the orchestration logic (intent detection, tool dispatch, pipeline)
 * without external I/O deps (DDG, Diffusers, pyttsx3).
 *
 * Architecture:
 *   Intent        → Enum (Search, Image, Code, General)
 *   IntentDetector→ Regex-based classifier for user input
 *   ToolSpec      → Tool specification (name, description, parameters)
 *   ToolRegistry  → HashMap-backed registry of available tools
 *   ResearchEngine→ Multi-step pipeline: detect → gather → execute → synthesize
 */

use std::collections::HashMap;

// ===========================================================================
// Intent Detection
// ===========================================================================

/// Detected intent from user input.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Intent {
    Search,
    Image,
    Code,
    General,
}

impl Intent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Intent::Search => "SEARCH",
            Intent::Image => "IMAGE",
            Intent::Code => "CODE",
            Intent::General => "GENERAL",
        }
    }
}

/// Regex-free intent detector using keyword matching.
/// Mirrors Python SovereignAgent._detect_intents() without regex crate dep.
#[derive(Debug)]
pub struct IntentDetector {
    search_keywords: Vec<&'static str>,
    image_keywords: Vec<&'static str>,
    code_keywords: Vec<&'static str>,
}

impl IntentDetector {
    pub fn new() -> Self {
        Self {
            search_keywords: vec![
                "search", "research", "find out", "what is", "who is", "latest", "news", "look up",
                "lookup", "find",
            ],
            image_keywords: vec![
                "draw",
                "generate an image",
                "generate image",
                "picture of",
                "visualize",
                "paint",
                "sketch",
                "render",
            ],
            code_keywords: vec![
                "calculate",
                "compute",
                "python",
                "code",
                "math",
                "algorithm",
                "solve",
                "evaluate",
                "execute",
            ],
        }
    }

    /// Detect all intents in a user input string. Returns unique intents.
    pub fn detect(&self, text: &str) -> Vec<Intent> {
        let lower = text.to_lowercase();
        let mut intents = Vec::new();

        if self.search_keywords.iter().any(|kw| lower.contains(kw)) {
            intents.push(Intent::Search);
        }
        if self.image_keywords.iter().any(|kw| lower.contains(kw)) {
            intents.push(Intent::Image);
        }
        if self.code_keywords.iter().any(|kw| lower.contains(kw)) {
            intents.push(Intent::Code);
        }
        if intents.is_empty() {
            intents.push(Intent::General);
        }

        intents
    }

    /// Check if a specific intent is present in the text.
    pub fn has_intent(&self, text: &str, intent: &Intent) -> bool {
        self.detect(text).contains(intent)
    }
}

impl Default for IntentDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tool Registry
// ===========================================================================

/// Specification for a registered tool.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParam>,
    pub intent: Intent,
    pub enabled: bool,
}

/// Parameter specification for a tool.
#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

impl ToolSpec {
    pub fn new(name: &str, description: &str, intent: Intent) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters: Vec::new(),
            intent,
            enabled: true,
        }
    }

    pub fn with_param(mut self, name: &str, param_type: &str, required: bool, desc: &str) -> Self {
        self.parameters.push(ToolParam {
            name: name.to_string(),
            param_type: param_type.to_string(),
            required,
            description: desc.to_string(),
        });
        self
    }
}

/// Registry of available tools.
#[derive(Debug)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolSpec>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    /// Register default tools (mirrors ToolArsenal capabilities).
    fn register_defaults(&mut self) {
        self.register(
            ToolSpec::new(
                "web_search",
                "Search the web using DuckDuckGo",
                Intent::Search,
            )
            .with_param("query", "String", true, "Search query")
            .with_param(
                "max_results",
                "Integer",
                false,
                "Maximum results (default: 3)",
            ),
        );
        self.register(
            ToolSpec::new(
                "generate_image",
                "Generate an image from text description",
                Intent::Image,
            )
            .with_param("prompt", "String", true, "Image description")
            .with_param("output_path", "String", false, "Output file path"),
        );
        self.register(
            ToolSpec::new(
                "execute_code",
                "Execute code in a sandboxed environment",
                Intent::Code,
            )
            .with_param("code", "String", true, "Code to execute")
            .with_param("language", "String", false, "Language (default: ark)"),
        );
        self.register(
            ToolSpec::new(
                "speak",
                "Text-to-speech using local TTS engine",
                Intent::General,
            )
            .with_param("text", "String", true, "Text to speak"),
        );
    }

    /// Register a tool.
    pub fn register(&mut self, spec: ToolSpec) {
        self.tools.insert(spec.name.clone(), spec);
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&ToolSpec> {
        self.tools.get(name)
    }

    /// Find tools matching a detected intent.
    pub fn find_by_intent(&self, intent: &Intent) -> Vec<&ToolSpec> {
        self.tools
            .values()
            .filter(|t| t.enabled && &t.intent == intent)
            .collect()
    }

    /// List all registered tool names.
    pub fn list_names(&self) -> Vec<&str> {
        self.tools.keys().map(|k| k.as_str()).collect()
    }

    /// Number of registered tools.
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Research Engine — Multi-step pipeline
// ===========================================================================

/// Result of a research pipeline step.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_name: String,
    pub intent: Intent,
    pub output: String,
    pub tool_used: Option<String>,
    pub success: bool,
}

/// Outcome of a research pipeline execution.
#[derive(Debug, Clone)]
pub struct ResearchResult {
    pub query: String,
    pub intents: Vec<Intent>,
    pub steps: Vec<StepResult>,
    pub synthesis: String,
}

/// Multi-step research pipeline.
/// Mirrors SovereignAgent.run() orchestration pattern:
///   1. Detect intents
///   2. For each intent → find matching tools → record step
///   3. Synthesize results
#[derive(Debug)]
pub struct ResearchEngine {
    pub detector: IntentDetector,
    pub registry: ToolRegistry,
    total_queries: u64,
}

impl ResearchEngine {
    pub fn new() -> Self {
        Self {
            detector: IntentDetector::new(),
            registry: ToolRegistry::new(),
            total_queries: 0,
        }
    }

    /// Plan an execution for a query: detect intents, find tools, build pipeline.
    /// Does NOT execute tools (that requires runtime I/O).
    pub fn plan(&mut self, query: &str) -> ResearchResult {
        self.total_queries += 1;
        let intents = self.detector.detect(query);
        let mut steps = Vec::new();

        for intent in intents.iter() {
            let tools = self.registry.find_by_intent(intent);
            if tools.is_empty() {
                steps.push(StepResult {
                    step_name: format!("{}:no_tool", intent.as_str()),
                    intent: intent.clone(),
                    output: "No tool available for this intent.".to_string(),
                    tool_used: None,
                    success: false,
                });
            } else {
                for tool in tools {
                    steps.push(StepResult {
                        step_name: format!("{}:{}", intent.as_str(), tool.name),
                        intent: intent.clone(),
                        output: format!("Ready to execute '{}': {}", tool.name, tool.description),
                        tool_used: Some(tool.name.clone()),
                        success: true,
                    });
                }
            }
        }

        let synthesis = format!(
            "Plan: {} intent(s) detected, {} step(s) queued for query: '{}'",
            intents.len(),
            steps.len(),
            if query.len() > 50 {
                &query[..50]
            } else {
                query
            }
        );

        ResearchResult {
            query: query.to_string(),
            intents,
            steps,
            synthesis,
        }
    }

    /// Get engine statistics.
    pub fn stats(&self) -> ResearchStats {
        ResearchStats {
            total_queries: self.total_queries,
            tool_count: self.registry.count(),
            tool_names: self
                .registry
                .list_names()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

impl Default for ResearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ResearchStats {
    pub total_queries: u64,
    pub tool_count: usize,
    pub tool_names: Vec<String>,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_search() {
        let det = IntentDetector::new();
        let intents = det.detect("search for quantum computing papers");
        assert!(intents.contains(&Intent::Search));
    }

    #[test]
    fn test_intent_image() {
        let det = IntentDetector::new();
        let intents = det.detect("draw a cat sitting on a moon");
        assert!(intents.contains(&Intent::Image));
    }

    #[test]
    fn test_intent_code() {
        let det = IntentDetector::new();
        let intents = det.detect("calculate the fibonacci sequence");
        assert!(intents.contains(&Intent::Code));
    }

    #[test]
    fn test_intent_general() {
        let det = IntentDetector::new();
        let intents = det.detect("hello, how are you?");
        assert_eq!(intents, vec![Intent::General]);
    }

    #[test]
    fn test_intent_multi() {
        let det = IntentDetector::new();
        let intents = det.detect("search for data and calculate the average");
        assert!(intents.contains(&Intent::Search));
        assert!(intents.contains(&Intent::Code));
    }

    #[test]
    fn test_intent_case_insensitive() {
        let det = IntentDetector::new();
        let intents = det.detect("SEARCH for QUANTUM Computing");
        assert!(intents.contains(&Intent::Search));
    }

    #[test]
    fn test_tool_registry_defaults() {
        let reg = ToolRegistry::new();
        assert!(reg.count() >= 4, "should have at least 4 default tools");
        assert!(reg.get("web_search").is_some());
        assert!(reg.get("generate_image").is_some());
        assert!(reg.get("execute_code").is_some());
        assert!(reg.get("speak").is_some());
    }

    #[test]
    fn test_tool_registry_by_intent() {
        let reg = ToolRegistry::new();
        let search_tools = reg.find_by_intent(&Intent::Search);
        assert!(!search_tools.is_empty(), "should find search tools");
        assert_eq!(search_tools[0].name, "web_search");
    }

    #[test]
    fn test_tool_spec_builder() {
        let spec = ToolSpec::new("test_tool", "A test tool", Intent::Code)
            .with_param("input", "String", true, "Input data")
            .with_param("verbose", "Boolean", false, "Verbose mode");
        assert_eq!(spec.parameters.len(), 2);
        assert!(spec.parameters[0].required);
        assert!(!spec.parameters[1].required);
    }

    #[test]
    fn test_research_plan_search() {
        let mut engine = ResearchEngine::new();
        let result = engine.plan("search for AI safety research");
        assert!(result.intents.contains(&Intent::Search));
        assert!(!result.steps.is_empty());
        assert!(result.steps[0].tool_used.is_some());
    }

    #[test]
    fn test_research_plan_multi_intent() {
        let mut engine = ResearchEngine::new();
        let result = engine.plan("search for data and draw a chart");
        assert!(result.intents.contains(&Intent::Search));
        assert!(result.intents.contains(&Intent::Image));
        assert!(result.steps.len() >= 2);
    }

    #[test]
    fn test_research_stats() {
        let mut engine = ResearchEngine::new();
        engine.plan("test query 1");
        engine.plan("test query 2");
        let stats = engine.stats();
        assert_eq!(stats.total_queries, 2);
        assert!(stats.tool_count >= 4);
    }
}
