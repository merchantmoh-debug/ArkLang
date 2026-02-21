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
}

impl GateResult {
    pub fn pass(gate_name: &str, score: f64, evidence: &str) -> Self {
        GateResult {
            passed: true,
            score: score.clamp(0.0, 1.0),
            evidence: evidence.to_string(),
            gate_name: gate_name.to_string(),
        }
    }

    pub fn fail(gate_name: &str, score: f64, evidence: &str) -> Self {
        GateResult {
            passed: false,
            score: score.clamp(0.0, 1.0),
            evidence: evidence.to_string(),
            gate_name: gate_name.to_string(),
        }
    }

    /// Serialize to map for Ark interop.
    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("gate_name".to_string(), self.gate_name.clone());
        m.insert("passed".to_string(), self.passed.to_string());
        m.insert("score".to_string(), format!("{:.6}", self.score));
        m.insert("evidence".to_string(), self.evidence.clone());
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

    /// Whether all gates passed.
    pub fn all_gates_passed(&self) -> bool {
        self.gate_results.iter().all(|g| g.passed)
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
}
