//! Metering engine — tracks LLM cost and enforces spending quotas.
//!
//! Architecture informed by OpenFang (MIT/Apache-2.0).
//! Ark-native implementation: in-memory store, synchronous,
//! zero external dependencies.

use serde::Serialize;
use std::sync::Mutex;

// ── Inlined Types ──────────────────────────────────────────────────

/// Agent identifier.
pub type AgentId = String;

/// A single usage record.
#[derive(Debug, Clone, Serialize)]
pub struct UsageRecord {
    pub agent_id: AgentId,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub tool_calls: u64,
    /// Unix epoch seconds when this record was created.
    pub timestamp: u64,
}

/// Per-agent spending quotas.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceQuota {
    pub max_cost_per_hour_usd: f64,
    pub max_cost_per_day_usd: f64,
    pub max_cost_per_month_usd: f64,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_cost_per_hour_usd: 0.0,
            max_cost_per_day_usd: 0.0,
            max_cost_per_month_usd: 0.0,
        }
    }
}

/// Global budget configuration.
#[derive(Debug, Clone, Serialize)]
pub struct BudgetConfig {
    pub max_hourly_usd: f64,
    pub max_daily_usd: f64,
    pub max_monthly_usd: f64,
    /// Alert when spend exceeds this fraction of limit (0.0 - 1.0).
    pub alert_threshold: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_hourly_usd: 0.0,
            max_daily_usd: 0.0,
            max_monthly_usd: 0.0,
            alert_threshold: 0.8,
        }
    }
}

/// Usage summary for reporting.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UsageSummary {
    pub call_count: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub total_tool_calls: u64,
}

/// Per-model usage breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct ModelUsage {
    pub model: String,
    pub call_count: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
}

/// Budget status snapshot — current spend vs limits for all time windows.
#[derive(Debug, Clone, Serialize)]
pub struct BudgetStatus {
    pub hourly_spend: f64,
    pub hourly_limit: f64,
    pub hourly_pct: f64,
    pub daily_spend: f64,
    pub daily_limit: f64,
    pub daily_pct: f64,
    pub monthly_spend: f64,
    pub monthly_limit: f64,
    pub monthly_pct: f64,
    pub alert_threshold: f64,
}

/// Metering error.
#[derive(Debug, Clone)]
pub enum MeteringError {
    /// Agent or global budget exceeded.
    QuotaExceeded(String),
}

impl std::fmt::Display for MeteringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeteringError::QuotaExceeded(msg) => write!(f, "QuotaExceeded: {}", msg),
        }
    }
}

impl std::error::Error for MeteringError {}

// ── Time helpers ───────────────────────────────────────────────────

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

const SECS_PER_HOUR: u64 = 3600;
const SECS_PER_DAY: u64 = 86_400;
const SECS_PER_MONTH: u64 = 30 * SECS_PER_DAY; // 30-day approximation

// ── Metering Engine ────────────────────────────────────────────────

/// The metering engine tracks usage cost and enforces quota limits.
///
/// Uses an in-memory store. For production persistence, wrap with
/// a serialization layer (e.g., serde_json to disk on shutdown).
pub struct MeteringEngine {
    records: Mutex<Vec<UsageRecord>>,
}

impl MeteringEngine {
    /// Create a new metering engine.
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    /// Record a usage event.
    pub fn record(&self, record: UsageRecord) {
        self.records.lock().unwrap().push(record);
    }

    /// Query total cost for an agent in the last hour.
    fn query_agent_hourly(&self, agent_id: &str) -> f64 {
        let cutoff = now_epoch().saturating_sub(SECS_PER_HOUR);
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.agent_id == agent_id && r.timestamp >= cutoff)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// Query total cost for an agent today (last 24h).
    fn query_agent_daily(&self, agent_id: &str) -> f64 {
        let cutoff = now_epoch().saturating_sub(SECS_PER_DAY);
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.agent_id == agent_id && r.timestamp >= cutoff)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// Query total cost for an agent this month (last 30 days).
    fn query_agent_monthly(&self, agent_id: &str) -> f64 {
        let cutoff = now_epoch().saturating_sub(SECS_PER_MONTH);
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.agent_id == agent_id && r.timestamp >= cutoff)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// Query global hourly spend (all agents).
    fn query_global_hourly(&self) -> f64 {
        let cutoff = now_epoch().saturating_sub(SECS_PER_HOUR);
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.timestamp >= cutoff)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// Query global daily spend (all agents).
    fn query_global_daily(&self) -> f64 {
        let cutoff = now_epoch().saturating_sub(SECS_PER_DAY);
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.timestamp >= cutoff)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// Query global monthly spend (all agents).
    fn query_global_monthly(&self) -> f64 {
        let cutoff = now_epoch().saturating_sub(SECS_PER_MONTH);
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.timestamp >= cutoff)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// Check if an agent is within its spending quotas.
    /// Returns Ok(()) if under all quotas, or QuotaExceeded if over any.
    pub fn check_quota(&self, agent_id: &str, quota: &ResourceQuota) -> Result<(), MeteringError> {
        if quota.max_cost_per_hour_usd > 0.0 {
            let cost = self.query_agent_hourly(agent_id);
            if cost >= quota.max_cost_per_hour_usd {
                return Err(MeteringError::QuotaExceeded(format!(
                    "Agent {} exceeded hourly cost quota: ${:.4} / ${:.4}",
                    agent_id, cost, quota.max_cost_per_hour_usd
                )));
            }
        }
        if quota.max_cost_per_day_usd > 0.0 {
            let cost = self.query_agent_daily(agent_id);
            if cost >= quota.max_cost_per_day_usd {
                return Err(MeteringError::QuotaExceeded(format!(
                    "Agent {} exceeded daily cost quota: ${:.4} / ${:.4}",
                    agent_id, cost, quota.max_cost_per_day_usd
                )));
            }
        }
        if quota.max_cost_per_month_usd > 0.0 {
            let cost = self.query_agent_monthly(agent_id);
            if cost >= quota.max_cost_per_month_usd {
                return Err(MeteringError::QuotaExceeded(format!(
                    "Agent {} exceeded monthly cost quota: ${:.4} / ${:.4}",
                    agent_id, cost, quota.max_cost_per_month_usd
                )));
            }
        }
        Ok(())
    }

    /// Check global budget limits (across all agents).
    pub fn check_global_budget(&self, budget: &BudgetConfig) -> Result<(), MeteringError> {
        if budget.max_hourly_usd > 0.0 {
            let cost = self.query_global_hourly();
            if cost >= budget.max_hourly_usd {
                return Err(MeteringError::QuotaExceeded(format!(
                    "Global hourly budget exceeded: ${:.4} / ${:.4}",
                    cost, budget.max_hourly_usd
                )));
            }
        }
        if budget.max_daily_usd > 0.0 {
            let cost = self.query_global_daily();
            if cost >= budget.max_daily_usd {
                return Err(MeteringError::QuotaExceeded(format!(
                    "Global daily budget exceeded: ${:.4} / ${:.4}",
                    cost, budget.max_daily_usd
                )));
            }
        }
        if budget.max_monthly_usd > 0.0 {
            let cost = self.query_global_monthly();
            if cost >= budget.max_monthly_usd {
                return Err(MeteringError::QuotaExceeded(format!(
                    "Global monthly budget exceeded: ${:.4} / ${:.4}",
                    cost, budget.max_monthly_usd
                )));
            }
        }
        Ok(())
    }

    /// Get budget status — current spend vs limits for all time windows.
    pub fn budget_status(&self, budget: &BudgetConfig) -> BudgetStatus {
        let hourly = self.query_global_hourly();
        let daily = self.query_global_daily();
        let monthly = self.query_global_monthly();

        BudgetStatus {
            hourly_spend: hourly,
            hourly_limit: budget.max_hourly_usd,
            hourly_pct: if budget.max_hourly_usd > 0.0 {
                hourly / budget.max_hourly_usd
            } else {
                0.0
            },
            daily_spend: daily,
            daily_limit: budget.max_daily_usd,
            daily_pct: if budget.max_daily_usd > 0.0 {
                daily / budget.max_daily_usd
            } else {
                0.0
            },
            monthly_spend: monthly,
            monthly_limit: budget.max_monthly_usd,
            monthly_pct: if budget.max_monthly_usd > 0.0 {
                monthly / budget.max_monthly_usd
            } else {
                0.0
            },
            alert_threshold: budget.alert_threshold,
        }
    }

    /// Get a usage summary, optionally filtered by agent.
    pub fn get_summary(&self, agent_id: Option<&str>) -> UsageSummary {
        let records = self.records.lock().unwrap();
        let iter: Box<dyn Iterator<Item = &UsageRecord> + '_> = match agent_id {
            Some(id) => Box::new(records.iter().filter(move |r| r.agent_id == id)),
            None => Box::new(records.iter()),
        };

        let mut summary = UsageSummary::default();
        for r in iter {
            summary.call_count += 1;
            summary.total_input_tokens += r.input_tokens;
            summary.total_output_tokens += r.output_tokens;
            summary.total_cost_usd += r.cost_usd;
            summary.total_tool_calls += r.tool_calls;
        }
        summary
    }

    /// Get usage grouped by model.
    pub fn get_by_model(&self) -> Vec<ModelUsage> {
        let records = self.records.lock().unwrap();
        let mut map: std::collections::HashMap<String, ModelUsage> =
            std::collections::HashMap::new();

        for r in records.iter() {
            let entry = map.entry(r.model.clone()).or_insert_with(|| ModelUsage {
                model: r.model.clone(),
                call_count: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cost_usd: 0.0,
            });
            entry.call_count += 1;
            entry.total_input_tokens += r.input_tokens;
            entry.total_output_tokens += r.output_tokens;
            entry.total_cost_usd += r.cost_usd;
        }

        map.into_values().collect()
    }

    /// Remove records older than `max_age_secs`.
    pub fn cleanup_older_than(&self, max_age_secs: u64) -> usize {
        let cutoff = now_epoch().saturating_sub(max_age_secs);
        let mut records = self.records.lock().unwrap();
        let before = records.len();
        records.retain(|r| r.timestamp >= cutoff);
        before - records.len()
    }

    /// Total number of records stored.
    pub fn record_count(&self) -> usize {
        self.records.lock().unwrap().len()
    }
}

impl Default for MeteringEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Cost Estimation ────────────────────────────────────────────────

/// Estimate the cost of an LLM call based on model name and token counts.
///
/// Pricing table covers 40+ models across Anthropic, OpenAI, Google,
/// DeepSeek, Meta, xAI, Mistral, Cohere, Perplexity, Qwen, and more.
/// Prices are per million tokens.
///
/// Returns cost in USD.
pub fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    let model_lower = model.to_lowercase();
    let (input_per_m, output_per_m) = estimate_cost_rates(&model_lower);
    let input_cost = (input_tokens as f64 / 1_000_000.0) * input_per_m;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * output_per_m;
    input_cost + output_cost
}

/// Returns (input_per_million, output_per_million) pricing for a model.
///
/// Order matters: more specific patterns must come before generic ones
/// (e.g. "gpt-4o-mini" before "gpt-4o", "gpt-4.1-mini" before "gpt-4.1").
fn estimate_cost_rates(model: &str) -> (f64, f64) {
    // ── Anthropic ──────────────────────────────────────────────
    if model.contains("haiku") {
        return (0.25, 1.25);
    }
    if model.contains("opus-4-6") || model.contains("claude-opus-4-6") {
        return (5.0, 25.0);
    }
    if model.contains("opus") {
        return (15.0, 75.0);
    }
    if model.contains("sonnet-4-6") || model.contains("claude-sonnet-4-6") {
        return (3.0, 15.0);
    }
    if model.contains("sonnet") {
        return (3.0, 15.0);
    }

    // ── OpenAI ─────────────────────────────────────────────────
    if model.contains("gpt-5.2-pro") {
        return (1.75, 14.0);
    }
    if model.contains("gpt-5.2") {
        return (1.75, 14.0);
    }
    if model.contains("gpt-5.1") {
        return (1.25, 10.0);
    }
    if model.contains("gpt-5-nano") {
        return (0.05, 0.40);
    }
    if model.contains("gpt-5-mini") {
        return (0.25, 2.0);
    }
    if model.contains("gpt-5") {
        return (1.25, 10.0);
    }
    if model.contains("gpt-4o-mini") {
        return (0.15, 0.60);
    }
    if model.contains("gpt-4o") {
        return (2.50, 10.0);
    }
    if model.contains("gpt-4.1-nano") {
        return (0.10, 0.40);
    }
    if model.contains("gpt-4.1-mini") {
        return (0.40, 1.60);
    }
    if model.contains("gpt-4.1") {
        return (2.00, 8.00);
    }
    if model.contains("o4-mini") {
        return (1.10, 4.40);
    }
    if model.contains("o3-mini") {
        return (1.10, 4.40);
    }
    if model.contains("o3") {
        return (2.00, 8.00);
    }
    if model.contains("gpt-4") {
        return (2.50, 10.0);
    }

    // ── Google Gemini ──────────────────────────────────────────
    if model.contains("gemini-3.1") {
        return (2.50, 15.0);
    }
    if model.contains("gemini-3") {
        return (0.50, 3.0);
    }
    if model.contains("gemini-2.5-flash-lite") {
        return (0.04, 0.15);
    }
    if model.contains("gemini-2.5-pro") {
        return (1.25, 10.0);
    }
    if model.contains("gemini-2.5-flash") {
        return (0.15, 0.60);
    }
    if model.contains("gemini-2.0-flash") || model.contains("gemini-flash") {
        return (0.10, 0.40);
    }
    if model.contains("gemini") {
        return (0.15, 0.60);
    }

    // ── DeepSeek ───────────────────────────────────────────────
    if model.contains("deepseek-reasoner") || model.contains("deepseek-r1") {
        return (0.55, 2.19);
    }
    if model.contains("deepseek") {
        return (0.27, 1.10);
    }

    // ── Cerebras (ultra-fast) ──────────────────────────────────
    if model.contains("cerebras") {
        return (0.06, 0.06);
    }

    // ── SambaNova ──────────────────────────────────────────────
    if model.contains("sambanova") {
        return (0.06, 0.06);
    }

    // ── Replicate ──────────────────────────────────────────────
    if model.contains("replicate") {
        return (0.40, 0.40);
    }

    // ── Open-source (Groq, Together, etc.) ─────────────────────
    if model.contains("llama-4-maverick") {
        return (0.50, 0.77);
    }
    if model.contains("llama-4-scout") {
        return (0.11, 0.34);
    }
    if model.contains("llama") || model.contains("mixtral") {
        return (0.05, 0.10);
    }

    // ── Qwen (Alibaba) ────────────────────────────────────────
    if model.contains("qwen-max") {
        return (4.00, 12.00);
    }
    if model.contains("qwen-vl") {
        return (1.50, 4.50);
    }
    if model.contains("qwen-plus") {
        return (0.80, 2.00);
    }
    if model.contains("qwen-turbo") {
        return (0.30, 0.60);
    }
    if model.contains("qwen") {
        return (0.20, 0.60);
    }

    // ── MiniMax ────────────────────────────────────────────────
    if model.contains("minimax") {
        return (1.00, 3.00);
    }

    // ── Zhipu / GLM ───────────────────────────────────────────
    if model.contains("glm-4-flash") {
        return (0.10, 0.10);
    }
    if model.contains("glm") {
        return (1.50, 5.00);
    }
    if model.contains("codegeex") {
        return (0.10, 0.10);
    }

    // ── Moonshot / Kimi ───────────────────────────────────────
    if model.contains("moonshot") || model.contains("kimi") {
        return (0.80, 0.80);
    }

    // ── Baidu ERNIE ───────────────────────────────────────────
    if model.contains("ernie") {
        return (2.00, 6.00);
    }

    // ── AWS Bedrock ───────────────────────────────────────────
    if model.contains("nova-pro") {
        return (0.80, 3.20);
    }
    if model.contains("nova-lite") {
        return (0.06, 0.24);
    }

    // ── Mistral ───────────────────────────────────────────────
    if model.contains("mistral-large") {
        return (2.00, 6.00);
    }
    if model.contains("mistral-small") || model.contains("mistral") {
        return (0.10, 0.30);
    }

    // ── Cohere ────────────────────────────────────────────────
    if model.contains("command-r-plus") {
        return (2.50, 10.0);
    }
    if model.contains("command-r") {
        return (0.15, 0.60);
    }

    // ── Perplexity ────────────────────────────────────────────
    if model.contains("sonar-pro") {
        return (3.0, 15.0);
    }
    if model.contains("sonar") {
        return (1.0, 5.0);
    }

    // ── xAI / Grok ────────────────────────────────────────────
    if model.contains("grok-4.1") {
        return (0.20, 0.50);
    }
    if model.contains("grok-4") {
        return (3.0, 15.0);
    }
    if model.contains("grok-3-mini") || model.contains("grok-2-mini") || model.contains("grok-mini")
    {
        return (0.30, 0.50);
    }
    if model.contains("grok-3") {
        return (3.0, 15.0);
    }
    if model.contains("grok") {
        return (2.0, 10.0);
    }

    // ── AI21 / Jamba ──────────────────────────────────────────
    if model.contains("jamba") {
        return (2.0, 8.0);
    }

    // ── Default (conservative) ────────────────────────────────
    (1.0, 3.0)
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(agent_id: &str, model: &str, cost: f64) -> UsageRecord {
        UsageRecord {
            agent_id: agent_id.to_string(),
            model: model.to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: cost,
            tool_calls: 1,
            timestamp: now_epoch(),
        }
    }

    #[test]
    fn test_record_and_check_quota_under() {
        let engine = MeteringEngine::new();
        let quota = ResourceQuota {
            max_cost_per_hour_usd: 1.0,
            ..Default::default()
        };
        engine.record(make_record("agent-1", "haiku", 0.001));
        assert!(engine.check_quota("agent-1", &quota).is_ok());
    }

    #[test]
    fn test_check_quota_exceeded() {
        let engine = MeteringEngine::new();
        let quota = ResourceQuota {
            max_cost_per_hour_usd: 0.01,
            ..Default::default()
        };
        engine.record(make_record("agent-1", "sonnet", 0.05));
        let result = engine.check_quota("agent-1", &quota);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("exceeded hourly cost quota"));
    }

    #[test]
    fn test_zero_limit_skips_enforcement() {
        let engine = MeteringEngine::new();
        let quota = ResourceQuota::default(); // all zeros
        engine.record(make_record("agent-1", "opus", 100.0));
        assert!(engine.check_quota("agent-1", &quota).is_ok());
    }

    #[test]
    fn test_daily_quota() {
        let engine = MeteringEngine::new();
        let quota = ResourceQuota {
            max_cost_per_day_usd: 0.10,
            ..Default::default()
        };
        engine.record(make_record("agent-1", "sonnet", 0.15));
        let result = engine.check_quota("agent-1", &quota);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("daily"));
    }

    #[test]
    fn test_monthly_quota() {
        let engine = MeteringEngine::new();
        let quota = ResourceQuota {
            max_cost_per_month_usd: 5.0,
            ..Default::default()
        };
        engine.record(make_record("agent-1", "opus", 6.0));
        let result = engine.check_quota("agent-1", &quota);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("monthly"));
    }

    #[test]
    fn test_global_budget_exceeded() {
        let engine = MeteringEngine::new();
        let budget = BudgetConfig {
            max_hourly_usd: 1.0,
            ..Default::default()
        };
        engine.record(make_record("agent-1", "opus", 0.6));
        engine.record(make_record("agent-2", "sonnet", 0.5));
        let result = engine.check_global_budget(&budget);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Global hourly"));
    }

    #[test]
    fn test_global_budget_ok() {
        let engine = MeteringEngine::new();
        let budget = BudgetConfig {
            max_hourly_usd: 10.0,
            ..Default::default()
        };
        engine.record(make_record("agent-1", "haiku", 0.001));
        assert!(engine.check_global_budget(&budget).is_ok());
    }

    #[test]
    fn test_budget_status() {
        let engine = MeteringEngine::new();
        let budget = BudgetConfig {
            max_hourly_usd: 10.0,
            max_daily_usd: 100.0,
            max_monthly_usd: 1000.0,
            alert_threshold: 0.8,
        };
        engine.record(make_record("agent-1", "sonnet", 5.0));
        let status = engine.budget_status(&budget);
        assert!((status.hourly_spend - 5.0).abs() < 0.01);
        assert!((status.hourly_pct - 0.5).abs() < 0.01);
        assert_eq!(status.alert_threshold, 0.8);
    }

    #[test]
    fn test_get_summary_all() {
        let engine = MeteringEngine::new();
        engine.record(make_record("a", "haiku", 0.01));
        engine.record(make_record("b", "sonnet", 0.05));
        let s = engine.get_summary(None);
        assert_eq!(s.call_count, 2);
    }

    #[test]
    fn test_get_summary_filtered() {
        let engine = MeteringEngine::new();
        engine.record(make_record("a", "haiku", 0.01));
        engine.record(make_record("b", "sonnet", 0.05));
        let s = engine.get_summary(Some("a"));
        assert_eq!(s.call_count, 1);
    }

    #[test]
    fn test_get_by_model() {
        let engine = MeteringEngine::new();
        engine.record(make_record("a", "haiku", 0.01));
        engine.record(make_record("b", "haiku", 0.02));
        engine.record(make_record("a", "sonnet", 0.10));
        let models = engine.get_by_model();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_record_count() {
        let engine = MeteringEngine::new();
        assert_eq!(engine.record_count(), 0);
        engine.record(make_record("a", "haiku", 0.01));
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn test_per_agent_isolation() {
        let engine = MeteringEngine::new();
        let quota = ResourceQuota {
            max_cost_per_hour_usd: 0.05,
            ..Default::default()
        };
        engine.record(make_record("agent-1", "opus", 0.10));
        // agent-2 should be fine — different agent
        assert!(engine.check_quota("agent-2", &quota).is_ok());
        // agent-1 should exceed
        assert!(engine.check_quota("agent-1", &quota).is_err());
    }

    // ── estimate_cost tests ────────────────────────────────────

    #[test]
    fn test_estimate_cost_haiku() {
        let cost = estimate_cost("claude-haiku-4-5-20251001", 1_000_000, 1_000_000);
        assert!((cost - 1.50).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_sonnet() {
        let cost = estimate_cost("claude-sonnet-4-20250514", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_opus() {
        let cost = estimate_cost("claude-opus-4-20250514", 1_000_000, 1_000_000);
        assert!((cost - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gpt4o() {
        let cost = estimate_cost("gpt-4o-2024-11-20", 1_000_000, 1_000_000);
        assert!((cost - 12.50).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gpt4o_mini() {
        let cost = estimate_cost("gpt-4o-mini", 1_000_000, 1_000_000);
        assert!((cost - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gemini_25_pro() {
        let cost = estimate_cost("gemini-2.5-pro", 1_000_000, 1_000_000);
        assert!((cost - 11.25).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_deepseek_chat() {
        let cost = estimate_cost("deepseek-chat", 1_000_000, 1_000_000);
        assert!((cost - 1.37).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_llama() {
        let cost = estimate_cost("llama-3.3-70b-versatile", 1_000_000, 1_000_000);
        assert!((cost - 0.15).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_grok() {
        let cost = estimate_cost("grok-2", 1_000_000, 1_000_000);
        assert!((cost - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_unknown() {
        let cost = estimate_cost("my-custom-model", 1_000_000, 1_000_000);
        assert!((cost - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_cerebras() {
        let cost = estimate_cost("cerebras/llama3.3-70b", 1_000_000, 1_000_000);
        assert!((cost - 0.12).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_qwen_max() {
        let cost = estimate_cost("qwen-max", 1_000_000, 1_000_000);
        assert!((cost - 16.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_sonar_pro() {
        let cost = estimate_cost("sonar-pro", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_jamba() {
        let cost = estimate_cost("jamba-1.5-large", 1_000_000, 1_000_000);
        assert!((cost - 10.0).abs() < 0.01);
    }
}
