/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * QDMA Memory Engine — Quantum Dream Memory Algorithm (core subset).
 *
 * Selective port of Remember-Me-AI's quantum_dream_memory_ultimate.py (v3.0.0).
 * Ports the algorithmic core (VectorSpace, DreamEntity, DetoxSystem,
 * ProjectionEngine, quantize/dequantize) without I/O deps (FAISS, Redis, HTTP).
 *
 * Architecture:
 *   VectorSpace       → Vector math (cosine_sim, normalize, mean, add/sub/scale)
 *   DreamEntity       → Memory unit (embedding, xi, score, importance, trit)
 *   DreamSeed         → Compressed cluster (seed_vec, members, diffs)
 *   Hologram          → Query result (confidence, provenance, delta_E)
 *   DetoxSystem       → Anomaly detection (toxicity_score, is_anomalous, repair)
 *   ProjectionEngine  → Dimensionality reduction (micro→macro, high-dim)
 *   QdmaStore         → In-memory HashMap-backed store with cosine search
 */

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ===========================================================================
// Constants
// ===========================================================================

const DEFAULT_DIM: usize = 384;
const EPSILON: f64 = 1e-12;

// ===========================================================================
// Simple deterministic PRNG for projection matrices
// ===========================================================================

/// Minimal PRNG for generating random projection matrices (no crate dep).
#[derive(Debug, Clone)]
struct ProjRng {
    state: u64,
}

impl ProjRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    /// Approximate standard normal via Box-Muller with LCG.
    fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bits = (self.state >> 11) as f64;
        bits / (1u64 << 53) as f64
    }

    /// Approximate Gaussian(0, 1) using Box-Muller transform.
    fn next_gaussian(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-15);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

// ===========================================================================
// VectorSpace — Unified vector math (pure f64, no numpy)
// ===========================================================================

pub struct VectorSpace;

impl VectorSpace {
    /// Cosine similarity between two vectors.
    pub fn cosine_sim(a: &[f64], b: &[f64]) -> f64 {
        let m = a.len().min(b.len());
        if m == 0 {
            return 0.0;
        }
        let mut dot = 0.0;
        let mut an = 0.0;
        let mut bn = 0.0;
        for i in 0..m {
            dot += a[i] * b[i];
            an += a[i] * a[i];
            bn += b[i] * b[i];
        }
        dot / (an.sqrt() + EPSILON) / (bn.sqrt() + EPSILON)
    }

    /// L2-normalize a vector in place.
    pub fn normalize(vec: &mut [f64]) {
        let norm: f64 = vec.iter().map(|x| x * x).sum::<f64>().sqrt() + EPSILON;
        for x in vec.iter_mut() {
            *x /= norm;
        }
    }

    /// Return a new normalized copy.
    pub fn normalized(vec: &[f64]) -> Vec<f64> {
        let norm: f64 = vec.iter().map(|x| x * x).sum::<f64>().sqrt() + EPSILON;
        vec.iter().map(|x| x / norm).collect()
    }

    /// Mean of a set of vectors.
    pub fn mean_vec(vecs: &[&[f64]]) -> Option<Vec<f64>> {
        if vecs.is_empty() {
            return None;
        }
        let dim = vecs.iter().map(|v| v.len()).max().unwrap_or(0);
        if dim == 0 {
            return None;
        }
        let n = vecs.len() as f64;
        let mut result = vec![0.0; dim];
        for v in vecs {
            for (i, x) in v.iter().enumerate() {
                if i < dim {
                    result[i] += x;
                }
            }
        }
        for x in result.iter_mut() {
            *x /= n;
        }
        Some(result)
    }

    /// Element-wise addition.
    pub fn vec_add(a: &[f64], b: &[f64]) -> Vec<f64> {
        let dim = a.len().max(b.len());
        (0..dim)
            .map(|i| {
                let av = if i < a.len() { a[i] } else { 0.0 };
                let bv = if i < b.len() { b[i] } else { 0.0 };
                av + bv
            })
            .collect()
    }

    /// Element-wise subtraction.
    pub fn vec_sub(a: &[f64], b: &[f64]) -> Vec<f64> {
        let dim = a.len().max(b.len());
        (0..dim)
            .map(|i| {
                let av = if i < a.len() { a[i] } else { 0.0 };
                let bv = if i < b.len() { b[i] } else { 0.0 };
                av - bv
            })
            .collect()
    }

    /// Scalar multiplication.
    pub fn vec_scale(vec: &[f64], scale: f64) -> Vec<f64> {
        vec.iter().map(|x| x * scale).collect()
    }

    /// L2 norm.
    pub fn norm(vec: &[f64]) -> f64 {
        vec.iter().map(|x| x * x).sum::<f64>().sqrt()
    }
}

// ===========================================================================
// Quantization — Vector compression
// ===========================================================================

/// Quantization metadata.
#[derive(Debug, Clone)]
pub struct QuantMeta {
    pub min_val: f64,
    pub max_val: f64,
    pub bits: u32,
}

/// Quantize a list of vectors to `bits`-bit integers.
pub fn quantize_list(vecs: &[Vec<f64>], bits: u32) -> (Vec<Vec<u32>>, QuantMeta) {
    let flat: Vec<f64> = vecs.iter().flat_map(|v| v.iter().copied()).collect();
    if flat.is_empty() {
        return (
            vec![],
            QuantMeta {
                min_val: 0.0,
                max_val: 0.0,
                bits,
            },
        );
    }
    let mn = flat.iter().copied().fold(f64::INFINITY, f64::min);
    let mx = flat.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let meta = QuantMeta {
        min_val: mn,
        max_val: mx,
        bits,
    };

    if (mx - mn).abs() < EPSILON {
        let qvecs = vecs.iter().map(|v| vec![0u32; v.len()]).collect();
        return (qvecs, meta);
    }

    let levels = ((1u64 << bits) - 1) as f64;
    let qvecs = vecs
        .iter()
        .map(|v| {
            v.iter()
                .map(|x| ((x - mn) / (mx - mn) * levels).round() as u32)
                .collect()
        })
        .collect();
    (qvecs, meta)
}

/// Dequantize a quantized vector.
pub fn dequantize(qvec: &[u32], meta: &QuantMeta) -> Vec<f64> {
    let levels = ((1u64 << meta.bits) - 1) as f64;
    if levels == 0.0 {
        return qvec.iter().map(|_| meta.min_val).collect();
    }
    qvec.iter()
        .map(|x| meta.min_val + (*x as f64 / levels) * (meta.max_val - meta.min_val))
        .collect()
}

// ===========================================================================
// DreamEntity — Memory unit
// ===========================================================================

#[derive(Debug, Clone)]
pub struct DreamEntity {
    pub id: String,
    pub embedding: Vec<f64>,
    pub shards: Vec<String>,
    pub xi: f64,
    pub score: f64,
    pub importance: f64,
    pub emotion: f64,
    pub trit: i32,
    pub core_protected: bool,
    pub quarantined: bool,
    pub version: u32,
    pub ts: f64,
    pub last_active: f64,
    pub decay_score: f64,
}

impl DreamEntity {
    pub fn new(id: String, embedding: Vec<f64>, shards: Vec<String>) -> Self {
        let now = now_ts();
        Self {
            id,
            embedding,
            shards,
            xi: 0.5,
            score: 1.0,
            importance: 0.0,
            emotion: 0.0,
            trit: 0,
            core_protected: false,
            quarantined: false,
            version: 0,
            ts: now,
            last_active: now,
            decay_score: 0.0,
        }
    }

    /// Touch: update last_active and increment version.
    pub fn touch(&mut self) {
        self.last_active = now_ts();
        self.version += 1;
    }
}

// ===========================================================================
// DreamSeed — Compressed cluster
// ===========================================================================

#[derive(Debug, Clone)]
pub struct DreamSeed {
    pub id: String,
    pub seed_vec: Vec<f64>,
    pub members: Vec<String>,
    pub diffs: HashMap<String, Vec<i32>>,
    pub quant_meta: Option<QuantMeta>,
    pub macro_repr: Option<Vec<f64>>,
    pub ts: f64,
}

impl DreamSeed {
    pub fn new(id: String, seed_vec: Vec<f64>, members: Vec<String>) -> Self {
        Self {
            id,
            seed_vec,
            members,
            diffs: HashMap::new(),
            quant_meta: None,
            macro_repr: None,
            ts: now_ts(),
        }
    }
}

// ===========================================================================
// Hologram — Query result
// ===========================================================================

#[derive(Debug, Clone)]
pub struct Hologram {
    pub id: String,
    pub embedding: Vec<f64>,
    pub confidence: f64,
    pub delta_e: f64,
    pub toxic_score: f64,
}

// ===========================================================================
// DetoxSystem — Anomaly detection and repair
// ===========================================================================

pub struct DetoxSystem;

impl DetoxSystem {
    /// Toxicity score based on magnitude, kurtosis, and distance from background.
    pub fn toxicity_score(emb: &[f64], background: Option<&[f64]>) -> f64 {
        if emb.is_empty() {
            return 0.0;
        }

        let n = emb.len() as f64;
        let mag = VectorSpace::norm(emb);
        let mean: f64 = emb.iter().sum::<f64>() / n;
        let variance: f64 = emb.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std = variance.sqrt() + EPSILON;

        // Kurtosis (fourth standardized moment)
        let kurt: f64 = emb.iter().map(|x| ((x - mean) / std).powi(4)).sum::<f64>() / n;

        let mut score = (mean.abs() / 10.0) * 0.4;
        score += (mag / (n.sqrt() + EPSILON)) * 0.3;
        score += (kurt / 3.0).min(1.0) * 0.3;

        if let Some(bg) = background {
            let diff = VectorSpace::vec_sub(emb, bg);
            let dist = VectorSpace::norm(&diff) / (VectorSpace::norm(bg) + EPSILON);
            score += dist.min(1.0) * 0.2;
        }

        score.min(1.0)
    }

    /// Check if an embedding is anomalous relative to background via z-score.
    pub fn is_anomalous(emb: &[f64], background: &[f64], z_threshold: f64) -> bool {
        if emb.is_empty() || background.is_empty() {
            return false;
        }

        let diff = VectorSpace::vec_sub(emb, background);
        let n = diff.len() as f64;
        let mean_diff: f64 = diff.iter().sum::<f64>() / n;
        let var: f64 = diff.iter().map(|x| (x - mean_diff).powi(2)).sum::<f64>() / n;
        let std_diff = var.sqrt() + EPSILON;

        for x in diff.iter() {
            let z = ((x - mean_diff) / std_diff).abs();
            if z > z_threshold {
                return true;
            }
        }
        false
    }

    /// Repair an embedding by damping extremes and optionally pulling toward background.
    pub fn repair(emb: &[f64], background: Option<&[f64]>, strength: f64) -> Vec<f64> {
        let mut repaired: Vec<f64> = emb
            .iter()
            .map(|x| {
                let correction = -strength * x.signum() * x.abs().min(0.05);
                x + correction
            })
            .collect();

        if let Some(bg) = background {
            for i in 0..repaired.len() {
                let bg_val = if i < bg.len() { bg[i] } else { 0.0 };
                repaired[i] = 0.7 * repaired[i] + 0.3 * bg_val;
            }
        }

        repaired
    }
}

// ===========================================================================
// ProjectionEngine — Dimensionality reduction via random projections
// ===========================================================================

#[derive(Debug)]
pub struct ProjectionEngine {
    pub dim: usize,
    pub micro_dim: usize,
    pub macro_dim: usize,
    pub high_dim: usize,
    micro_proj: Vec<Vec<f64>>, // micro_dim × dim
    macro_proj: Vec<Vec<f64>>, // macro_dim × micro_dim
    high_proj: Vec<Vec<f64>>,  // high_dim × dim
    micro_count: u64,
}

impl ProjectionEngine {
    pub fn new(dim: usize, micro_dim: usize, macro_dim: usize, high_dim: usize) -> Self {
        let mut rng = ProjRng::new(42);

        let micro_proj = (0..micro_dim)
            .map(|_| (0..dim).map(|_| rng.next_gaussian()).collect())
            .collect();
        let macro_proj = (0..macro_dim)
            .map(|_| (0..micro_dim).map(|_| rng.next_gaussian()).collect())
            .collect();
        let high_proj = (0..high_dim)
            .map(|_| (0..dim).map(|_| rng.next_gaussian()).collect())
            .collect();

        Self {
            dim,
            micro_dim,
            macro_dim,
            high_dim,
            micro_proj,
            macro_proj,
            high_proj,
            micro_count: 0,
        }
    }

    /// Project embedding from dim → micro_dim → macro_dim (with tanh nonlinearity).
    pub fn micro_to_macro(&mut self, emb: &[f64]) -> (Vec<f64>, ProjectionMeta) {
        // micro = micro_proj · emb
        let micro: Vec<f64> = self
            .micro_proj
            .iter()
            .map(|row| row.iter().zip(emb.iter()).map(|(a, b)| a * b).sum::<f64>())
            .collect();

        self.micro_count += 1;

        // macro = macro_proj · tanh(micro)
        let tanh_micro: Vec<f64> = micro.iter().map(|x| x.tanh()).collect();
        let macro_vec: Vec<f64> = self
            .macro_proj
            .iter()
            .map(|row| {
                row.iter()
                    .zip(tanh_micro.iter())
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
            })
            .collect();

        let normalized = VectorSpace::normalized(&macro_vec);
        let micro_norm = VectorSpace::norm(&micro);
        let macro_norm = VectorSpace::norm(&normalized);

        (
            normalized,
            ProjectionMeta {
                micro_norm,
                macro_norm,
            },
        )
    }

    /// Project embedding to high-dimensional space with tanh.
    pub fn high_dim_project(&self, emb: &[f64]) -> Vec<f64> {
        let high: Vec<f64> = self
            .high_proj
            .iter()
            .map(|row| {
                row.iter()
                    .zip(emb.iter())
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
                    .tanh()
            })
            .collect();
        VectorSpace::normalized(&high)
    }
}

#[derive(Debug, Clone)]
pub struct ProjectionMeta {
    pub micro_norm: f64,
    pub macro_norm: f64,
}

// ===========================================================================
// QdmaStore — In-memory HashMap-backed entity store with cosine search
// ===========================================================================

#[derive(Debug)]
pub struct QdmaStore {
    pub entities: HashMap<String, DreamEntity>,
    pub seeds: HashMap<String, DreamSeed>,
    dim: usize,
    projection_engine: Option<ProjectionEngine>,
    total_stores: u64,
    total_queries: u64,
}

impl QdmaStore {
    pub fn new(dim: usize) -> Self {
        let pe = ProjectionEngine::new(dim, dim / 4, dim / 8, dim * 2);
        Self {
            entities: HashMap::new(),
            seeds: HashMap::new(),
            dim,
            projection_engine: Some(pe),
            total_stores: 0,
            total_queries: 0,
        }
    }

    /// Store a DreamEntity. Returns entity ID.
    pub fn store(&mut self, entity: DreamEntity) -> String {
        let id = entity.id.clone();
        self.entities.insert(id.clone(), entity);
        self.total_stores += 1;
        id
    }

    /// Store a DreamEntity from raw embedding + shards.
    pub fn store_raw(&mut self, id: String, embedding: Vec<f64>, shards: Vec<String>) -> String {
        let entity = DreamEntity::new(id, embedding, shards);
        self.store(entity)
    }

    /// Cosine-similarity search: returns top-k (id, confidence, delta_E) as Holograms.
    pub fn query(&mut self, query_emb: &[f64], topk: usize) -> Vec<Hologram> {
        self.total_queries += 1;

        let mut scored: Vec<(String, f64)> = self
            .entities
            .iter()
            .filter(|(_, e)| !e.quarantined)
            .map(|(id, e)| {
                let sim = VectorSpace::cosine_sim(query_emb, &e.embedding);
                (id.clone(), sim)
            })
            .collect();

        // Sort descending by similarity
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(topk);

        scored
            .into_iter()
            .map(|(id, confidence)| {
                let entity = &self.entities[&id];
                let delta_e = (entity.score - confidence).abs();
                let toxic = DetoxSystem::toxicity_score(&entity.embedding, Some(query_emb));
                Hologram {
                    id,
                    embedding: entity.embedding.clone(),
                    confidence,
                    delta_e,
                    toxic_score: toxic,
                }
            })
            .collect()
    }

    /// Project an embedding through the internal projection engine.
    pub fn project(&mut self, emb: &[f64]) -> Option<(Vec<f64>, ProjectionMeta)> {
        self.projection_engine
            .as_mut()
            .map(|pe| pe.micro_to_macro(emb))
    }

    /// Get store statistics.
    pub fn stats(&self) -> StoreStats {
        let avg_score = if self.entities.is_empty() {
            0.0
        } else {
            self.entities.values().map(|e| e.score).sum::<f64>() / self.entities.len() as f64
        };

        StoreStats {
            entity_count: self.entities.len(),
            seed_count: self.seeds.len(),
            dim: self.dim,
            total_stores: self.total_stores,
            total_queries: self.total_queries,
            avg_score,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoreStats {
    pub entity_count: usize,
    pub seed_count: usize,
    pub dim: usize,
    pub total_stores: u64,
    pub total_queries: u64,
    pub avg_score: f64,
}

// ===========================================================================
// Helpers
// ===========================================================================

fn now_ts() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vec(dim: usize, val: f64) -> Vec<f64> {
        vec![val; dim]
    }

    fn varied_vec(dim: usize, seed: usize) -> Vec<f64> {
        (0..dim).map(|i| ((i + seed) as f64 * 0.1).sin()).collect()
    }

    #[test]
    fn test_cosine_sim_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = VectorSpace::cosine_sim(&v, &v);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "identical vecs → sim=1.0, got {}",
            sim
        );
    }

    #[test]
    fn test_cosine_sim_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = VectorSpace::cosine_sim(&a, &b);
        assert!(sim.abs() < 1e-6, "orthogonal vecs → sim≈0, got {}", sim);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        VectorSpace::normalize(&mut v);
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-6,
            "normalized norm should be 1.0, got {}",
            norm
        );
    }

    #[test]
    fn test_mean_vec() {
        let a = vec![2.0, 4.0];
        let b = vec![6.0, 8.0];
        let mean = VectorSpace::mean_vec(&[&a[..], &b[..]]).unwrap();
        assert!((mean[0] - 4.0).abs() < 1e-6);
        assert!((mean[1] - 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_vec_add_sub() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let sum = VectorSpace::vec_add(&a, &b);
        assert!((sum[0] - 5.0).abs() < 1e-6);
        let diff = VectorSpace::vec_sub(&a, &b);
        assert!((diff[0] - (-3.0)).abs() < 1e-6);
    }

    #[test]
    fn test_quantize_dequantize_roundtrip() {
        let vecs = vec![vec![0.0, 0.5, 1.0], vec![0.25, 0.75, 0.5]];
        let (qvecs, meta) = quantize_list(&vecs, 8);
        assert_eq!(qvecs.len(), 2);

        let deq = dequantize(&qvecs[0], &meta);
        // Roundtrip error should be small
        for (i, x) in deq.iter().enumerate() {
            assert!(
                (x - vecs[0][i]).abs() < 0.005,
                "roundtrip error too large: {} vs {}",
                x,
                vecs[0][i]
            );
        }
    }

    #[test]
    fn test_dream_entity_new() {
        let e = DreamEntity::new(
            "test-1".to_string(),
            vec![1.0, 2.0, 3.0],
            vec!["s1".to_string()],
        );
        assert_eq!(e.id, "test-1");
        assert_eq!(e.xi, 0.5);
        assert_eq!(e.score, 1.0);
        assert_eq!(e.trit, 0);
        assert!(!e.quarantined);
    }

    #[test]
    fn test_dream_entity_touch() {
        let mut e = DreamEntity::new("test-2".to_string(), vec![1.0], vec![]);
        let v0 = e.version;
        e.touch();
        assert_eq!(e.version, v0 + 1);
    }

    #[test]
    fn test_detox_zero_for_uniform() {
        // Uniform small values → low toxicity
        let emb = sample_vec(64, 0.01);
        let score = DetoxSystem::toxicity_score(&emb, None);
        assert!(score < 0.3, "uniform small should be non-toxic: {}", score);
    }

    #[test]
    fn test_detox_anomalous() {
        let bg = sample_vec(64, 0.0);
        let anomaly = {
            let mut v = sample_vec(64, 0.0);
            v[0] = 100.0; // extreme outlier
            v
        };
        assert!(
            DetoxSystem::is_anomalous(&anomaly, &bg, 3.0),
            "should detect extreme outlier"
        );
    }

    #[test]
    fn test_detox_repair_dampens() {
        let emb = vec![1.0, -1.0, 0.5];
        let repaired = DetoxSystem::repair(&emb, None, 0.4);
        // Repaired should have smaller absolute values
        for (orig, rep) in emb.iter().zip(repaired.iter()) {
            assert!(rep.abs() <= orig.abs() + EPSILON, "repair should dampen");
        }
    }

    #[test]
    fn test_projection_engine_dimensions() {
        let mut pe = ProjectionEngine::new(384, 96, 48, 768);
        let emb = varied_vec(384, 0);
        let (macro_vec, meta) = pe.micro_to_macro(&emb);
        assert_eq!(macro_vec.len(), 48, "macro should be macro_dim=48");
        assert!(meta.micro_norm > 0.0);

        let high = pe.high_dim_project(&emb);
        assert_eq!(high.len(), 768, "high should be high_dim=768");
    }

    #[test]
    fn test_projection_normalized() {
        let mut pe = ProjectionEngine::new(64, 16, 8, 128);
        let emb = varied_vec(64, 42);
        let (macro_vec, _) = pe.micro_to_macro(&emb);
        let norm = VectorSpace::norm(&macro_vec);
        assert!(
            (norm - 1.0).abs() < 0.01,
            "macro output should be normalized: {}",
            norm
        );
    }

    #[test]
    fn test_store_and_query() {
        let mut store = QdmaStore::new(4);
        store.store_raw("a".to_string(), vec![1.0, 0.0, 0.0, 0.0], vec![]);
        store.store_raw("b".to_string(), vec![0.0, 1.0, 0.0, 0.0], vec![]);
        store.store_raw("c".to_string(), vec![0.9, 0.1, 0.0, 0.0], vec![]);

        let results = store.query(&[1.0, 0.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        // "a" should be the top hit (exact match)
        assert_eq!(results[0].id, "a");
        assert!(results[0].confidence > 0.99);
    }

    #[test]
    fn test_store_stats() {
        let mut store = QdmaStore::new(4);
        store.store_raw("x".to_string(), vec![1.0, 0.0, 0.0, 0.0], vec![]);
        store.store_raw("y".to_string(), vec![0.0, 1.0, 0.0, 0.0], vec![]);

        let s = store.stats();
        assert_eq!(s.entity_count, 2);
        assert_eq!(s.total_stores, 2);
        assert_eq!(s.dim, 4);
    }
}
