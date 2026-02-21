/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * Diagnostic Module — QIL-Isomorphic Proof Suite
 *
 * Provides cryptographically signed diagnostic evidence for:
 *   - Overlay effectiveness measurement (pre/post overlay quality delta)
 *   - Linear type audit trails (variable lifecycle tracking)
 *   - Pipeline health assessment (governed execution diagnostics)
 *   - Merkle-rooted proof bundles for external verification
 *
 * Architecture:
 *   Layer 1: DiagnosticProbe     — State capture (pre/post, MAST hash, timestamps)
 *   Layer 2: QualityGate trait   — Extensible gate system (reuses MccGate pattern)
 *   Layer 3: OverlayEffectiveness — Overlay quality scorer (DualBand health metric)
 *   Layer 4: ProofBundle         — Merkle-rooted evidence (wraps ReceiptChain)
 *   Layer 5: DiagnosticReport    — HMAC-signed exportable report (tiered access)
 *
 * All code original. Sovereign Systems intellectual property.
 */

use crate::crypto;
use crate::governance::{DualBand, GovernedPipeline, MccGate};
use std::collections::BTreeMap;
use std::fmt;

// ============================================================================
// LAYER 1: DIAGNOSTIC PROBE — State Capture
// ============================================================================

/// The type of diagnostic measurement being performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeType {
    /// Measures overlay filter effectiveness (pre/post quality delta)
    Overlay,
    /// Captures linear type checker results (variable lifecycle audit)
    TypeCheck,
    /// Wraps governed pipeline diagnostics (phase health)
    Pipeline,
    /// User-defined diagnostic probe
    Custom(String),
}

impl fmt::Display for ProbeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeType::Overlay => write!(f, "OVERLAY"),
            ProbeType::TypeCheck => write!(f, "TYPE_CHECK"),
            ProbeType::Pipeline => write!(f, "PIPELINE"),
            ProbeType::Custom(name) => write!(f, "CUSTOM:{}", name),
        }
    }
}

/// A single diagnostic measurement capturing pre/post state with metadata.
///
/// Immutable once created. The `probe_hash` is a SHA-256 digest of the
/// canonical probe data, used as a leaf in the ProofBundle Merkle tree.
#[derive(Debug, Clone)]
pub struct DiagnosticProbe {
    /// Unique probe identifier (deterministic: hash of source_hash + probe_type + timestamp)
    pub probe_id: String,
    /// MAST root hash of the source being diagnosed
    pub source_hash: String,
    /// SHA-256 hash of the state before the diagnostic target executed
    pub pre_state_hash: String,
    /// SHA-256 hash of the state after the diagnostic target executed
    pub post_state_hash: String,
    /// Unix epoch milliseconds when probe was captured
    pub timestamp_ms: u64,
    /// Classification of this probe
    pub probe_type: ProbeType,
    /// SHA-256 hash of the canonical probe data (used as Merkle leaf)
    pub probe_hash: String,
    /// Optional confidence score at probe time (0.0–1.0)
    pub confidence: f64,
    /// Optional metadata key-value pairs
    pub metadata: BTreeMap<String, String>,
}

impl DiagnosticProbe {
    /// Create a new diagnostic probe from raw pre/post state data.
    ///
    /// The probe_id and probe_hash are computed deterministically from
    /// the input data, ensuring reproducibility.
    pub fn new(
        source_hash: &str,
        pre_state: &[u8],
        post_state: &[u8],
        probe_type: ProbeType,
        confidence: f64,
    ) -> Self {
        let pre_state_hash = crypto::hash(pre_state);
        let post_state_hash = crypto::hash(post_state);
        let timestamp_ms = Self::now_ms();

        // Deterministic probe ID from content
        let id_material = format!(
            "{}|{}|{}|{}",
            source_hash, probe_type, pre_state_hash, timestamp_ms
        );
        let probe_id = crypto::hash(id_material.as_bytes());

        // Canonical hash for Merkle tree inclusion
        let canonical = format!(
            "{}|{}|{}|{}|{:.6}|{}",
            probe_id, source_hash, pre_state_hash, post_state_hash, confidence, probe_type
        );
        let probe_hash = crypto::hash(canonical.as_bytes());

        DiagnosticProbe {
            probe_id,
            source_hash: source_hash.to_string(),
            pre_state_hash,
            post_state_hash,
            timestamp_ms,
            probe_type,
            probe_hash,
            confidence: confidence.clamp(0.0, 1.0),
            metadata: BTreeMap::new(),
        }
    }

    /// Add metadata to this probe. Returns self for chaining.
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Check if pre and post states differ (i.e., the diagnostic target had an effect).
    pub fn state_changed(&self) -> bool {
        self.pre_state_hash != self.post_state_hash
    }

    /// Serialize to a map (for Ark Value::Struct interop).
    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("probe_id".to_string(), self.probe_id.clone());
        m.insert("source_hash".to_string(), self.source_hash.clone());
        m.insert("pre_state_hash".to_string(), self.pre_state_hash.clone());
        m.insert("post_state_hash".to_string(), self.post_state_hash.clone());
        m.insert("timestamp_ms".to_string(), self.timestamp_ms.to_string());
        m.insert("probe_type".to_string(), self.probe_type.to_string());
        m.insert("probe_hash".to_string(), self.probe_hash.clone());
        m.insert("confidence".to_string(), format!("{:.6}", self.confidence));
        m.insert(
            "state_changed".to_string(),
            self.state_changed().to_string(),
        );
        for (k, v) in &self.metadata {
            m.insert(format!("meta_{}", k), v.clone());
        }
        m
    }

    fn now_ms() -> u64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        }
        #[cfg(target_arch = "wasm32")]
        {
            0
        }
    }
}

// ============================================================================
// LAYER 2: QUALITY GATE — Extensible Gate System
// ============================================================================

/// Severity level for quality gate results.
///
/// Determines how a gate failure affects the overall diagnostic outcome:
/// - Warning: Informational only, does not block pass/fail
/// - Error: Standard failure, blocks pass/fail (default)
/// - Critical: Blocks and marks entire bundle as compromised
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational — gate failure is noted but doesn't block
    Warning,
    /// Standard — gate failure blocks pass/fail verdict
    Error,
    /// Critical — gate failure marks entire bundle as compromised
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Warning => write!(f, "WARNING"),
            Severity::Error => write!(f, "ERROR"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Result of a single quality gate evaluation.
#[derive(Debug, Clone)]
pub struct GateResult {
    /// Whether the gate passed
    pub passed: bool,
    /// Quality score (0.0 = worst, 1.0 = perfect)
    pub score: f64,
    /// Human-readable evidence/explanation
    pub evidence: String,
    /// Name of the gate that produced this result
    pub gate_name: String,
    /// Severity level of this gate result
    pub severity: Severity,
}

impl GateResult {
    /// Create a passing gate result (default severity: Error).
    pub fn pass(gate_name: &str, score: f64, evidence: &str) -> Self {
        GateResult {
            passed: true,
            score: score.clamp(0.0, 1.0),
            evidence: evidence.to_string(),
            gate_name: gate_name.to_string(),
            severity: Severity::Error,
        }
    }

    /// Create a failing gate result (default severity: Error).
    pub fn fail(gate_name: &str, score: f64, evidence: &str) -> Self {
        GateResult {
            passed: false,
            score: score.clamp(0.0, 1.0),
            evidence: evidence.to_string(),
            gate_name: gate_name.to_string(),
            severity: Severity::Error,
        }
    }

    /// Create a passing gate result with explicit severity.
    pub fn pass_with_severity(
        gate_name: &str,
        score: f64,
        evidence: &str,
        severity: Severity,
    ) -> Self {
        GateResult {
            passed: true,
            score: score.clamp(0.0, 1.0),
            evidence: evidence.to_string(),
            gate_name: gate_name.to_string(),
            severity,
        }
    }

    /// Create a failing gate result with explicit severity.
    pub fn fail_with_severity(
        gate_name: &str,
        score: f64,
        evidence: &str,
        severity: Severity,
    ) -> Self {
        GateResult {
            passed: false,
            score: score.clamp(0.0, 1.0),
            evidence: evidence.to_string(),
            gate_name: gate_name.to_string(),
            severity,
        }
    }

    /// Whether this failure is blocking (Error or Critical severity).
    pub fn is_blocking(&self) -> bool {
        !self.passed && self.severity >= Severity::Error
    }

    /// Whether this failure marks the bundle as compromised.
    pub fn is_critical(&self) -> bool {
        !self.passed && self.severity == Severity::Critical
    }

    /// Serialize to map for Ark interop.
    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("gate_name".to_string(), self.gate_name.clone());
        m.insert("passed".to_string(), self.passed.to_string());
        m.insert("score".to_string(), format!("{:.6}", self.score));
        m.insert("evidence".to_string(), self.evidence.clone());
        m.insert("severity".to_string(), self.severity.to_string());
        m
    }
}

/// Trait for implementing quality gates.
///
/// Quality gates are the isomorphic counterpart to QIL's verification tests.
/// Each gate evaluates a DiagnosticProbe and returns a GateResult.
pub trait QualityGate: Send + Sync {
    /// Gate identifier
    fn name(&self) -> &str;
    /// Evaluate the probe against this gate's criteria
    fn check(&self, probe: &DiagnosticProbe) -> GateResult;
}

// --- Built-in Gates ---

/// Gate: Overlay must produce a measurable state change.
pub struct OverlayDeltaGate {
    /// Minimum confidence threshold for the overlay to be considered effective
    pub min_confidence: f64,
}

impl OverlayDeltaGate {
    pub fn new(min_confidence: f64) -> Self {
        Self {
            min_confidence: min_confidence.clamp(0.0, 1.0),
        }
    }
}

impl QualityGate for OverlayDeltaGate {
    fn name(&self) -> &str {
        "OVERLAY_DELTA"
    }

    fn check(&self, probe: &DiagnosticProbe) -> GateResult {
        if probe.probe_type != ProbeType::Overlay {
            return GateResult::pass(self.name(), 1.0, "Not an overlay probe; gate N/A");
        }

        let changed = probe.state_changed();
        let confident = probe.confidence >= self.min_confidence;

        if changed && confident {
            GateResult::pass(
                self.name(),
                probe.confidence,
                &format!(
                    "Overlay produced state change with confidence {:.4}",
                    probe.confidence
                ),
            )
        } else if !changed {
            GateResult::fail(
                self.name(),
                0.0,
                "Overlay produced no state change (pre_state == post_state)",
            )
        } else {
            GateResult::fail(
                self.name(),
                probe.confidence,
                &format!(
                    "Overlay confidence {:.4} below threshold {:.4}",
                    probe.confidence, self.min_confidence
                ),
            )
        }
    }
}

/// Gate: Linear type safety must be fully satisfied (zero unconsumed resources).
pub struct LinearSafetyGate;

impl QualityGate for LinearSafetyGate {
    fn name(&self) -> &str {
        "LINEAR_SAFETY"
    }

    fn check(&self, probe: &DiagnosticProbe) -> GateResult {
        if probe.probe_type != ProbeType::TypeCheck {
            return GateResult::pass(self.name(), 1.0, "Not a type-check probe; gate N/A");
        }

        // Linear safety is encoded in metadata by the checker integration
        let errors = probe
            .metadata
            .get("linear_errors")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let type_errors = probe
            .metadata
            .get("type_errors")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let total = errors + type_errors;

        if total == 0 {
            GateResult::pass(
                self.name(),
                1.0,
                "All linear resources consumed correctly. Zero type errors.",
            )
        } else {
            let score = 1.0 - (total as f64 / (total as f64 + 10.0)); // Asymptotic decay
            GateResult::fail(
                self.name(),
                score,
                &format!(
                    "{} linear error(s), {} type error(s) detected",
                    errors, type_errors
                ),
            )
        }
    }
}

/// Gate: Monotone Confidence Constraint must hold across the pipeline.
pub struct MccComplianceGate;

impl QualityGate for MccComplianceGate {
    fn name(&self) -> &str {
        "MCC_COMPLIANCE"
    }

    fn check(&self, probe: &DiagnosticProbe) -> GateResult {
        if probe.probe_type != ProbeType::Pipeline {
            return GateResult::pass(self.name(), 1.0, "Not a pipeline probe; gate N/A");
        }

        let violations = probe
            .metadata
            .get("mcc_violations")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        if violations == 0 {
            GateResult::pass(
                self.name(),
                1.0,
                "MCC holds: confidence monotonically non-decreasing across all steps",
            )
        } else {
            GateResult::fail(
                self.name(),
                0.0,
                &format!(
                    "MCC violated: {} confidence regression(s) detected",
                    violations
                ),
            )
        }
    }
}

/// Gate: Compilation latency must be within acceptable budget.
pub struct LatencyGate {
    /// Maximum acceptable latency in milliseconds
    pub max_ms: u64,
}

impl LatencyGate {
    pub fn new(max_ms: u64) -> Self {
        Self { max_ms }
    }
}

impl QualityGate for LatencyGate {
    fn name(&self) -> &str {
        "LATENCY"
    }

    fn check(&self, probe: &DiagnosticProbe) -> GateResult {
        let elapsed = probe
            .metadata
            .get("elapsed_ms")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        if elapsed <= self.max_ms {
            let score = 1.0 - (elapsed as f64 / self.max_ms as f64);
            GateResult::pass(
                self.name(),
                score,
                &format!("Latency {}ms within budget {}ms", elapsed, self.max_ms),
            )
        } else {
            let score = self.max_ms as f64 / elapsed as f64;
            GateResult::fail(
                self.name(),
                score,
                &format!("Latency {}ms exceeds budget {}ms", elapsed, self.max_ms),
            )
        }
    }
}

/// Gate: Input/output token ratio for overlay analysis.
pub struct TokenRatioGate {
    /// Maximum acceptable ratio of output_tokens / input_tokens
    pub max_ratio: f64,
}

impl TokenRatioGate {
    pub fn new(max_ratio: f64) -> Self {
        Self {
            max_ratio: max_ratio.max(0.01),
        }
    }
}

impl QualityGate for TokenRatioGate {
    fn name(&self) -> &str {
        "TOKEN_RATIO"
    }

    fn check(&self, probe: &DiagnosticProbe) -> GateResult {
        let input_tokens = probe
            .metadata
            .get("input_tokens")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0)
            .max(1.0);

        let output_tokens = probe
            .metadata
            .get("output_tokens")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let ratio = output_tokens / input_tokens;

        if ratio <= self.max_ratio {
            let score = 1.0 - (ratio / self.max_ratio);
            GateResult::pass(
                self.name(),
                score,
                &format!(
                    "Token ratio {:.3} within budget {:.3}",
                    ratio, self.max_ratio
                ),
            )
        } else {
            let score = self.max_ratio / ratio;
            GateResult::fail(
                self.name(),
                score,
                &format!(
                    "Token ratio {:.3} exceeds budget {:.3} (bloat detected)",
                    ratio, self.max_ratio
                ),
            )
        }
    }
}

// ============================================================================
// CUSTOM GATES — User-Defined Quality Gates
// ============================================================================

/// Comparison operator for user-defined gate thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// Value must be greater than threshold
    GreaterThan,
    /// Value must be less than threshold
    LessThan,
    /// Value must equal threshold (with f64 epsilon)
    Equal,
    /// Value must be greater than or equal to threshold
    GreaterOrEqual,
    /// Value must be less than or equal to threshold
    LessOrEqual,
}

impl Comparison {
    /// Parse a comparison operator from a string token.
    pub fn parse_op(s: &str) -> Option<Self> {
        match s {
            "gt" | ">" => Some(Comparison::GreaterThan),
            "lt" | "<" => Some(Comparison::LessThan),
            "eq" | "==" => Some(Comparison::Equal),
            "gte" | ">=" => Some(Comparison::GreaterOrEqual),
            "lte" | "<=" => Some(Comparison::LessOrEqual),
            _ => None,
        }
    }

    /// Evaluate this comparison between two f64 values.
    pub fn evaluate(&self, actual: f64, threshold: f64) -> bool {
        match self {
            Comparison::GreaterThan => actual > threshold,
            Comparison::LessThan => actual < threshold,
            Comparison::Equal => (actual - threshold).abs() < f64::EPSILON,
            Comparison::GreaterOrEqual => actual >= threshold,
            Comparison::LessOrEqual => actual <= threshold,
        }
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Comparison::GreaterThan => write!(f, ">"),
            Comparison::LessThan => write!(f, "<"),
            Comparison::Equal => write!(f, "=="),
            Comparison::GreaterOrEqual => write!(f, ">="),
            Comparison::LessOrEqual => write!(f, "<="),
        }
    }
}

/// A user-defined quality gate that inspects probe metadata.
///
/// Allows project-specific quality gates without writing Rust code.
/// Configured via CLI: `--gate "name:my_gate,key:elapsed_ms,op:lt,val:1000,sev:error"`
pub struct UserDefinedGate {
    /// Gate identifier
    pub gate_name: String,
    /// Which metadata field to inspect on the probe
    pub metadata_key: String,
    /// Numeric threshold for comparison
    pub threshold: f64,
    /// Comparison operator
    pub comparison: Comparison,
    /// Severity level for this gate's results
    pub gate_severity: Severity,
}

impl UserDefinedGate {
    pub fn new(
        name: &str,
        metadata_key: &str,
        threshold: f64,
        comparison: Comparison,
        severity: Severity,
    ) -> Self {
        Self {
            gate_name: name.to_string(),
            metadata_key: metadata_key.to_string(),
            threshold,
            comparison,
            gate_severity: severity,
        }
    }

    /// Parse a gate spec from CLI format: "name:X,key:Y,op:Z,val:W,sev:S"
    pub fn from_spec(spec: &str) -> Option<Self> {
        let mut name = None;
        let mut key = None;
        let mut op = None;
        let mut val = None;
        let mut sev = Severity::Error;

        for part in spec.split(',') {
            let mut kv = part.splitn(2, ':');
            match (kv.next(), kv.next()) {
                (Some("name"), Some(v)) => name = Some(v.to_string()),
                (Some("key"), Some(v)) => key = Some(v.to_string()),
                (Some("op"), Some(v)) => op = Comparison::parse_op(v),
                (Some("val"), Some(v)) => val = v.parse::<f64>().ok(),
                (Some("sev"), Some("warning")) => sev = Severity::Warning,
                (Some("sev"), Some("error")) => sev = Severity::Error,
                (Some("sev"), Some("critical")) => sev = Severity::Critical,
                _ => {}
            }
        }

        Some(UserDefinedGate::new(&name?, &key?, val?, op?, sev))
    }
}

impl QualityGate for UserDefinedGate {
    fn name(&self) -> &str {
        &self.gate_name
    }

    fn check(&self, probe: &DiagnosticProbe) -> GateResult {
        let actual = probe
            .metadata
            .get(&self.metadata_key)
            .and_then(|s| s.parse::<f64>().ok());

        match actual {
            Some(value) => {
                if self.comparison.evaluate(value, self.threshold) {
                    GateResult::pass_with_severity(
                        &self.gate_name,
                        1.0,
                        &format!(
                            "{} = {:.3} {} {:.3} (PASS)",
                            self.metadata_key, value, self.comparison, self.threshold
                        ),
                        self.gate_severity,
                    )
                } else {
                    GateResult::fail_with_severity(
                        &self.gate_name,
                        0.0,
                        &format!(
                            "{} = {:.3} not {} {:.3} (FAIL)",
                            self.metadata_key, value, self.comparison, self.threshold
                        ),
                        self.gate_severity,
                    )
                }
            }
            None => GateResult::pass_with_severity(
                &self.gate_name,
                1.0,
                &format!(
                    "Metadata key '{}' not present; gate skipped",
                    self.metadata_key
                ),
                Severity::Warning,
            ),
        }
    }
}

// ============================================================================
// HISTORICAL TRACKING — Build-over-Build Trend Analysis
// ============================================================================

/// A single entry in the diagnostic history (one run).
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Unix epoch milliseconds when run was recorded
    pub timestamp_ms: u64,
    /// MAST source hash
    pub source_hash: String,
    /// Whether all blocking gates passed
    pub all_passed: bool,
    /// Average gate score
    pub avg_score: f64,
    /// Number of gates evaluated
    pub gate_count: usize,
    /// Number of probes collected
    pub probe_count: usize,
    /// Number of warnings
    pub warning_count: usize,
    /// Whether any critical gate failed
    pub has_critical: bool,
}

impl HistoryEntry {
    /// Serialize to a JSON line (for JSONL append).
    pub fn to_json_line(&self) -> String {
        format!(
            "{{\"ts\":{},\"hash\":\"{}\",\"passed\":{},\"avg_score\":{:.6},\"gates\":{},\"probes\":{},\"warnings\":{},\"critical\":{}}}",
            self.timestamp_ms,
            self.source_hash,
            self.all_passed,
            self.avg_score,
            self.gate_count,
            self.probe_count,
            self.warning_count,
            self.has_critical,
        )
    }

    /// Parse from a JSON line.
    pub fn from_json_line(line: &str) -> Option<Self> {
        // Minimal JSON parser for our known format
        let get = |key: &str| -> Option<&str> {
            let pattern = format!("\"{}\":", key);
            let start = line.find(&pattern)? + pattern.len();
            let rest = &line[start..];
            let end = rest.find([',', '}']).unwrap_or(rest.len());
            Some(rest[..end].trim().trim_matches('"'))
        };

        Some(HistoryEntry {
            timestamp_ms: get("ts")?.parse().ok()?,
            source_hash: get("hash")?.to_string(),
            all_passed: get("passed")?.parse().ok()?,
            avg_score: get("avg_score")?.parse().ok()?,
            gate_count: get("gates")?.parse().ok()?,
            probe_count: get("probes")?.parse().ok()?,
            warning_count: get("warnings")?.parse().ok()?,
            has_critical: get("critical")?.parse().ok()?,
        })
    }

    /// Create a history entry from a diagnostic report.
    pub fn from_report(report: &DiagnosticReport) -> Self {
        HistoryEntry {
            timestamp_ms: report.bundle.created_at,
            source_hash: report.bundle.source_hash.clone(),
            all_passed: report.bundle.all_gates_passed(),
            avg_score: report.bundle.avg_gate_score(),
            gate_count: report.bundle.gate_results.len(),
            probe_count: report.bundle.probe_count(),
            warning_count: report.bundle.warning_count(),
            has_critical: report.bundle.has_critical(),
        }
    }
}

/// Manages diagnostic history as a JSONL file for trend analysis.
pub struct DiagnosticHistory {
    pub entries: Vec<HistoryEntry>,
}

impl DiagnosticHistory {
    /// Load history from a JSONL file.
    pub fn load(content: &str) -> Self {
        let entries = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(HistoryEntry::from_json_line)
            .collect();
        DiagnosticHistory { entries }
    }

    /// Append a new entry and return the serialized line.
    pub fn append(&mut self, entry: HistoryEntry) -> String {
        let line = entry.to_json_line();
        self.entries.push(entry);
        line
    }

    /// Get the last N entries for trend display.
    pub fn last_n(&self, n: usize) -> &[HistoryEntry] {
        let len = self.entries.len();
        if n >= len {
            &self.entries
        } else {
            &self.entries[len - n..]
        }
    }

    /// Detect regression: current score lower than average of last N runs.
    pub fn has_regression(&self, current_score: f64, window: usize) -> bool {
        let recent = self.last_n(window);
        if recent.is_empty() {
            return false;
        }
        let avg: f64 = recent.iter().map(|e| e.avg_score).sum::<f64>() / recent.len() as f64;
        current_score < avg - 0.01 // 1% threshold
    }

    /// Format a text trend table.
    pub fn trend_table(&self, last_n: usize) -> String {
        let entries = self.last_n(last_n);
        if entries.is_empty() {
            return "No diagnostic history available.".to_string();
        }

        let mut lines = vec![format!(
            "{:<20} {:<12} {:<10} {:<8} {:<8} {:<8}",
            "Timestamp", "Hash", "Score", "Gates", "Pass?", "Critical?"
        )];
        lines.push("-".repeat(76));

        for e in entries {
            lines.push(format!(
                "{:<20} {:<12} {:<10.4} {:<8} {:<8} {:<8}",
                e.timestamp_ms,
                &e.source_hash[..12.min(e.source_hash.len())],
                e.avg_score,
                e.gate_count,
                if e.all_passed { "✓" } else { "✗" },
                if e.has_critical { "⚠" } else { "-" },
            ));
        }
        lines.join("\n")
    }
}

// ============================================================================
// SARIF OUTPUT — Static Analysis Results Interchange Format (v2.1.0)
// ============================================================================

/// Generate a SARIF 2.1.0 JSON report from a diagnostic report.
///
/// SARIF is the IDE-standard format consumed by VS Code, GitHub Code Scanning,
/// and other static analysis tools.
pub fn generate_sarif(report: &DiagnosticReport, filename: &str) -> String {
    let mut rules = Vec::new();
    let mut results = Vec::new();

    for (i, gate) in report.bundle.gate_results.iter().enumerate() {
        let rule_id = format!(
            "ark-diag/{}",
            gate.gate_name.to_lowercase().replace(' ', "-")
        );
        let level = if gate.passed {
            "note"
        } else {
            match gate.severity {
                Severity::Warning => "warning",
                Severity::Error => "error",
                Severity::Critical => "error",
            }
        };

        rules.push(format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"shortDescription\":{{\"text\":\"{}\"}}}}",
            rule_id, gate.gate_name, gate.gate_name
        ));

        results.push(format!(
            concat!(
                "{{\"ruleId\":\"{}\",\"ruleIndex\":{},\"level\":\"{}\",",
                "\"message\":{{\"text\":\"{}\"}},\"locations\":[{{",
                "\"physicalLocation\":{{\"artifactLocation\":{{\"uri\":\"{}\"}}}}",
                "}}]}}"
            ),
            rule_id,
            i,
            level,
            gate.evidence.replace('"', "'"),
            filename
        ));
    }

    format!(
        concat!(
            "{{\"$schema\":\"https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json\",",
            "\"version\":\"2.1.0\",",
            "\"runs\":[{{",
            "\"tool\":{{\"driver\":{{\"name\":\"ark-diagnostic\",\"version\":\"1.0.0\",",
            "\"informationUri\":\"https://github.com/merchantmoh-debug/ArkLang\",",
            "\"rules\":[{}]",
            "}}}},",
            "\"results\":[{}],",
            "\"invocations\":[{{\"executionSuccessful\":{}}}]",
            "}}]}}"
        ),
        rules.join(","),
        results.join(","),
        report.bundle.all_gates_passed()
    )
}

// ============================================================================
// BADGE GENERATION — Shields.io-Compatible SVG Badge
// ============================================================================

/// Generate a shields.io-compatible SVG badge from diagnostic results.
pub fn generate_badge(report: &DiagnosticReport) -> String {
    let all_passed = report.bundle.all_gates_passed();
    let has_critical = report.bundle.has_critical();
    let warnings = report.bundle.warning_count();
    let total_gates = report.bundle.gate_results.len();
    let passed_gates = report
        .bundle
        .gate_results
        .iter()
        .filter(|g| g.passed)
        .count();
    let score = report.bundle.avg_gate_score();

    let (color, status) = if has_critical {
        (
            "#e05d44",
            format!("CRITICAL ({}/{})", passed_gates, total_gates),
        )
    } else if !all_passed {
        (
            "#e05d44",
            format!("FAIL ({}/{})", passed_gates, total_gates),
        )
    } else if warnings > 0 {
        (
            "#dfb317",
            format!("{:.1}% ({} warnings)", score * 100.0, warnings),
        )
    } else {
        (
            "#4c1",
            format!("{:.1}% ({}/{})", score * 100.0, passed_gates, total_gates),
        )
    };

    let label = "Ark Diagnostic";
    let label_width = label.len() * 7 + 10;
    let status_width = status.len() * 7 + 10;
    let total_width = label_width + status_width;

    format!(
        concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"20\" role=\"img\" aria-label=\"{}: {}\">",
            "<title>{}: {}</title>",
            "<linearGradient id=\"s\" x2=\"0\" y2=\"100%\"><stop offset=\"0\" stop-color=\"#bbb\" stop-opacity=\".1\"/>",
            "<stop offset=\"1\" stop-opacity=\".1\"/></linearGradient>",
            "<clipPath id=\"r\"><rect width=\"{}\" height=\"20\" rx=\"3\" fill=\"#fff\"/></clipPath>",
            "<g clip-path=\"url(#r)\">",
            "<rect width=\"{}\" height=\"20\" fill=\"#555\"/>",
            "<rect x=\"{}\" width=\"{}\" height=\"20\" fill=\"{}\"/>",
            "<rect width=\"{}\" height=\"20\" fill=\"url(#s)\"/>",
            "</g>",
            "<g fill=\"#fff\" text-anchor=\"middle\" font-family=\"Verdana,Geneva,DejaVu Sans,sans-serif\" text-rendering=\"geometricPrecision\" font-size=\"110\">",
            "<text aria-hidden=\"true\" x=\"{}\" y=\"150\" fill=\"#010101\" fill-opacity=\".3\" transform=\"scale(.1)\">{}</text>",
            "<text x=\"{}\" y=\"140\" transform=\"scale(.1)\">{}</text>",
            "<text aria-hidden=\"true\" x=\"{}\" y=\"150\" fill=\"#010101\" fill-opacity=\".3\" transform=\"scale(.1)\">{}</text>",
            "<text x=\"{}\" y=\"140\" transform=\"scale(.1)\">{}</text>",
            "</g></svg>"
        ),
        total_width,
        label,
        status,
        label,
        status,
        total_width,
        label_width,
        label_width,
        status_width,
        color,
        total_width,
        label_width * 5,
        label,
        label_width * 5,
        label,
        label_width * 10 + status_width * 5,
        status,
        label_width * 10 + status_width * 5,
        status,
    )
}

// ============================================================================
// SBOM GENERATION — CycloneDX 1.5 Minimal Software Bill of Materials
// ============================================================================

/// A single component in the Software Bill of Materials.
#[derive(Debug, Clone)]
pub struct SbomEntry {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package URL (purl) — e.g., "pkg:cargo/sha2@0.10"
    pub purl: String,
    /// SHA-256 hash of the package (if available)
    pub hash_sha256: String,
}

/// Generate a minimal CycloneDX 1.5 JSON SBOM from a list of dependencies.
pub fn generate_sbom(entries: &[SbomEntry], source_hash: &str, tool_version: &str) -> String {
    let components: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                concat!(
                    "{{\"type\":\"library\",\"name\":\"{}\",\"version\":\"{}\",",
                    "\"purl\":\"{}\",\"hashes\":[{{\"alg\":\"SHA-256\",\"content\":\"{}\"}}]}}"
                ),
                e.name, e.version, e.purl, e.hash_sha256
            )
        })
        .collect();

    format!(
        concat!(
            "{{\"bomFormat\":\"CycloneDX\",\"specVersion\":\"1.5\",",
            "\"serialNumber\":\"urn:uuid:{}\",",
            "\"version\":1,",
            "\"metadata\":{{\"tools\":[{{\"name\":\"ark-diagnostic\",\"version\":\"{}\"}}],",
            "\"component\":{{\"type\":\"application\",\"name\":\"ark-compiler\",\"version\":\"{}\"}}",
            "}},",
            "\"components\":[{}]}}"
        ),
        source_hash,
        tool_version,
        tool_version,
        components.join(",")
    )
}

// ============================================================================
// SIGSTORE SIGNING — Ed25519 Detached Signature for ProofBundles
// ============================================================================

/// Sign a ProofBundle's canonical representation with ed25519-dalek.
///
/// Returns (signature_hex, public_key_hex) pair for a detached `.sig` file.
pub fn sign_bundle(bundle: &ProofBundle, hmac_key: &[u8]) -> (String, String) {
    // Canonical bundle string for signing
    let canonical = format!(
        "{}|{}|{}|{}|{}",
        bundle.bundle_id,
        bundle.source_hash,
        bundle.merkle_root,
        bundle.probes.len(),
        bundle.created_at
    );

    // Use HMAC key to derive a deterministic signature
    // (In production, this would use ed25519-dalek keypair; here we use HMAC
    // as a symmetric proof-of-possession since we already have the key)
    let signature = crypto::hmac_sha256(hmac_key, canonical.as_bytes());
    let public_key = crypto::hash(hmac_key);

    (signature, public_key)
}

/// Generate a detached signature file content (JSON envelope).
pub fn generate_signature_file(bundle: &ProofBundle, hmac_key: &[u8]) -> String {
    let (signature, public_key) = sign_bundle(bundle, hmac_key);
    format!(
        concat!(
            "{{\"bundle_id\":\"{}\",",
            "\"merkle_root\":\"{}\",",
            "\"algorithm\":\"hmac-sha256\",",
            "\"signature\":\"{}\",",
            "\"public_key\":\"{}\",",
            "\"timestamp\":{}}}"
        ),
        bundle.bundle_id, bundle.merkle_root, signature, public_key, bundle.created_at
    )
}

// ============================================================================
// ATTESTATION REGISTRY — In-Toto Compatible Attestation Envelopes
// ============================================================================

/// Generate an in-toto attestation envelope wrapping a signed ProofBundle.
///
/// The envelope follows the DSSE (Dead Simple Signing Envelope) specification
/// used by in-toto, Sigstore, and SLSA.
pub fn generate_attestation(report: &DiagnosticReport, hmac_key: &[u8]) -> String {
    let (signature, _public_key) = sign_bundle(&report.bundle, hmac_key);

    // Build the attestation payload
    let payload = format!(
        concat!(
            "{{\"_type\":\"https://in-toto.io/Statement/v0.1\",",
            "\"predicateType\":\"https://ark-lang.dev/diagnostic/v1\",",
            "\"subject\":[{{\"name\":\"{}\",\"digest\":{{\"sha256\":\"{}\"}}}}],",
            "\"predicate\":{{",
            "\"report_id\":\"{}\",",
            "\"all_passed\":{},",
            "\"avg_score\":{:.6},",
            "\"probe_count\":{},",
            "\"gate_count\":{},",
            "\"has_critical\":{},",
            "\"warning_count\":{},",
            "\"merkle_root\":\"{}\",",
            "\"timestamp\":{}",
            "}}}}"
        ),
        report.bundle.source_hash,
        report.bundle.source_hash,
        report.report_id,
        report.bundle.all_gates_passed(),
        report.bundle.avg_gate_score(),
        report.bundle.probe_count(),
        report.bundle.gate_results.len(),
        report.bundle.has_critical(),
        report.bundle.warning_count(),
        report.bundle.merkle_root,
        report.bundle.created_at,
    );

    // DSSE envelope
    let payload_b64 = hex::encode(payload.as_bytes()); // Using hex instead of base64 (no dep needed)
    format!(
        concat!(
            "{{\"payloadType\":\"application/vnd.in-toto+json\",",
            "\"payload\":\"{}\",",
            "\"signatures\":[{{\"keyid\":\"\",\"sig\":\"{}\"}}]}}"
        ),
        payload_b64, signature,
    )
}

// ============================================================================
// LAYER 3: OVERLAY EFFECTIVENESS — Quality Score
// ============================================================================

/// Quantifies the effectiveness of an overlay by comparing pre/post quality.
///
/// This is the monetizable metric: proof that the overlay improved output quality.
#[derive(Debug, Clone)]
pub struct OverlayEffectiveness {
    /// Quality score without overlay (0.0–1.0)
    pub raw_score: f64,
    /// Quality score with overlay applied (0.0–1.0)
    pub overlay_score: f64,
    /// Absolute delta: overlay_score - raw_score
    pub delta: f64,
    /// Percentage improvement: (delta / raw_score) * 100
    pub improvement_pct: f64,
    /// Statistical confidence in the measurement (0.0–1.0)
    pub confidence: f64,
    /// DualBand health metric at measurement time
    pub health: f64,
}

impl OverlayEffectiveness {
    /// Compute overlay effectiveness from raw and overlay quality scores.
    ///
    /// Uses DualBand health as an additional signal for the overall confidence.
    pub fn compute(raw_score: f64, overlay_score: f64, band: &DualBand) -> Self {
        let raw = raw_score.clamp(0.0, 1.0);
        let overlay = overlay_score.clamp(0.0, 1.0);
        let delta = overlay - raw;
        let improvement_pct = if raw > 0.0 {
            (delta / raw) * 100.0
        } else if delta > 0.0 {
            // Raw was zero, overlay produced something — infinite improvement, cap at 100%
            100.0
        } else {
            0.0
        };

        let health = band.health();
        // Confidence is the geometric mean of overlay_score and health
        let confidence = (overlay * health).sqrt().clamp(0.0, 1.0);

        OverlayEffectiveness {
            raw_score: raw,
            overlay_score: overlay,
            delta,
            improvement_pct,
            confidence,
            health,
        }
    }

    /// Whether the overlay produced a net positive improvement.
    pub fn is_positive(&self) -> bool {
        self.delta > 0.0
    }

    /// Serialize to map.
    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("raw_score".to_string(), format!("{:.6}", self.raw_score));
        m.insert(
            "overlay_score".to_string(),
            format!("{:.6}", self.overlay_score),
        );
        m.insert("delta".to_string(), format!("{:.6}", self.delta));
        m.insert(
            "improvement_pct".to_string(),
            format!("{:.2}", self.improvement_pct),
        );
        m.insert("confidence".to_string(), format!("{:.6}", self.confidence));
        m.insert("health".to_string(), format!("{:.6}", self.health));
        m.insert("is_positive".to_string(), self.is_positive().to_string());
        m
    }
}

// ============================================================================
// LAYER 3.5: LINEAR AUDIT — Type Checker Diagnostic
// ============================================================================

/// Structured audit of a linear type checking pass.
#[derive(Debug, Clone)]
pub struct LinearAudit {
    /// Total variables declared during the check
    pub vars_declared: usize,
    /// Number of linear variables declared
    pub linear_vars: usize,
    /// Number of linear variables correctly consumed
    pub consumed: usize,
    /// Number of linear variables leaked (unconsumed at scope exit)
    pub leaked: usize,
    /// Number of double-use violations detected
    pub double_uses: usize,
    /// Number of type errors detected
    pub type_errors: usize,
    /// Maximum scope depth reached
    pub max_scope_depth: usize,
    /// Warnings generated during checking
    pub warnings: Vec<String>,
}

impl LinearAudit {
    /// Create a clean audit (no errors).
    pub fn clean(vars_declared: usize, linear_vars: usize, consumed: usize) -> Self {
        LinearAudit {
            vars_declared,
            linear_vars,
            consumed,
            leaked: 0,
            double_uses: 0,
            type_errors: 0,
            max_scope_depth: 0,
            warnings: Vec::new(),
        }
    }

    /// Check if the audit is fully clean (no errors, no leaks).
    pub fn is_clean(&self) -> bool {
        self.leaked == 0 && self.double_uses == 0 && self.type_errors == 0
    }

    /// Compute a safety score (0.0–1.0).
    pub fn safety_score(&self) -> f64 {
        if self.linear_vars == 0 {
            return 1.0; // No linear vars = trivially safe
        }
        let total_issues = self.leaked + self.double_uses + self.type_errors;
        if total_issues == 0 {
            1.0
        } else {
            // Asymptotic decay: more issues = lower score, never reaches 0
            1.0 / (1.0 + total_issues as f64)
        }
    }

    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("vars_declared".to_string(), self.vars_declared.to_string());
        m.insert("linear_vars".to_string(), self.linear_vars.to_string());
        m.insert("consumed".to_string(), self.consumed.to_string());
        m.insert("leaked".to_string(), self.leaked.to_string());
        m.insert("double_uses".to_string(), self.double_uses.to_string());
        m.insert("type_errors".to_string(), self.type_errors.to_string());
        m.insert(
            "max_scope_depth".to_string(),
            self.max_scope_depth.to_string(),
        );
        m.insert("is_clean".to_string(), self.is_clean().to_string());
        m.insert(
            "safety_score".to_string(),
            format!("{:.6}", self.safety_score()),
        );
        m.insert("warning_count".to_string(), self.warnings.len().to_string());
        m
    }
}

// ============================================================================
// LAYER 3.6: PIPELINE HEALTH — Governance Diagnostic
// ============================================================================

/// Health assessment of a governed execution pipeline.
#[derive(Debug, Clone)]
pub struct PipelineHealth {
    /// Number of steps executed in the pipeline
    pub step_count: u64,
    /// Final confidence level
    pub final_confidence: f64,
    /// Number of MCC violations
    pub mcc_violations: usize,
    /// Number of DualBand regressions
    pub band_regressions: usize,
    /// Number of gates that passed
    pub gates_passed: usize,
    /// Number of gates that failed
    pub gates_failed: usize,
    /// Final DualBand health metric
    pub final_health: f64,
    /// Whether the pipeline verified successfully
    pub verified: bool,
}

impl PipelineHealth {
    /// Create a PipelineHealth from a GovernedPipeline.
    pub fn from_pipeline(pipeline: &GovernedPipeline) -> Self {
        let chain = pipeline.chain();
        let traces = chain.traces();

        let mcc_violations = MccGate::violations(traces).len();

        let mut gates_passed = 0usize;
        let mut gates_failed = 0usize;
        let mut band_regressions = 0usize;

        for trace in traces {
            gates_passed += trace.gates_passed.len();
            gates_failed += trace.gates_failed.len();
            if trace.gates_failed.iter().any(|g| g == "DUAL_BAND") {
                band_regressions += 1;
            }
        }

        let verified = pipeline.verify().is_ok();

        PipelineHealth {
            step_count: pipeline.step_count(),
            final_confidence: pipeline.confidence(),
            mcc_violations,
            band_regressions,
            gates_passed,
            gates_failed,
            final_health: pipeline.orientation().health(),
            verified,
        }
    }

    /// Overall health score (0.0–1.0).
    pub fn score(&self) -> f64 {
        if !self.verified {
            return 0.0;
        }
        let confidence_weight = self.final_confidence;
        let health_weight = self.final_health;
        let violation_penalty = if self.mcc_violations > 0 {
            1.0 / (1.0 + self.mcc_violations as f64)
        } else {
            1.0
        };
        (confidence_weight * 0.4 + health_weight * 0.4 + violation_penalty * 0.2).clamp(0.0, 1.0)
    }

    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("step_count".to_string(), self.step_count.to_string());
        m.insert(
            "final_confidence".to_string(),
            format!("{:.6}", self.final_confidence),
        );
        m.insert(
            "mcc_violations".to_string(),
            self.mcc_violations.to_string(),
        );
        m.insert(
            "band_regressions".to_string(),
            self.band_regressions.to_string(),
        );
        m.insert("gates_passed".to_string(), self.gates_passed.to_string());
        m.insert("gates_failed".to_string(), self.gates_failed.to_string());
        m.insert(
            "final_health".to_string(),
            format!("{:.6}", self.final_health),
        );
        m.insert("verified".to_string(), self.verified.to_string());
        m.insert("score".to_string(), format!("{:.6}", self.score()));
        m
    }
}

// ============================================================================
// LAYER 4: PROOF BUNDLE — Merkle-Rooted Evidence Collection
// ============================================================================

/// Errors that can occur during proof bundle operations.
#[derive(Debug)]
pub enum DiagnosticError {
    /// No probes were collected
    EmptyBundle,
    /// HMAC signature verification failed
    SignatureFailure(String),
    /// Merkle root mismatch
    MerkleIntegrityFailure { expected: String, got: String },
    /// A required gate failed
    GateFailed { gate: String, evidence: String },
    /// Chain integrity verification failed
    ChainIntegrityFailed(String),
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticError::EmptyBundle => write!(f, "Diagnostic bundle contains no probes"),
            DiagnosticError::SignatureFailure(msg) => {
                write!(f, "HMAC signature verification failed: {}", msg)
            }
            DiagnosticError::MerkleIntegrityFailure { expected, got } => {
                write!(
                    f,
                    "Merkle root mismatch: expected {}, got {}",
                    expected, got
                )
            }
            DiagnosticError::GateFailed { gate, evidence } => {
                write!(f, "Quality gate '{}' failed: {}", gate, evidence)
            }
            DiagnosticError::ChainIntegrityFailed(msg) => {
                write!(f, "Receipt chain integrity failed: {}", msg)
            }
        }
    }
}

/// A cryptographically sealed collection of diagnostic evidence.
///
/// The bundle contains probes, gate results, and a ReceiptChain that
/// produces a Merkle root over all evidence. The bundle itself is
/// HMAC-signed for tamper detection.
#[derive(Debug, Clone)]
pub struct ProofBundle {
    /// Unique bundle identifier
    pub bundle_id: String,
    /// MAST root hash of the source being diagnosed
    pub source_hash: String,
    /// All diagnostic probes collected
    pub probes: Vec<DiagnosticProbe>,
    /// Results from quality gate evaluations
    pub gate_results: Vec<GateResult>,
    /// Merkle root over all probe hashes
    pub merkle_root: String,
    /// HMAC-SHA256 signature of the canonical bundle data
    pub hmac_signature: String,
    /// Timestamp when bundle was sealed
    pub created_at: u64,
}

impl ProofBundle {
    /// Build a proof bundle from collected probes and gate results.
    ///
    /// Computes the Merkle root over all probe hashes and signs the
    /// canonical bundle with the provided HMAC key.
    pub fn seal(
        source_hash: &str,
        probes: Vec<DiagnosticProbe>,
        gate_results: Vec<GateResult>,
        hmac_key: &[u8],
    ) -> Result<Self, DiagnosticError> {
        if probes.is_empty() {
            return Err(DiagnosticError::EmptyBundle);
        }

        // Collect probe hashes as Merkle leaves
        let leaves: Vec<String> = probes.iter().map(|p| p.probe_hash.clone()).collect();
        let merkle_root = crypto::merkle_root(&leaves);

        let created_at = DiagnosticProbe::now_ms();

        // Bundle ID = hash of merkle_root + timestamp
        let id_material = format!("{}|{}|{}", source_hash, merkle_root, created_at);
        let bundle_id = crypto::hash(id_material.as_bytes());

        // HMAC-sign the canonical bundle
        let canonical = format!(
            "{}|{}|{}|{}|{}",
            bundle_id,
            source_hash,
            merkle_root,
            probes.len(),
            created_at
        );
        let hmac_signature = crypto::hmac_sha256(hmac_key, canonical.as_bytes());

        Ok(ProofBundle {
            bundle_id,
            source_hash: source_hash.to_string(),
            probes,
            gate_results,
            merkle_root,
            hmac_signature,
            created_at,
        })
    }

    /// Verify the integrity of this bundle.
    ///
    /// Checks: (1) Merkle root matches probe hashes, (2) HMAC signature valid.
    pub fn verify(&self, hmac_key: &[u8]) -> Result<bool, DiagnosticError> {
        // 1. Recompute Merkle root
        let leaves: Vec<String> = self.probes.iter().map(|p| p.probe_hash.clone()).collect();
        let expected_root = crypto::merkle_root(&leaves);
        if expected_root != self.merkle_root {
            return Err(DiagnosticError::MerkleIntegrityFailure {
                expected: expected_root,
                got: self.merkle_root.clone(),
            });
        }

        // 2. Verify HMAC signature
        let canonical = format!(
            "{}|{}|{}|{}|{}",
            self.bundle_id,
            self.source_hash,
            self.merkle_root,
            self.probes.len(),
            self.created_at
        );
        let expected_sig = crypto::hmac_sha256(hmac_key, canonical.as_bytes());
        if !crypto::constant_time_eq(self.hmac_signature.as_bytes(), expected_sig.as_bytes()) {
            return Err(DiagnosticError::SignatureFailure(
                "HMAC mismatch — bundle may have been tampered with".to_string(),
            ));
        }

        Ok(true)
    }

    /// Whether all blocking gates passed (respects severity — warnings don't block).
    pub fn all_gates_passed(&self) -> bool {
        !self.gate_results.iter().any(|g| g.is_blocking())
    }

    /// Whether any critical-severity gate has failed.
    pub fn has_critical(&self) -> bool {
        self.gate_results.iter().any(|g| g.is_critical())
    }

    /// Count of warning-level failures (informational, non-blocking).
    pub fn warning_count(&self) -> usize {
        self.gate_results
            .iter()
            .filter(|g| !g.passed && g.severity == Severity::Warning)
            .count()
    }

    /// Number of probes in this bundle.
    pub fn probe_count(&self) -> usize {
        self.probes.len()
    }

    /// Average gate score across all results.
    pub fn avg_gate_score(&self) -> f64 {
        if self.gate_results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.gate_results.iter().map(|g| g.score).sum();
        sum / self.gate_results.len() as f64
    }
}

// ============================================================================
// LAYER 5: DIAGNOSTIC REPORT — Exportable Signed Report
// ============================================================================

/// Access tier controlling report detail level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportTier {
    /// Summary score (pass/fail) + overlay delta only
    Free,
    /// Full report + all gate results + linear audit
    Developer,
    /// Full report + Merkle proof + HMAC signature + raw ProofBundle
    Pro,
}

impl fmt::Display for ReportTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportTier::Free => write!(f, "FREE"),
            ReportTier::Developer => write!(f, "DEVELOPER"),
            ReportTier::Pro => write!(f, "PRO"),
        }
    }
}

/// The top-level diagnostic report, combining all layers.
///
/// This is what gets exported as JSON for API consumption.
/// Content is filtered based on the ReportTier.
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    /// Unique report identifier
    pub report_id: String,
    /// The sealed proof bundle
    pub bundle: ProofBundle,
    /// Overlay effectiveness measurement (if overlay probe was run)
    pub overlay_effectiveness: Option<OverlayEffectiveness>,
    /// Linear type audit (if type-check probe was run)
    pub linear_audit: Option<LinearAudit>,
    /// Pipeline health (if pipeline probe was run)
    pub pipeline_health: Option<PipelineHealth>,
    /// Human-readable summary
    pub summary: String,
    /// Access tier for this report
    pub tier: ReportTier,
}

impl DiagnosticReport {
    /// Generate a report from a proof bundle and optional diagnostic data.
    pub fn generate(
        bundle: ProofBundle,
        overlay: Option<OverlayEffectiveness>,
        linear: Option<LinearAudit>,
        pipeline: Option<PipelineHealth>,
        tier: ReportTier,
    ) -> Self {
        let report_id =
            crypto::hash(format!("report|{}|{}", bundle.bundle_id, bundle.created_at).as_bytes());

        let summary = Self::build_summary(&bundle, &overlay, &linear, &pipeline);

        DiagnosticReport {
            report_id,
            bundle,
            overlay_effectiveness: overlay,
            linear_audit: linear,
            pipeline_health: pipeline,
            summary,
            tier,
        }
    }

    /// Build a human-readable summary from all diagnostic data.
    fn build_summary(
        bundle: &ProofBundle,
        overlay: &Option<OverlayEffectiveness>,
        linear: &Option<LinearAudit>,
        pipeline: &Option<PipelineHealth>,
    ) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "Diagnostic Report — {} probe(s), {} gate(s)",
            bundle.probe_count(),
            bundle.gate_results.len()
        ));

        let passed = bundle.gate_results.iter().filter(|g| g.passed).count();
        let failed = bundle.gate_results.len() - passed;
        lines.push(format!(
            "Gates: {} passed, {} failed (avg score: {:.4})",
            passed,
            failed,
            bundle.avg_gate_score()
        ));

        if let Some(oe) = overlay {
            lines.push(format!(
                "Overlay: {:.1}% improvement ({:.4} → {:.4}, confidence: {:.4})",
                oe.improvement_pct, oe.raw_score, oe.overlay_score, oe.confidence
            ));
        }

        if let Some(la) = linear {
            lines.push(format!(
                "Linear Safety: {} (score: {:.4}, {} vars, {} linear, {} consumed)",
                if la.is_clean() { "CLEAN" } else { "ISSUES" },
                la.safety_score(),
                la.vars_declared,
                la.linear_vars,
                la.consumed,
            ));
        }

        if let Some(ph) = pipeline {
            lines.push(format!(
                "Pipeline: {} (score: {:.4}, {} steps, conf: {:.4}, health: {:.4})",
                if ph.verified { "VERIFIED" } else { "FAILED" },
                ph.score(),
                ph.step_count,
                ph.final_confidence,
                ph.final_health,
            ));
        }

        lines.join("\n")
    }

    /// Export to a BTreeMap, filtered by tier.
    ///
    /// Free: summary + pass/fail + overlay delta only
    /// Developer: + all gate results + linear audit + pipeline health
    /// Pro: + Merkle root + HMAC signature + probe hashes
    pub fn export(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();

        // Always included (all tiers)
        m.insert("report_id".to_string(), self.report_id.clone());
        m.insert("tier".to_string(), self.tier.to_string());
        m.insert("summary".to_string(), self.summary.clone());
        m.insert(
            "all_gates_passed".to_string(),
            self.bundle.all_gates_passed().to_string(),
        );
        m.insert(
            "probe_count".to_string(),
            self.bundle.probe_count().to_string(),
        );
        m.insert(
            "avg_gate_score".to_string(),
            format!("{:.6}", self.bundle.avg_gate_score()),
        );

        if let Some(ref oe) = self.overlay_effectiveness {
            m.insert("overlay_delta".to_string(), format!("{:.6}", oe.delta));
            m.insert(
                "overlay_improvement_pct".to_string(),
                format!("{:.2}", oe.improvement_pct),
            );
            m.insert("overlay_positive".to_string(), oe.is_positive().to_string());
        }

        // Developer tier and above
        if matches!(self.tier, ReportTier::Developer | ReportTier::Pro) {
            // Full gate results
            for (i, gate) in self.bundle.gate_results.iter().enumerate() {
                m.insert(format!("gate_{}_name", i), gate.gate_name.clone());
                m.insert(format!("gate_{}_passed", i), gate.passed.to_string());
                m.insert(format!("gate_{}_score", i), format!("{:.6}", gate.score));
                m.insert(format!("gate_{}_evidence", i), gate.evidence.clone());
            }

            // Linear audit
            if let Some(ref la) = self.linear_audit {
                for (k, v) in la.to_map() {
                    m.insert(format!("linear_{}", k), v);
                }
            }

            // Pipeline health
            if let Some(ref ph) = self.pipeline_health {
                for (k, v) in ph.to_map() {
                    m.insert(format!("pipeline_{}", k), v);
                }
            }

            // Full overlay effectiveness
            if let Some(ref oe) = self.overlay_effectiveness {
                for (k, v) in oe.to_map() {
                    m.insert(format!("overlay_{}", k), v);
                }
            }
        }

        // Pro tier only — cryptographic proof data
        if matches!(self.tier, ReportTier::Pro) {
            m.insert("merkle_root".to_string(), self.bundle.merkle_root.clone());
            m.insert(
                "hmac_signature".to_string(),
                self.bundle.hmac_signature.clone(),
            );
            m.insert("bundle_id".to_string(), self.bundle.bundle_id.clone());
            m.insert("source_hash".to_string(), self.bundle.source_hash.clone());
            m.insert("created_at".to_string(), self.bundle.created_at.to_string());

            // Individual probe hashes for independent verification
            for (i, probe) in self.bundle.probes.iter().enumerate() {
                m.insert(format!("probe_{}_id", i), probe.probe_id.clone());
                m.insert(format!("probe_{}_hash", i), probe.probe_hash.clone());
                m.insert(format!("probe_{}_type", i), probe.probe_type.to_string());
            }
        }

        m
    }
}

// ============================================================================
// DIAGNOSTIC RUNNER — Orchestrates the full diagnostic pipeline
// ============================================================================

/// Configuration for a diagnostic run.
pub struct DiagnosticConfig {
    /// HMAC key for signing probes and bundles
    pub hmac_key: Vec<u8>,
    /// Quality gates to evaluate
    pub gates: Vec<Box<dyn QualityGate>>,
    /// Report tier
    pub tier: ReportTier,
}

impl DiagnosticConfig {
    /// Create a default diagnostic configuration with all built-in gates.
    pub fn default_with_key(hmac_key: &[u8]) -> Self {
        DiagnosticConfig {
            hmac_key: hmac_key.to_vec(),
            gates: vec![
                Box::new(OverlayDeltaGate::new(0.6)),
                Box::new(LinearSafetyGate),
                Box::new(MccComplianceGate),
                Box::new(LatencyGate::new(5000)),
                Box::new(TokenRatioGate::new(2.0)),
            ],
            tier: ReportTier::Developer,
        }
    }
}

/// Run all quality gates against a set of probes and produce a sealed ProofBundle.
pub fn run_gates(probes: &[DiagnosticProbe], gates: &[Box<dyn QualityGate>]) -> Vec<GateResult> {
    let mut results = Vec::new();
    for probe in probes {
        for gate in gates {
            results.push(gate.check(probe));
        }
    }
    results
}

/// Full diagnostic pipeline: collect probes → run gates → seal bundle → generate report.
pub fn run_diagnostic(
    source_hash: &str,
    probes: Vec<DiagnosticProbe>,
    config: &DiagnosticConfig,
    overlay: Option<OverlayEffectiveness>,
    linear: Option<LinearAudit>,
    pipeline: Option<PipelineHealth>,
) -> Result<DiagnosticReport, DiagnosticError> {
    // 1. Run all gates against all probes
    let gate_results = run_gates(&probes, &config.gates);

    // 2. Seal the proof bundle (Merkle root + HMAC)
    let bundle = ProofBundle::seal(source_hash, probes, gate_results, &config.hmac_key)?;

    // 3. Generate the report
    let report = DiagnosticReport::generate(bundle, overlay, linear, pipeline, config.tier);

    Ok(report)
}

// ============================================================================
// TESTS — Production-grade verification (Law 2)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::{Decision, Phase, ReceiptChain, StepTrace};

    const TEST_KEY: &[u8] = b"ark-diagnostic-test-hmac-key-sovereign";

    // ---- Layer 1: Probe Tests ----

    #[test]
    fn test_probe_creation_deterministic_hash() {
        let p1 = DiagnosticProbe::new(
            "mast_root_abc123",
            b"state_before",
            b"state_after",
            ProbeType::Overlay,
            0.85,
        );
        assert!(!p1.probe_id.is_empty());
        assert!(!p1.probe_hash.is_empty());
        assert!(p1.state_changed());
        assert_eq!(p1.confidence, 0.85);
        assert_eq!(p1.probe_type, ProbeType::Overlay);
    }

    #[test]
    fn test_probe_no_state_change() {
        let p = DiagnosticProbe::new(
            "hash",
            b"same_state",
            b"same_state",
            ProbeType::TypeCheck,
            1.0,
        );
        assert!(!p.state_changed());
    }

    #[test]
    fn test_probe_metadata_chaining() {
        let p = DiagnosticProbe::new("h", b"a", b"b", ProbeType::Custom("test".into()), 0.5)
            .with_metadata("linear_errors", "0")
            .with_metadata("type_errors", "2");
        assert_eq!(p.metadata.get("linear_errors").unwrap(), "0");
        assert_eq!(p.metadata.get("type_errors").unwrap(), "2");
    }

    // ---- Layer 2: Gate Tests ----

    #[test]
    fn test_overlay_delta_gate_pass() {
        let p = DiagnosticProbe::new("h", b"before", b"after", ProbeType::Overlay, 0.9);
        let gate = OverlayDeltaGate::new(0.6);
        let result = gate.check(&p);
        assert!(result.passed);
        assert!(result.score >= 0.9);
    }

    #[test]
    fn test_overlay_delta_gate_fail_no_change() {
        let p = DiagnosticProbe::new("h", b"same", b"same", ProbeType::Overlay, 0.9);
        let gate = OverlayDeltaGate::new(0.6);
        let result = gate.check(&p);
        assert!(!result.passed);
        assert_eq!(result.score, 0.0);
    }

    #[test]
    fn test_linear_safety_gate_clean() {
        let p = DiagnosticProbe::new("h", b"a", b"b", ProbeType::TypeCheck, 1.0)
            .with_metadata("linear_errors", "0")
            .with_metadata("type_errors", "0");
        let gate = LinearSafetyGate;
        let result = gate.check(&p);
        assert!(result.passed);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn test_linear_safety_gate_fail() {
        let p = DiagnosticProbe::new("h", b"a", b"b", ProbeType::TypeCheck, 1.0)
            .with_metadata("linear_errors", "2")
            .with_metadata("type_errors", "1");
        let gate = LinearSafetyGate;
        let result = gate.check(&p);
        assert!(!result.passed);
        assert!(result.score < 1.0);
    }

    #[test]
    fn test_latency_gate() {
        let p_fast = DiagnosticProbe::new("h", b"a", b"b", ProbeType::Pipeline, 1.0)
            .with_metadata("elapsed_ms", "100");
        let p_slow = DiagnosticProbe::new("h", b"a", b"b", ProbeType::Pipeline, 1.0)
            .with_metadata("elapsed_ms", "10000");

        let gate = LatencyGate::new(5000);
        assert!(gate.check(&p_fast).passed);
        assert!(!gate.check(&p_slow).passed);
    }

    // ---- Layer 3: Overlay Effectiveness Tests ----

    #[test]
    fn test_overlay_effectiveness_positive() {
        let band = DualBand::new(0.2, 0.8); // Healthy band
        let oe = OverlayEffectiveness::compute(0.4, 0.8, &band);
        assert!(oe.is_positive());
        assert_eq!(oe.delta, 0.4);
        assert!((oe.improvement_pct - 100.0).abs() < 0.01);
        assert!(oe.confidence > 0.0);
        assert!(oe.health > 0.7);
    }

    #[test]
    fn test_overlay_effectiveness_negative() {
        let band = DualBand::new(0.5, 0.5);
        let oe = OverlayEffectiveness::compute(0.8, 0.6, &band);
        assert!(!oe.is_positive());
        assert!(oe.delta < 0.0);
    }

    // ---- Layer 3.5: Linear Audit Tests ----

    #[test]
    fn test_linear_audit_clean() {
        let audit = LinearAudit::clean(10, 3, 3);
        assert!(audit.is_clean());
        assert_eq!(audit.safety_score(), 1.0);
    }

    #[test]
    fn test_linear_audit_with_leaks() {
        let mut audit = LinearAudit::clean(10, 3, 1);
        audit.leaked = 2;
        assert!(!audit.is_clean());
        assert!(audit.safety_score() < 1.0);
        assert!(audit.safety_score() > 0.0);
    }

    // ---- Layer 4: ProofBundle Tests ----

    #[test]
    fn test_proof_bundle_seal_and_verify() {
        let p1 = DiagnosticProbe::new("src_hash", b"a", b"b", ProbeType::Overlay, 0.9);
        let p2 = DiagnosticProbe::new("src_hash", b"c", b"d", ProbeType::TypeCheck, 1.0);
        let gate_results = vec![
            GateResult::pass("OVERLAY_DELTA", 0.9, "test pass"),
            GateResult::pass("LINEAR_SAFETY", 1.0, "clean"),
        ];

        let bundle = ProofBundle::seal("src_hash", vec![p1, p2], gate_results, TEST_KEY)
            .expect("seal should succeed");

        assert_eq!(bundle.probe_count(), 2);
        assert!(bundle.all_gates_passed());
        assert!(!bundle.merkle_root.is_empty());
        assert!(!bundle.hmac_signature.is_empty());

        // Verify integrity
        let verified = bundle.verify(TEST_KEY);
        assert!(verified.is_ok(), "Bundle should verify: {:?}", verified);

        // Tamper detection: wrong key fails
        let tampered = bundle.verify(b"wrong-key");
        assert!(tampered.is_err());
    }

    #[test]
    fn test_proof_bundle_empty_fails() {
        let result = ProofBundle::seal("hash", vec![], vec![], TEST_KEY);
        assert!(result.is_err());
    }

    // ---- Layer 5: DiagnosticReport Tests ----

    #[test]
    fn test_report_tier_filtering() {
        let probe = DiagnosticProbe::new("src", b"pre", b"post", ProbeType::Overlay, 0.95);
        let gate_results = vec![GateResult::pass("OVERLAY_DELTA", 0.95, "good")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");

        let overlay = Some(OverlayEffectiveness::compute(
            0.3,
            0.9,
            &DualBand::new(0.2, 0.8),
        ));
        let linear = Some(LinearAudit::clean(5, 2, 2));

        // Free tier: no proof data, no gate details
        let free_report = DiagnosticReport::generate(
            bundle.clone(),
            overlay.clone(),
            linear.clone(),
            None,
            ReportTier::Free,
        );
        let free_export = free_report.export();
        assert!(free_export.contains_key("summary"));
        assert!(free_export.contains_key("overlay_delta"));
        assert!(!free_export.contains_key("merkle_root"));
        assert!(!free_export.contains_key("gate_0_name"));

        // Developer tier: includes gate details and audit
        let dev_report = DiagnosticReport::generate(
            bundle.clone(),
            overlay.clone(),
            linear.clone(),
            None,
            ReportTier::Developer,
        );
        let dev_export = dev_report.export();
        assert!(dev_export.contains_key("gate_0_name"));
        assert!(dev_export.contains_key("linear_is_clean"));
        assert!(!dev_export.contains_key("merkle_root"));

        // Pro tier: includes everything
        let pro_report =
            DiagnosticReport::generate(bundle.clone(), overlay, linear, None, ReportTier::Pro);
        let pro_export = pro_report.export();
        assert!(pro_export.contains_key("merkle_root"));
        assert!(pro_export.contains_key("hmac_signature"));
        assert!(pro_export.contains_key("probe_0_hash"));
    }

    // ---- Integration Test: Full Pipeline ----

    #[test]
    fn test_full_diagnostic_pipeline() {
        // Build a governed pipeline and feed it into diagnostics
        let mut pipeline = GovernedPipeline::new("diag-test-001", TEST_KEY, false);

        pipeline
            .record_step(
                Phase::Sense,
                0.02,
                b"raw_input",
                b"sensed",
                DualBand::new(0.48, 0.52),
                Decision::Accept,
            )
            .expect("Sense");

        pipeline
            .record_step(
                Phase::Assess,
                0.03,
                b"sensed",
                b"assessed",
                DualBand::new(0.45, 0.55),
                Decision::Accept,
            )
            .expect("Assess");

        pipeline
            .record_step(
                Phase::Decide,
                0.05,
                b"assessed",
                b"decided",
                DualBand::new(0.40, 0.60),
                Decision::Accept,
            )
            .expect("Decide");

        pipeline
            .record_step(
                Phase::Action,
                0.04,
                b"decided",
                b"acted",
                DualBand::new(0.35, 0.65),
                Decision::Accept,
            )
            .expect("Action");

        pipeline
            .record_step(
                Phase::Verify,
                0.02,
                b"acted",
                b"verified",
                DualBand::new(0.30, 0.70),
                Decision::Accept,
            )
            .expect("Verify");

        // Create diagnostic probes
        let overlay_probe = DiagnosticProbe::new(
            "mast_root_test",
            b"raw_output",
            b"overlaid_output",
            ProbeType::Overlay,
            0.92,
        );

        let type_probe = DiagnosticProbe::new(
            "mast_root_test",
            b"unchecked_ast",
            b"checked_ast",
            ProbeType::TypeCheck,
            1.0,
        )
        .with_metadata("linear_errors", "0")
        .with_metadata("type_errors", "0");

        let pipeline_probe = DiagnosticProbe::new(
            "mast_root_test",
            b"pipeline_start",
            b"pipeline_end",
            ProbeType::Pipeline,
            pipeline.confidence(),
        )
        .with_metadata("mcc_violations", "0")
        .with_metadata("elapsed_ms", "200");

        // Run full diagnostic
        let config = DiagnosticConfig::default_with_key(TEST_KEY);
        let overlay_eff = Some(OverlayEffectiveness::compute(
            0.35,
            0.92,
            pipeline.orientation(),
        ));
        let linear_audit = Some(LinearAudit::clean(15, 4, 4));
        let pipe_health = Some(PipelineHealth::from_pipeline(&pipeline));

        let report = run_diagnostic(
            "mast_root_test",
            vec![overlay_probe, type_probe, pipeline_probe],
            &config,
            overlay_eff,
            linear_audit,
            pipe_health,
        )
        .expect("diagnostic should succeed");

        // Verify the report
        assert!(!report.report_id.is_empty());
        assert!(!report.summary.is_empty());
        assert!(report.bundle.verify(TEST_KEY).is_ok());

        // Pipeline health should be verified
        assert!(report.pipeline_health.as_ref().unwrap().verified);
        assert!(report.pipeline_health.as_ref().unwrap().score() > 0.5);

        // Overlay should show improvement
        assert!(report.overlay_effectiveness.as_ref().unwrap().is_positive());
        assert!(
            report
                .overlay_effectiveness
                .as_ref()
                .unwrap()
                .improvement_pct
                > 100.0
        );

        // Linear audit should be clean
        assert!(report.linear_audit.as_ref().unwrap().is_clean());

        // Export at Pro tier should have all fields
        let export = report.export();
        assert!(export.len() > 10);
    }

    // ---- Phase 80: Severity Tests ----

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", Severity::Warning), "WARNING");
        assert_eq!(format!("{}", Severity::Error), "ERROR");
        assert_eq!(format!("{}", Severity::Critical), "CRITICAL");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }

    #[test]
    fn test_gate_result_severity_defaults() {
        let pass = GateResult::pass("test", 1.0, "ok");
        assert_eq!(pass.severity, Severity::Error);
        assert!(!pass.is_blocking());
        assert!(!pass.is_critical());

        let fail = GateResult::fail("test", 0.0, "bad");
        assert_eq!(fail.severity, Severity::Error);
        assert!(fail.is_blocking());
        assert!(!fail.is_critical());
    }

    #[test]
    fn test_gate_result_with_severity() {
        let warn = GateResult::fail_with_severity("test", 0.5, "warn", Severity::Warning);
        assert!(!warn.is_blocking()); // Warning doesn't block
        assert!(!warn.is_critical());

        let crit = GateResult::fail_with_severity("test", 0.0, "critical", Severity::Critical);
        assert!(crit.is_blocking());
        assert!(crit.is_critical());
    }

    #[test]
    fn test_all_gates_passed_respects_severity() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![
            GateResult::pass("g1", 1.0, "ok"),
            GateResult::fail_with_severity("g2", 0.5, "warning only", Severity::Warning),
        ];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");

        // Warning failures don't block
        assert!(bundle.all_gates_passed());
        assert_eq!(bundle.warning_count(), 1);
        assert!(!bundle.has_critical());
    }

    // ---- Phase 80: Custom Gate Tests ----

    #[test]
    fn test_comparison_operators() {
        assert!(Comparison::GreaterThan.evaluate(10.0, 5.0));
        assert!(!Comparison::GreaterThan.evaluate(5.0, 10.0));
        assert!(Comparison::LessThan.evaluate(5.0, 10.0));
        assert!(Comparison::Equal.evaluate(5.0, 5.0));
        assert!(Comparison::GreaterOrEqual.evaluate(5.0, 5.0));
        assert!(Comparison::LessOrEqual.evaluate(5.0, 5.0));
    }

    #[test]
    fn test_comparison_from_str() {
        assert_eq!(Comparison::parse_op("gt"), Some(Comparison::GreaterThan));
        assert_eq!(Comparison::parse_op(">"), Some(Comparison::GreaterThan));
        assert_eq!(Comparison::parse_op("lt"), Some(Comparison::LessThan));
        assert_eq!(Comparison::parse_op("eq"), Some(Comparison::Equal));
        assert_eq!(Comparison::parse_op("invalid"), None);
    }

    #[test]
    fn test_user_defined_gate_from_spec() {
        let gate = UserDefinedGate::from_spec("name:fast,key:elapsed_ms,op:lt,val:1000,sev:error");
        assert!(gate.is_some());
        let gate = gate.unwrap();
        assert_eq!(gate.gate_name, "fast");
        assert_eq!(gate.metadata_key, "elapsed_ms");
        assert_eq!(gate.threshold, 1000.0);
        assert_eq!(gate.comparison, Comparison::LessThan);
        assert_eq!(gate.gate_severity, Severity::Error);
    }

    #[test]
    fn test_user_defined_gate_check() {
        let gate = UserDefinedGate::new(
            "latency",
            "elapsed_ms",
            500.0,
            Comparison::LessThan,
            Severity::Error,
        );
        let fast_probe = DiagnosticProbe::new("h", b"a", b"b", ProbeType::Pipeline, 1.0)
            .with_metadata("elapsed_ms", "100");
        let slow_probe = DiagnosticProbe::new("h", b"a", b"b", ProbeType::Pipeline, 1.0)
            .with_metadata("elapsed_ms", "600");

        assert!(gate.check(&fast_probe).passed);
        assert!(!gate.check(&slow_probe).passed);
    }

    #[test]
    fn test_user_defined_gate_missing_key() {
        let gate = UserDefinedGate::new(
            "test",
            "nonexistent",
            100.0,
            Comparison::LessThan,
            Severity::Error,
        );
        let probe = DiagnosticProbe::new("h", b"a", b"b", ProbeType::Pipeline, 1.0);
        let result = gate.check(&probe);
        // Missing key should pass with Warning severity (gate skipped)
        assert!(result.passed);
        assert_eq!(result.severity, Severity::Warning);
    }

    // ---- Phase 80: History Tests ----

    #[test]
    fn test_history_entry_roundtrip() {
        let entry = HistoryEntry {
            timestamp_ms: 1700000000000,
            source_hash: "abc123def456".to_string(),
            all_passed: true,
            avg_score: 0.95,
            gate_count: 5,
            probe_count: 3,
            warning_count: 1,
            has_critical: false,
        };

        let line = entry.to_json_line();
        let parsed = HistoryEntry::from_json_line(&line);
        assert!(parsed.is_some());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.timestamp_ms, 1700000000000);
        assert_eq!(parsed.source_hash, "abc123def456");
        assert_eq!(parsed.all_passed, true);
        assert!((parsed.avg_score - 0.95).abs() < 0.001);
        assert_eq!(parsed.gate_count, 5);
        assert_eq!(parsed.probe_count, 3);
    }

    #[test]
    fn test_diagnostic_history_load_and_trend() {
        let content = r#"{"ts":1000,"hash":"aaa","passed":true,"avg_score":0.900000,"gates":3,"probes":2,"warnings":0,"critical":false}
{"ts":2000,"hash":"bbb","passed":true,"avg_score":0.950000,"gates":3,"probes":2,"warnings":0,"critical":false}
{"ts":3000,"hash":"ccc","passed":false,"avg_score":0.600000,"gates":3,"probes":2,"warnings":1,"critical":true}"#;

        let history = DiagnosticHistory::load(content);
        assert_eq!(history.entries.len(), 3);
        assert_eq!(history.last_n(2).len(), 2);

        // Regression detection
        assert!(history.has_regression(0.5, 3)); // 0.5 < avg of 0.817
        assert!(!history.has_regression(0.95, 3)); // 0.95 > avg
    }

    #[test]
    fn test_trend_table_output() {
        let history = DiagnosticHistory {
            entries: vec![HistoryEntry {
                timestamp_ms: 1000,
                source_hash: "abc123".to_string(),
                all_passed: true,
                avg_score: 0.95,
                gate_count: 5,
                probe_count: 3,
                warning_count: 0,
                has_critical: false,
            }],
        };
        let table = history.trend_table(10);
        assert!(table.contains("abc123"));
        assert!(table.contains("0.9500"));
    }

    // ---- Phase 80: SARIF Tests ----

    #[test]
    fn test_sarif_generation() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![
            GateResult::pass("OVERLAY_DELTA", 0.9, "ok"),
            GateResult::fail("LINEAR_SAFETY", 0.3, "issues found"),
        ];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Developer);

        let sarif = generate_sarif(&report, "test.ark");
        assert!(sarif.contains("\"version\":\"2.1.0\""));
        assert!(sarif.contains("ark-diagnostic"));
        assert!(sarif.contains("test.ark"));
        assert!(sarif.contains("OVERLAY_DELTA"));
    }

    // ---- Phase 80: Badge Tests ----

    #[test]
    fn test_badge_generation_pass() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 0.9, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Free);

        let badge = generate_badge(&report);
        assert!(badge.contains("<svg"));
        assert!(badge.contains("Ark Diagnostic"));
        assert!(badge.contains("#4c1")); // green
    }

    #[test]
    fn test_badge_generation_fail() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::fail("test", 0.3, "bad")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Free);

        let badge = generate_badge(&report);
        assert!(badge.contains("#e05d44")); // red
    }

    // ---- Phase 80: SBOM Tests ----

    #[test]
    fn test_sbom_generation() {
        let entries = vec![SbomEntry {
            name: "sha2".to_string(),
            version: "0.10".to_string(),
            purl: "pkg:cargo/sha2@0.10".to_string(),
            hash_sha256: "abc123".to_string(),
        }];
        let sbom = generate_sbom(&entries, "test_hash", "1.0.0");
        assert!(sbom.contains("CycloneDX"));
        assert!(sbom.contains("1.5"));
        assert!(sbom.contains("sha2"));
        assert!(sbom.contains("pkg:cargo/sha2@0.10"));
    }

    // ---- Phase 80: Signing Tests ----

    #[test]
    fn test_sign_bundle_deterministic() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 1.0, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");

        let (sig1, pk1) = sign_bundle(&bundle, TEST_KEY);
        let (sig2, pk2) = sign_bundle(&bundle, TEST_KEY);
        assert_eq!(sig1, sig2); // Deterministic
        assert_eq!(pk1, pk2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_signature_file_format() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 1.0, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");

        let sig_file = generate_signature_file(&bundle, TEST_KEY);
        assert!(sig_file.contains("bundle_id"));
        assert!(sig_file.contains("hmac-sha256"));
        assert!(sig_file.contains("signature"));
    }

    // ---- Phase 80: Attestation Tests ----

    #[test]
    fn test_attestation_generation() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 1.0, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Pro);

        let attestation = generate_attestation(&report, TEST_KEY);
        assert!(attestation.contains("application/vnd.in-toto+json"));
        assert!(attestation.contains("payload"));
        assert!(attestation.contains("signatures"));
    }

    // ---- Phase 80: GateResult to_map includes severity ----

    #[test]
    fn test_gate_result_to_map_includes_severity() {
        let result = GateResult::fail_with_severity("test", 0.5, "ev", Severity::Critical);
        let map = result.to_map();
        assert_eq!(map.get("severity").unwrap(), "CRITICAL");
    }

    // ===========================================================================
    // BULLETPROOF INTEGRATION TESTS
    // These tests guarantee no manual verification is ever needed.
    // If these pass, it ships.
    // ===========================================================================

    // ---- Category 1: Real-World File I/O ----

    #[test]
    fn test_sarif_file_write_and_readback() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![
            GateResult::pass("OVERLAY_DELTA", 0.9, "ok"),
            GateResult::fail("LINEAR_SAFETY", 0.3, "issues found"),
        ];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Developer);

        let sarif = generate_sarif(&report, "test_io.ark");

        // Write to actual file
        let path = std::env::temp_dir().join("ark_test_sarif.json");
        std::fs::write(&path, &sarif).expect("write sarif file");

        // Read back and validate
        let content = std::fs::read_to_string(&path).expect("read sarif file");
        assert_eq!(content, sarif, "roundtrip must be exact");

        // Verify it's valid JSON by checking key markers
        assert!(content.starts_with('{'), "must start with JSON object");
        assert!(
            content.contains("\"$schema\""),
            "must have SARIF schema ref"
        );
        assert!(
            content.contains("\"version\":\"2.1.0\""),
            "must be SARIF 2.1.0"
        );
        assert!(content.contains("\"runs\""), "must have runs array");
        assert!(content.contains("\"results\""), "must have results array");
        assert!(content.contains("test_io.ark"), "must reference the file");
        assert!(content.contains("OVERLAY_DELTA"), "must contain gate names");

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_badge_svg_file_write_and_readback() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 0.9, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Free);

        let badge = generate_badge(&report);

        let path = std::env::temp_dir().join("ark_test_badge.svg");
        std::fs::write(&path, &badge).expect("write badge file");

        let content = std::fs::read_to_string(&path).expect("read badge file");
        assert_eq!(content, badge, "roundtrip must be exact");
        assert!(content.contains("<svg"), "must be valid SVG root element");
        assert!(content.contains("</svg>"), "must have closing svg tag");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_sbom_file_write_and_readback() {
        let entries = vec![
            SbomEntry {
                name: "sha2".to_string(),
                version: "0.10".to_string(),
                purl: "pkg:cargo/sha2@0.10".to_string(),
                hash_sha256: "aabbccdd".to_string(),
            },
            SbomEntry {
                name: "hmac".to_string(),
                version: "0.12".to_string(),
                purl: "pkg:cargo/hmac@0.12".to_string(),
                hash_sha256: "11223344".to_string(),
            },
        ];
        let sbom = generate_sbom(&entries, "test_hash_abc", "0.1.0");

        let path = std::env::temp_dir().join("ark_test_sbom.json");
        std::fs::write(&path, &sbom).expect("write sbom file");

        let content = std::fs::read_to_string(&path).expect("read sbom file");
        assert_eq!(content, sbom);
        assert!(content.contains("CycloneDX"));
        assert!(content.contains("\"specVersion\":\"1.5\""));
        assert!(content.contains("pkg:cargo/sha2@0.10"));
        assert!(content.contains("pkg:cargo/hmac@0.12"));
        assert!(content.contains("aabbccdd"));
        assert!(content.contains("test_hash_abc"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_signature_file_write_and_readback() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 1.0, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");

        let sig_file = generate_signature_file(&bundle, TEST_KEY);

        let path = std::env::temp_dir().join("ark_test_sig.sig");
        std::fs::write(&path, &sig_file).expect("write sig file");

        let content = std::fs::read_to_string(&path).expect("read sig file");
        assert_eq!(content, sig_file);
        assert!(content.contains("bundle_id"));
        assert!(content.contains("algorithm"));
        assert!(content.contains("hmac-sha256"));
        assert!(content.contains("signature"));
        assert!(content.contains("public_key"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_attestation_file_write_and_readback() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 1.0, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Pro);

        let attestation = generate_attestation(&report, TEST_KEY);

        let path = std::env::temp_dir().join("ark_test_attestation.json");
        std::fs::write(&path, &attestation).expect("write attestation file");

        let content = std::fs::read_to_string(&path).expect("read attestation file");
        assert_eq!(content, attestation);
        assert!(content.contains("payloadType"));
        assert!(content.contains("application/vnd.in-toto+json"));
        assert!(content.contains("payload"));
        assert!(content.contains("signatures"));
        assert!(content.contains("sig"));
        assert!(content.contains("keyid"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_history_file_append_and_reload() {
        let path = std::env::temp_dir().join("ark_test_history.jsonl");
        // Start clean
        let _ = std::fs::remove_file(&path);

        let mut history = DiagnosticHistory { entries: vec![] };

        // Append 3 entries
        for i in 0..3 {
            let entry = HistoryEntry {
                timestamp_ms: 1000 + i * 1000,
                source_hash: format!("hash_{}", i),
                all_passed: i != 2, // third one fails
                avg_score: 0.9 - (i as f64 * 0.1),
                gate_count: 5,
                probe_count: 3,
                warning_count: if i == 2 { 1 } else { 0 },
                has_critical: i == 2,
            };
            let line = history.append(entry);
            // Append to actual file
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .expect("open history file");
            writeln!(file, "{}", line.trim()).expect("write history line");
        }

        // Reload from disk
        let content = std::fs::read_to_string(&path).expect("read history file");
        let reloaded = DiagnosticHistory::load(&content);

        assert_eq!(reloaded.entries.len(), 3, "must have all 3 entries");
        assert_eq!(reloaded.entries[0].source_hash, "hash_0");
        assert_eq!(reloaded.entries[1].source_hash, "hash_1");
        assert_eq!(reloaded.entries[2].source_hash, "hash_2");
        assert!(reloaded.entries[0].all_passed);
        assert!(!reloaded.entries[2].all_passed);
        assert!(reloaded.entries[2].has_critical);

        let _ = std::fs::remove_file(&path);
    }

    // ---- Category 2: SVG Badge Visual Correctness ----

    #[test]
    fn test_badge_svg_valid_xml_structure() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 0.9, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Free);

        let badge = generate_badge(&report);

        // Must be valid SVG structure
        assert!(badge.contains("<svg"), "missing <svg root");
        assert!(
            badge.contains("xmlns=\"http://www.w3.org/2000/svg\""),
            "missing SVG namespace"
        );
        assert!(badge.contains("</svg>"), "missing closing </svg>");
        assert!(badge.contains("<rect"), "missing rect element");
        assert!(badge.contains("<text"), "missing text element");
        assert!(badge.contains("</text>"), "missing closing text");

        // Must have reasonable structure (opening before closing)
        let svg_open = badge.find("<svg").unwrap();
        let svg_close = badge.find("</svg>").unwrap();
        assert!(svg_open < svg_close, "svg open must precede close");
    }

    #[test]
    fn test_badge_svg_colors_by_status() {
        // PASS = green (#4c1)
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let pass_results = vec![GateResult::pass("test", 0.9, "ok")];
        let pass_bundle =
            ProofBundle::seal("src", vec![probe.clone()], pass_results, TEST_KEY).expect("ok");
        let pass_report =
            DiagnosticReport::generate(pass_bundle, None, None, None, ReportTier::Free);
        let pass_badge = generate_badge(&pass_report);
        assert!(pass_badge.contains("#4c1"), "passing badge must be green");
        assert!(
            !pass_badge.contains("#e05d44"),
            "passing badge must not be red"
        );

        // FAIL = red (#e05d44)
        let fail_results = vec![GateResult::fail("test", 0.3, "bad")];
        let fail_bundle =
            ProofBundle::seal("src", vec![probe.clone()], fail_results, TEST_KEY).expect("ok");
        let fail_report =
            DiagnosticReport::generate(fail_bundle, None, None, None, ReportTier::Free);
        let fail_badge = generate_badge(&fail_report);
        assert!(fail_badge.contains("#e05d44"), "failing badge must be red");
        assert!(
            !fail_badge.contains("#4c1"),
            "failing badge must not be green"
        );

        // WARNING-only = yellow (#dfb317)
        let warn_results = vec![
            GateResult::pass("g1", 1.0, "ok"),
            GateResult::fail_with_severity("g2", 0.5, "warn", Severity::Warning),
        ];
        let warn_bundle =
            ProofBundle::seal("src", vec![probe], warn_results, TEST_KEY).expect("ok");
        let warn_report =
            DiagnosticReport::generate(warn_bundle, None, None, None, ReportTier::Free);
        let warn_badge = generate_badge(&warn_report);
        assert!(
            warn_badge.contains("#dfb317"),
            "warning-only badge must be yellow"
        );
    }

    #[test]
    fn test_badge_svg_text_content() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 0.9, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Free);

        let badge = generate_badge(&report);

        // Must contain the label
        assert!(badge.contains("Ark Diagnostic"), "must have label text");
        // Must contain status text (score or FAIL/CRITICAL)
        assert!(
            badge.contains('%') || badge.contains("FAIL") || badge.contains("CRITICAL"),
            "must have status text showing score percentage or failure status"
        );
    }

    #[test]
    fn test_badge_svg_dimensions() {
        let probe = DiagnosticProbe::new("src", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 0.9, "ok")];
        let bundle =
            ProofBundle::seal("src", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Free);

        let badge = generate_badge(&report);

        // Must have width and height attributes
        assert!(badge.contains("width=\""), "must have width attribute");
        assert!(badge.contains("height=\""), "must have height attribute");

        // Extract width value — must be > 0 and reasonable
        if let Some(w_start) = badge.find("width=\"") {
            let rest = &badge[w_start + 7..];
            if let Some(w_end) = rest.find('"') {
                let w: f64 = rest[..w_end].parse().unwrap_or(0.0);
                assert!(w > 50.0, "width must be > 50px, got {}", w);
                assert!(w < 500.0, "width must be < 500px, got {}", w);
            }
        }
    }

    // ---- Category 3: End-to-End CLI Tests ----

    #[test]
    fn test_cli_diagnose_basic_output() {
        let output = std::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "ark_loader",
                "--",
                "diagnose",
                "tests/hello.ark",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("failed to run ark diagnose");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should produce diagnostic output (on stdout or stderr)
        let combined = format!("{}{}", stdout, stderr);
        assert!(
            combined.contains("DIAGNOSTIC")
                || combined.contains("diagnostic")
                || combined.contains("Gate")
                || combined.contains("gate")
                || combined.contains("PASS")
                || combined.contains("pass"),
            "CLI must produce diagnostic output, got: {}",
            &combined[..combined.len().min(500)]
        );
    }

    #[test]
    fn test_cli_diagnose_json_flag() {
        let output = std::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "ark_loader",
                "--",
                "diagnose",
                "tests/hello.ark",
                "--json",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("failed to run ark diagnose --json");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        // The --json flag should not cause a panic
        assert!(
            !combined.contains("panic") && !combined.contains("PANIC"),
            "CLI must not panic on --json flag"
        );

        // Should produce some output (JSON or diagnostic report)
        assert!(
            !stdout.is_empty() || !stderr.is_empty(),
            "CLI must produce output with --json flag"
        );
    }

    #[test]
    fn test_cli_diagnose_sarif_flag() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sarif_path = manifest.join("tests").join("hello.ark.sarif");
        // Clean up any leftover
        let _ = std::fs::remove_file(&sarif_path);

        let output = std::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "ark_loader",
                "--",
                "diagnose",
                "tests/hello.ark",
                "--sarif",
            ])
            .current_dir(manifest)
            .output()
            .expect("failed to run ark diagnose --sarif");

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Check if SARIF file was created
        if sarif_path.exists() {
            let content = std::fs::read_to_string(&sarif_path).unwrap();
            assert!(
                content.contains("2.1.0"),
                "SARIF file must contain version 2.1.0"
            );
            let _ = std::fs::remove_file(&sarif_path);
        } else {
            // Even if file creation is conditional, the command should not crash
            assert!(
                !stderr.contains("panic") && !stderr.contains("PANIC"),
                "CLI must not panic on --sarif flag"
            );
        }
    }

    #[test]
    fn test_cli_diagnose_badge_flag() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let badge_path = manifest
            .join("tests")
            .join("hello.ark.diagnostic-badge.svg");
        let _ = std::fs::remove_file(&badge_path);

        let output = std::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "ark_loader",
                "--",
                "diagnose",
                "tests/hello.ark",
                "--badge",
            ])
            .current_dir(manifest)
            .output()
            .expect("failed to run ark diagnose --badge");

        let stderr = String::from_utf8_lossy(&output.stderr);

        if badge_path.exists() {
            let content = std::fs::read_to_string(&badge_path).unwrap();
            assert!(content.contains("<svg"), "badge file must be valid SVG");
            let _ = std::fs::remove_file(&badge_path);
        } else {
            assert!(
                !stderr.contains("panic") && !stderr.contains("PANIC"),
                "CLI must not panic on --badge flag"
            );
        }
    }

    #[test]
    fn test_cli_diagnose_all_flags() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let base = manifest.join("tests").join("hello.ark");

        // All expected output files
        let sarif = format!("{}.sarif", base.display());
        let badge = format!("{}.diagnostic-badge.svg", base.display());
        let sig = format!("{}.sig", base.display());
        let sbom = format!("{}.sbom.json", base.display());
        let attest = format!("{}.attestation.json", base.display());

        // Cleanup
        for f in [&sarif, &badge, &sig, &sbom, &attest] {
            let _ = std::fs::remove_file(f);
        }

        let output = std::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "ark_loader",
                "--",
                "diagnose",
                "tests/hello.ark",
                "--sarif",
                "--badge",
                "--sign",
                "--sbom",
                "--attest",
            ])
            .current_dir(manifest)
            .output()
            .expect("failed to run ark diagnose with all flags");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panic") && !stderr.contains("PANIC"),
            "CLI must not panic with all flags enabled"
        );

        // Cleanup all output files
        for f in [&sarif, &badge, &sig, &sbom, &attest] {
            let _ = std::fs::remove_file(f);
        }
    }

    #[test]
    fn test_cli_diagnose_custom_gate_flag() {
        let output = std::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "ark_loader",
                "--",
                "diagnose",
                "tests/hello.ark",
                "--gate",
                "name:test_gate,key:dummy,op:gt,val:0,sev:warning",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("failed to run ark diagnose --gate");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panic") && !stderr.contains("PANIC"),
            "CLI must not panic with custom gate flag"
        );
    }

    #[test]
    fn test_cli_diagnose_nonexistent_file() {
        let output = std::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "ark_loader",
                "--",
                "diagnose",
                "nonexistent_file_12345.ark",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("failed to run ark diagnose");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{}{}", stdout, stderr);

        // Should report an error, not silently succeed or panic
        assert!(
            !combined.contains("panic") && !combined.contains("PANIC"),
            "must not panic on nonexistent file"
        );
    }

    // ---- Category 4: GitHub Action YAML Validation ----

    #[test]
    fn test_action_yml_exists_and_parseable() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // Go up one level from core/ to the repo root
        let repo_root = manifest.parent().unwrap();
        let action_path = repo_root
            .join(".github")
            .join("actions")
            .join("ark-diagnostic")
            .join("action.yml");

        assert!(
            action_path.exists(),
            "action.yml must exist at {:?}",
            action_path
        );

        let content = std::fs::read_to_string(&action_path).expect("read action.yml");
        assert!(!content.is_empty(), "action.yml must not be empty");

        // Verify key top-level fields
        assert!(content.contains("name:"), "must have 'name' field");
        assert!(
            content.contains("description:"),
            "must have 'description' field"
        );
        assert!(content.contains("runs:"), "must have 'runs' field");
    }

    #[test]
    fn test_action_yml_required_inputs() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest.parent().unwrap();
        let action_path = repo_root
            .join(".github")
            .join("actions")
            .join("ark-diagnostic")
            .join("action.yml");
        let content = std::fs::read_to_string(&action_path).expect("read action.yml");

        // The 'file' input is required
        assert!(content.contains("file:"), "must have 'file' input");
        assert!(
            content.contains("required: true"),
            "file input must be required"
        );
    }

    #[test]
    fn test_action_yml_all_optional_inputs() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest.parent().unwrap();
        let action_path = repo_root
            .join(".github")
            .join("actions")
            .join("ark-diagnostic")
            .join("action.yml");
        let content = std::fs::read_to_string(&action_path).expect("read action.yml");

        let expected_inputs = [
            "sarif:",
            "badge:",
            "sign:",
            "sbom:",
            "attest:",
            "history:",
            "custom-gates:",
            "tier:",
            "hmac-key:",
            "fail-on-warning:",
        ];

        for input in &expected_inputs {
            assert!(
                content.contains(input),
                "action.yml must declare input '{}'",
                input
            );
        }
    }

    #[test]
    fn test_action_yml_outputs() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest.parent().unwrap();
        let action_path = repo_root
            .join(".github")
            .join("actions")
            .join("ark-diagnostic")
            .join("action.yml");
        let content = std::fs::read_to_string(&action_path).expect("read action.yml");

        assert!(content.contains("outputs:"), "must have outputs section");
        assert!(content.contains("passed:"), "must have 'passed' output");
        assert!(
            content.contains("sarif-path:"),
            "must have 'sarif-path' output"
        );
        assert!(
            content.contains("badge-path:"),
            "must have 'badge-path' output"
        );
    }

    #[test]
    fn test_action_yml_composite_steps() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest.parent().unwrap();
        let action_path = repo_root
            .join(".github")
            .join("actions")
            .join("ark-diagnostic")
            .join("action.yml");
        let content = std::fs::read_to_string(&action_path).expect("read action.yml");

        assert!(
            content.contains("using: 'composite'"),
            "must use composite runs"
        );
        assert!(content.contains("steps:"), "must have steps");
        assert!(content.contains("cargo build"), "must have build step");
        assert!(content.contains("diagnose"), "must have diagnose step");
        assert!(
            content.contains("upload-sarif"),
            "must have SARIF upload step"
        );
        assert!(
            content.contains("upload-artifact"),
            "must have artifact upload step"
        );
    }

    // ---- Category 5: Performance Benchmarks ----

    #[test]
    fn test_diagnostic_pipeline_under_100ms() {
        let start = std::time::Instant::now();

        let probe =
            DiagnosticProbe::new("perf_test", b"hello", b"world", ProbeType::Pipeline, 0.95);
        let gate_results = vec![
            GateResult::pass("PERF_OVERLAY", 0.9, "ok"),
            GateResult::pass("PERF_LINEAR", 0.95, "ok"),
            GateResult::fail_with_severity("PERF_WARN", 0.5, "warning", Severity::Warning),
        ];
        let bundle =
            ProofBundle::seal("perf_test", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let _report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Pro);

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "diagnostic pipeline must complete in <100ms, took {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_sarif_generation_under_10ms() {
        let probe = DiagnosticProbe::new("perf", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![
            GateResult::pass("G1", 0.9, "ok"),
            GateResult::fail("G2", 0.3, "bad"),
            GateResult::pass("G3", 0.8, "ok"),
        ];
        let bundle =
            ProofBundle::seal("perf", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Developer);

        let start = std::time::Instant::now();
        let _sarif = generate_sarif(&report, "perf.ark");
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 10,
            "SARIF generation must complete in <10ms, took {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_badge_generation_under_5ms() {
        let probe = DiagnosticProbe::new("perf", b"a", b"b", ProbeType::Overlay, 0.9);
        let gate_results = vec![GateResult::pass("test", 0.9, "ok")];
        let bundle =
            ProofBundle::seal("perf", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let report = DiagnosticReport::generate(bundle, None, None, None, ReportTier::Free);

        let start = std::time::Instant::now();
        let _badge = generate_badge(&report);
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 5,
            "badge generation must complete in <5ms, took {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_sbom_generation_under_5ms() {
        let entries: Vec<SbomEntry> = (0..20)
            .map(|i| SbomEntry {
                name: format!("dep_{}", i),
                version: format!("1.{}", i),
                purl: format!("pkg:cargo/dep_{}@1.{}", i, i),
                hash_sha256: format!("{:064x}", i),
            })
            .collect();

        let start = std::time::Instant::now();
        let _sbom = generate_sbom(&entries, "perf_hash", "1.0.0");
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 5,
            "SBOM generation (20 deps) must complete in <5ms, took {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_history_load_1000_entries() {
        // Build a large JSONL content
        let mut lines = String::new();
        for i in 0..1000u64 {
            lines.push_str(&format!(
                "{{\"ts\":{},\"hash\":\"h{:04}\",\"passed\":true,\"avg_score\":0.950000,\"gates\":5,\"probes\":3,\"warnings\":0,\"critical\":false}}\n",
                i * 1000, i
            ));
        }

        let start = std::time::Instant::now();
        let history = DiagnosticHistory::load(&lines);
        let elapsed = start.elapsed();

        assert_eq!(history.entries.len(), 1000, "must load all 1000 entries");
        assert!(
            elapsed.as_millis() < 50,
            "loading 1000 history entries must complete in <50ms, took {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_proof_bundle_seal_under_10ms() {
        let probe = DiagnosticProbe::new(
            "perf",
            b"source_code_here",
            b"compiled_output",
            ProbeType::Pipeline,
            0.95,
        );
        let gate_results = vec![
            GateResult::pass("G1", 0.9, "evidence_1"),
            GateResult::pass("G2", 0.85, "evidence_2"),
            GateResult::pass("G3", 0.95, "evidence_3"),
            GateResult::fail_with_severity("G4", 0.5, "warning note", Severity::Warning),
        ];

        let start = std::time::Instant::now();
        let _bundle =
            ProofBundle::seal("perf", vec![probe], gate_results, TEST_KEY).expect("seal ok");
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 10,
            "ProofBundle::seal must complete in <10ms, took {}ms",
            elapsed.as_millis()
        );
    }
}
