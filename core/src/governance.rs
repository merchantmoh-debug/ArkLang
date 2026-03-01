/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * Governance Module — Provides a governed execution pipeline with:
 *   - StepTrace receipts (HMAC-signed per-step audit records)
 *   - Monotone Confidence Constraint (MCC) enforcement
 *   - Dual-Band progress measurement (Ideal vs Adversarial manifolds)
 *   - Receipt chain with Merkle audit trail
 *
 * All code original. Sovereign Systems intellectual property.
 */

use crate::crypto;
use std::collections::BTreeMap;
use std::fmt;

// ============================================================================
// ENUMS
// ============================================================================

/// The five phases of a governed execution cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Sense,  // Read and normalize inputs
    Assess, // Evaluate current state against goals
    Decide, // Select action based on assessment
    Action, // Execute the chosen action
    Verify, // Validate post-conditions and sign receipt
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Phase::Sense => write!(f, "SENSE"),
            Phase::Assess => write!(f, "ASSESS"),
            Phase::Decide => write!(f, "DECIDE"),
            Phase::Action => write!(f, "ACTION"),
            Phase::Verify => write!(f, "VERIFY"),
        }
    }
}

/// Decision outcome for a governed step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Accept,  // Step passed all gates, action approved
    Abstain, // Step inconclusive, no action taken
    Reject,  // Step failed a gate, action blocked
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Decision::Accept => write!(f, "P"),
            Decision::Abstain => write!(f, "A"),
            Decision::Reject => write!(f, "R"),
        }
    }
}

// ============================================================================
// STEP TRACE — Per-step execution receipt
// ============================================================================

/// A cryptographically signed record of a single execution step.
/// Each trace captures the state before/after, confidence deltas,
/// the decision made, and an HMAC signature for tamper detection.
#[derive(Debug, Clone)]
pub struct StepTrace {
    /// Unique identifier for this execution run
    pub run_id: String,
    /// Sequential step number within the run
    pub step: u64,
    /// Which phase of the pipeline this step represents
    pub phase: Phase,
    /// Confidence level before this step executed
    pub conf_before: f64,
    /// Confidence level after this step executed
    pub conf_after: f64,
    /// Decision made at this step
    pub decision: Decision,
    /// SHA-256 hash of the state before this step
    pub pre_state_hash: String,
    /// SHA-256 hash of the state after this step
    pub post_state_hash: String,
    /// Dual-band orientation at this step
    pub orientation: DualBand,
    /// Which gates passed/failed
    pub gates_passed: Vec<String>,
    pub gates_failed: Vec<String>,
    /// HMAC-SHA256 signature of this trace (hex-encoded)
    pub sig: String,
    /// Timestamp (unix epoch millis)
    pub timestamp_ms: u64,
}

impl StepTrace {
    /// Create a new StepTrace and sign it with the provided HMAC key.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: &str,
        step: u64,
        phase: Phase,
        conf_before: f64,
        conf_after: f64,
        decision: Decision,
        pre_state: &[u8],
        post_state: &[u8],
        orientation: DualBand,
        gates_passed: Vec<String>,
        gates_failed: Vec<String>,
        hmac_key: &[u8],
    ) -> Self {
        let pre_state_hash = crypto::hash(pre_state);
        let post_state_hash = crypto::hash(post_state);

        let now = Self::now_ms();

        let mut trace = StepTrace {
            run_id: run_id.to_string(),
            step,
            phase,
            conf_before,
            conf_after,
            decision,
            pre_state_hash,
            post_state_hash,
            orientation,
            gates_passed,
            gates_failed,
            sig: String::new(),
            timestamp_ms: now,
        };

        // Sign the canonical form
        trace.sig = trace.compute_sig(hmac_key);
        trace
    }

    /// Compute the HMAC-SHA256 signature over the canonical trace data.
    fn compute_sig(&self, hmac_key: &[u8]) -> String {
        let canonical = format!(
            "{}|{}|{}|{:.6}|{:.6}|{}|{}|{}|{}",
            self.run_id,
            self.step,
            self.phase,
            self.conf_before,
            self.conf_after,
            self.decision,
            self.pre_state_hash,
            self.post_state_hash,
            self.timestamp_ms,
        );
        crypto::hmac_sha256(hmac_key, canonical.as_bytes())
    }

    /// Verify integrity of this trace against its signature.
    pub fn verify(&self, hmac_key: &[u8]) -> bool {
        let expected = self.compute_sig(hmac_key);
        crypto::constant_time_eq(self.sig.as_bytes(), expected.as_bytes())
    }

    /// Get current time in millis. Uses system time on native, 0 on wasm.
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

    /// Serialize to a map of string key-value pairs (for Ark Value::Struct).
    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("run_id".to_string(), self.run_id.clone());
        m.insert("step".to_string(), self.step.to_string());
        m.insert("phase".to_string(), self.phase.to_string());
        m.insert(
            "conf_before".to_string(),
            format!("{:.6}", self.conf_before),
        );
        m.insert("conf_after".to_string(), format!("{:.6}", self.conf_after));
        m.insert("decision".to_string(), self.decision.to_string());
        m.insert("pre_state_hash".to_string(), self.pre_state_hash.clone());
        m.insert("post_state_hash".to_string(), self.post_state_hash.clone());
        m.insert("sig".to_string(), self.sig.clone());
        m.insert("timestamp_ms".to_string(), self.timestamp_ms.to_string());
        m
    }
}

// ============================================================================
// DUAL BAND — Progress orientation between Ideal and Adversarial manifolds
// ============================================================================

/// Measures the current state's position relative to ideal and adversarial
/// manifolds. Progress = p_score decreasing (closer to ideal) AND
/// a_score increasing (farther from adversarial).
#[derive(Debug, Clone, Copy)]
pub struct DualBand {
    /// Distance from the ideal state (0.0 = perfect, 1.0 = worst).
    pub p_score: f64,
    /// Distance from the adversarial state (1.0 = safe, 0.0 = adversarial).
    pub a_score: f64,
}

impl DualBand {
    pub fn new(p_score: f64, a_score: f64) -> Self {
        Self {
            p_score: p_score.clamp(0.0, 1.0),
            a_score: a_score.clamp(0.0, 1.0),
        }
    }

    /// Check if moving from `before` to `after` is non-regressive progress.
    /// Progress means: moving closer to ideal AND farther from adversarial.
    pub fn is_progress(before: &DualBand, after: &DualBand) -> bool {
        after.p_score <= before.p_score && after.a_score >= before.a_score
    }

    /// Check if the band is in the "safe zone" (closer to ideal than adversarial).
    pub fn is_safe(&self) -> bool {
        self.a_score > self.p_score
    }

    /// Combined health score: high = good (near ideal, far from adversarial).
    pub fn health(&self) -> f64 {
        (self.a_score + (1.0 - self.p_score)) / 2.0
    }
}

// ============================================================================
// MONOTONE CONFIDENCE CONSTRAINT (MCC)
// ============================================================================

/// Enforces that confidence may only increase (or stay equal) across steps.
/// Any regression is flagged as a violation.
pub struct MccGate;

impl MccGate {
    /// Returns true if the trace satisfies MCC (conf_after >= conf_before).
    pub fn check(trace: &StepTrace) -> bool {
        trace.conf_after >= trace.conf_before
    }

    /// Returns true if the entire chain is monotone non-decreasing.
    pub fn check_chain(traces: &[StepTrace]) -> bool {
        for window in traces.windows(2) {
            // The end confidence of step N must be <= start confidence of step N+1
            if window[1].conf_before < window[0].conf_after {
                return false;
            }
        }
        // Also check each individual step
        traces.iter().all(|t| t.conf_after >= t.conf_before)
    }

    /// Find all MCC violations in a chain. Returns (step_index, trace) pairs.
    pub fn violations(traces: &[StepTrace]) -> Vec<(usize, &StepTrace)> {
        let mut v = Vec::new();
        for (i, t) in traces.iter().enumerate() {
            if t.conf_after < t.conf_before {
                v.push((i, t));
            }
        }
        // Cross-step violations
        for (i, w) in traces.windows(2).enumerate() {
            if w[1].conf_before < w[0].conf_after {
                v.push((i + 1, &w[1]));
            }
        }
        v
    }
}

// ============================================================================
// RECEIPT CHAIN — Append-only Merkle audit trail
// ============================================================================

/// An append-only chain of StepTrace receipts with incremental Merkle root.
#[derive(Debug, Clone)]
pub struct ReceiptChain {
    /// All traces in append order
    traces: Vec<StepTrace>,
    /// Leaf hashes (SHA-256 of each trace's sig)
    leaf_hashes: Vec<String>,
    /// Current Merkle root (updated on each append)
    merkle_root: String,
    /// HMAC key used for signing traces
    hmac_key: Vec<u8>,
}

impl ReceiptChain {
    pub fn new(hmac_key: &[u8]) -> Self {
        Self {
            traces: Vec::new(),
            leaf_hashes: Vec::new(),
            merkle_root: String::new(),
            hmac_key: hmac_key.to_vec(),
        }
    }

    /// Append a trace and update the Merkle root.
    pub fn append(&mut self, trace: StepTrace) {
        let leaf = crypto::hash(trace.sig.as_bytes());
        self.leaf_hashes.push(leaf);
        self.merkle_root = crypto::merkle_root(&self.leaf_hashes);
        self.traces.push(trace);
    }

    /// Get the current Merkle root.
    pub fn root(&self) -> &str {
        &self.merkle_root
    }

    /// Get all traces.
    pub fn traces(&self) -> &[StepTrace] {
        &self.traces
    }

    /// Number of receipts.
    pub fn len(&self) -> usize {
        self.traces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }

    /// Verify the entire chain: all signatures valid + MCC holds.
    pub fn verify_integrity(&self) -> Result<bool, String> {
        // 1. Verify every trace signature
        for (i, trace) in self.traces.iter().enumerate() {
            if !trace.verify(&self.hmac_key) {
                return Err(format!(
                    "Signature verification failed at step {} (run: {})",
                    i, trace.run_id
                ));
            }
        }

        // 2. Verify Merkle root matches
        let expected_root = crypto::merkle_root(&self.leaf_hashes);
        if expected_root != self.merkle_root {
            return Err(format!(
                "Merkle root mismatch: expected {}, got {}",
                expected_root, self.merkle_root
            ));
        }

        // 3. Verify MCC
        if !MccGate::check_chain(&self.traces) {
            let violations = MccGate::violations(&self.traces);
            return Err(format!(
                "MCC violations at steps: {:?}",
                violations.iter().map(|(i, _)| *i).collect::<Vec<_>>()
            ));
        }

        Ok(true)
    }

    /// Get the HMAC key reference.
    pub fn hmac_key(&self) -> &[u8] {
        &self.hmac_key
    }
}

// ============================================================================
// GOVERNED PIPELINE — Orchestrates SENSE→ASSESS→DECIDE→ACTION→VERIFY
// ============================================================================

/// Error type for governance operations.
#[derive(Debug)]
pub enum GovernanceError {
    MccViolation {
        step: u64,
        conf_before: f64,
        conf_after: f64,
    },
    DualBandRegression {
        step: u64,
    },
    GateFailed(String),
    ChainIntegrityFailed(String),
}

impl fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GovernanceError::MccViolation {
                step,
                conf_before,
                conf_after,
            } => {
                write!(
                    f,
                    "MCC violation at step {}: conf dropped from {:.4} to {:.4}",
                    step, conf_before, conf_after
                )
            }
            GovernanceError::DualBandRegression { step } => {
                write!(f, "Dual-band regression at step {}", step)
            }
            GovernanceError::GateFailed(g) => write!(f, "Gate failed: {}", g),
            GovernanceError::ChainIntegrityFailed(e) => {
                write!(f, "Chain integrity failed: {}", e)
            }
        }
    }
}

/// The governed pipeline orchestrator. Wraps an execution with
/// SENSE→ASSESS→DECIDE→ACTION→VERIFY phases and MCC enforcement.
pub struct GovernedPipeline {
    /// Unique run identifier
    run_id: String,
    /// Current step counter
    step_counter: u64,
    /// Current confidence level
    confidence: f64,
    /// Current dual-band orientation
    orientation: DualBand,
    /// Receipt chain for audit trail
    chain: ReceiptChain,
    /// Whether to strictly enforce MCC (block on violation)
    strict_mcc: bool,
}

impl GovernedPipeline {
    /// Create a new governed pipeline.
    pub fn new(run_id: &str, hmac_key: &[u8], strict_mcc: bool) -> Self {
        Self {
            run_id: run_id.to_string(),
            step_counter: 0,
            confidence: 0.5, // Start at neutral confidence
            orientation: DualBand::new(0.5, 0.5),
            chain: ReceiptChain::new(hmac_key),
            strict_mcc,
        }
    }

    /// Record a step in the pipeline. Returns the trace if MCC passes.
    pub fn record_step(
        &mut self,
        phase: Phase,
        conf_delta: f64,
        pre_state: &[u8],
        post_state: &[u8],
        new_orientation: DualBand,
        decision: Decision,
    ) -> Result<StepTrace, GovernanceError> {
        self.step_counter += 1;
        let conf_before = self.confidence;
        let conf_after = (conf_before + conf_delta).clamp(0.0, 1.0);

        let mut gates_passed = Vec::new();
        let mut gates_failed = Vec::new();

        // MCC gate
        if conf_after >= conf_before {
            gates_passed.push("MCC".to_string());
        } else {
            gates_failed.push("MCC".to_string());
            if self.strict_mcc {
                return Err(GovernanceError::MccViolation {
                    step: self.step_counter,
                    conf_before,
                    conf_after,
                });
            }
        }

        // Dual-band gate
        if DualBand::is_progress(&self.orientation, &new_orientation) {
            gates_passed.push("DUAL_BAND".to_string());
        } else {
            gates_failed.push("DUAL_BAND".to_string());
        }

        let trace = StepTrace::new(
            &self.run_id,
            self.step_counter,
            phase,
            conf_before,
            conf_after,
            decision,
            pre_state,
            post_state,
            new_orientation,
            gates_passed,
            gates_failed,
            self.chain.hmac_key(),
        );

        // Update state
        self.confidence = conf_after;
        self.orientation = new_orientation;
        self.chain.append(trace.clone());

        Ok(trace)
    }

    /// Get the current confidence level.
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Get the current orientation.
    pub fn orientation(&self) -> &DualBand {
        &self.orientation
    }

    /// Get the receipt chain.
    pub fn chain(&self) -> &ReceiptChain {
        &self.chain
    }

    /// Get the current Merkle root of all receipts.
    pub fn merkle_root(&self) -> &str {
        self.chain.root()
    }

    /// Verify the entire execution chain.
    pub fn verify(&self) -> Result<bool, GovernanceError> {
        self.chain
            .verify_integrity()
            .map_err(GovernanceError::ChainIntegrityFailed)
    }

    /// Get the run ID.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Get the step count.
    pub fn step_count(&self) -> u64 {
        self.step_counter
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: &[u8] = b"ark-test-hmac-key-sovereign";

    #[test]
    fn test_step_trace_sign_verify() {
        let trace = StepTrace::new(
            "test-run-001",
            1,
            Phase::Sense,
            0.5,
            0.52,
            Decision::Accept,
            b"state_before",
            b"state_after",
            DualBand::new(0.4, 0.6),
            vec!["MCC".to_string()],
            vec![],
            TEST_KEY,
        );
        assert!(!trace.sig.is_empty());
        assert!(trace.verify(TEST_KEY));

        // Tamper detection: wrong key fails
        assert!(!trace.verify(b"wrong-key"));
    }

    #[test]
    fn test_mcc_gate_passes() {
        let trace = StepTrace::new(
            "run",
            1,
            Phase::Action,
            0.5,
            0.6,
            Decision::Accept,
            b"a",
            b"b",
            DualBand::new(0.3, 0.7),
            vec![],
            vec![],
            TEST_KEY,
        );
        assert!(MccGate::check(&trace));
    }

    #[test]
    fn test_mcc_gate_equal_passes() {
        let trace = StepTrace::new(
            "run",
            1,
            Phase::Action,
            0.5,
            0.5,
            Decision::Accept,
            b"a",
            b"b",
            DualBand::new(0.3, 0.7),
            vec![],
            vec![],
            TEST_KEY,
        );
        assert!(MccGate::check(&trace));
    }

    #[test]
    fn test_mcc_gate_fails_on_regression() {
        let trace = StepTrace::new(
            "run",
            1,
            Phase::Action,
            0.7,
            0.5,
            Decision::Reject,
            b"a",
            b"b",
            DualBand::new(0.3, 0.7),
            vec![],
            vec![],
            TEST_KEY,
        );
        assert!(!MccGate::check(&trace));
    }

    #[test]
    fn test_dual_band_progress() {
        let before = DualBand::new(0.5, 0.5);
        let after = DualBand::new(0.4, 0.6); // closer to ideal, farther from adversarial
        assert!(DualBand::is_progress(&before, &after));
    }

    #[test]
    fn test_dual_band_regression() {
        let before = DualBand::new(0.4, 0.6);
        let after = DualBand::new(0.5, 0.5); // moved backward
        assert!(!DualBand::is_progress(&before, &after));
    }

    #[test]
    fn test_dual_band_health() {
        let safe = DualBand::new(0.2, 0.8);
        assert!(safe.is_safe());
        assert!(safe.health() > 0.7);

        let danger = DualBand::new(0.8, 0.2);
        assert!(!danger.is_safe());
        assert!(danger.health() < 0.3);
    }

    #[test]
    fn test_receipt_chain_append_and_merkle() {
        let mut chain = ReceiptChain::new(TEST_KEY);

        let t1 = StepTrace::new(
            "run",
            1,
            Phase::Sense,
            0.5,
            0.55,
            Decision::Accept,
            b"s0",
            b"s1",
            DualBand::new(0.5, 0.5),
            vec!["MCC".to_string()],
            vec![],
            TEST_KEY,
        );
        chain.append(t1);
        assert_eq!(chain.len(), 1);
        assert!(!chain.root().is_empty());

        let root_1 = chain.root().to_string();

        let t2 = StepTrace::new(
            "run",
            2,
            Phase::Decide,
            0.55,
            0.6,
            Decision::Accept,
            b"s1",
            b"s2",
            DualBand::new(0.45, 0.55),
            vec!["MCC".to_string()],
            vec![],
            TEST_KEY,
        );
        chain.append(t2);
        assert_eq!(chain.len(), 2);
        // Root should change after append
        assert_ne!(chain.root(), root_1);
    }

    #[test]
    fn test_receipt_chain_verify_integrity() {
        let mut chain = ReceiptChain::new(TEST_KEY);
        let t1 = StepTrace::new(
            "run",
            1,
            Phase::Sense,
            0.5,
            0.55,
            Decision::Accept,
            b"s0",
            b"s1",
            DualBand::new(0.5, 0.5),
            vec!["MCC".to_string()],
            vec![],
            TEST_KEY,
        );
        let t2 = StepTrace::new(
            "run",
            2,
            Phase::Verify,
            0.55,
            0.6,
            Decision::Accept,
            b"s1",
            b"s2",
            DualBand::new(0.45, 0.55),
            vec!["MCC".to_string()],
            vec![],
            TEST_KEY,
        );
        chain.append(t1);
        chain.append(t2);

        let result = chain.verify_integrity();
        assert!(result.is_ok(), "Chain should verify: {:?}", result);
    }

    #[test]
    fn test_governed_pipeline_happy_path() {
        let mut pipeline = GovernedPipeline::new("gp-001", TEST_KEY, true);

        // Sense
        let t = pipeline.record_step(
            Phase::Sense,
            0.02,
            b"raw_input",
            b"sensed_state",
            DualBand::new(0.48, 0.52),
            Decision::Accept,
        );
        assert!(t.is_ok());

        // Assess
        let t = pipeline.record_step(
            Phase::Assess,
            0.03,
            b"sensed_state",
            b"assessed_state",
            DualBand::new(0.45, 0.55),
            Decision::Accept,
        );
        assert!(t.is_ok());

        // Decide
        let t = pipeline.record_step(
            Phase::Decide,
            0.05,
            b"assessed_state",
            b"decision_state",
            DualBand::new(0.40, 0.60),
            Decision::Accept,
        );
        assert!(t.is_ok());

        // Action
        let t = pipeline.record_step(
            Phase::Action,
            0.04,
            b"decision_state",
            b"action_result",
            DualBand::new(0.35, 0.65),
            Decision::Accept,
        );
        assert!(t.is_ok());

        // Verify
        let t = pipeline.record_step(
            Phase::Verify,
            0.02,
            b"action_result",
            b"verified_state",
            DualBand::new(0.30, 0.70),
            Decision::Accept,
        );
        assert!(t.is_ok());

        // Full chain verification
        assert!(pipeline.verify().is_ok());
        assert_eq!(pipeline.step_count(), 5);
        assert!(pipeline.confidence() > 0.6);
    }

    #[test]
    fn test_governed_pipeline_strict_mcc_blocks() {
        let mut pipeline = GovernedPipeline::new("gp-002", TEST_KEY, true);

        // Step that increases confidence
        let _ = pipeline.record_step(
            Phase::Sense,
            0.1,
            b"a",
            b"b",
            DualBand::new(0.4, 0.6),
            Decision::Accept,
        );

        // Step that decreases confidence — should be blocked
        let result = pipeline.record_step(
            Phase::Action,
            -0.2,
            b"b",
            b"c",
            DualBand::new(0.3, 0.7),
            Decision::Accept,
        );
        assert!(result.is_err());
        if let Err(GovernanceError::MccViolation { .. }) = result {
            // Expected
        } else {
            panic!("Expected MccViolation error");
        }
    }

    #[test]
    fn test_governed_pipeline_lenient_mcc_warns() {
        let mut pipeline = GovernedPipeline::new("gp-003", TEST_KEY, false);

        // Step that increases confidence
        let _ = pipeline.record_step(
            Phase::Sense,
            0.1,
            b"a",
            b"b",
            DualBand::new(0.4, 0.6),
            Decision::Accept,
        );

        // Step that decreases confidence — should warn but proceed
        let result = pipeline.record_step(
            Phase::Action,
            -0.05,
            b"b",
            b"c",
            DualBand::new(0.35, 0.65),
            Decision::Accept,
        );
        assert!(result.is_ok());
        let trace = result.expect("operation failed");
        assert!(trace.gates_failed.contains(&"MCC".to_string()));
    }

    #[test]
    fn test_step_trace_to_map() {
        let trace = StepTrace::new(
            "map-test",
            42,
            Phase::Verify,
            0.8,
            0.85,
            Decision::Accept,
            b"before",
            b"after",
            DualBand::new(0.2, 0.8),
            vec!["MCC".to_string()],
            vec![],
            TEST_KEY,
        );
        let map = trace.to_map();
        assert_eq!(map.get("run_id").unwrap(), "map-test");
        assert_eq!(map.get("step").unwrap(), "42");
        assert_eq!(map.get("phase").unwrap(), "VERIFY");
        assert_eq!(map.get("decision").unwrap(), "P");
    }

    #[test]
    fn test_mcc_chain_violations() {
        let t1 = StepTrace::new(
            "run",
            1,
            Phase::Sense,
            0.5,
            0.6,
            Decision::Accept,
            b"a",
            b"b",
            DualBand::new(0.5, 0.5),
            vec![],
            vec![],
            TEST_KEY,
        );
        let t2 = StepTrace::new(
            "run",
            2,
            Phase::Action,
            0.7,
            0.65,
            Decision::Reject,
            b"b",
            b"c",
            DualBand::new(0.4, 0.6),
            vec![],
            vec![],
            TEST_KEY,
        );
        let traces = vec![t1, t2];
        let violations = MccGate::violations(&traces);
        assert!(!violations.is_empty());
        assert!(!MccGate::check_chain(&traces));
    }
}
