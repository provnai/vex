//! Observability - Metrics, tracing, and cost tracking

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetrics {
    pub request_id: String,
    pub timestamp: i64,
    pub model_used: String,
    pub routing_strategy: String,
    pub complexity_score: f64,
    pub tokens_input: u32,
    pub tokens_output: u32,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub first_token_ms: Option<u64>,
    pub cache_hit: bool,
    pub cache_similarity: Option<f32>,
    pub compression_ratio: Option<f64>,
    pub guardrails_passed: bool,
    pub error: Option<String>,
}

impl RequestMetrics {
    pub fn new(request_id: String, model_used: String, routing_strategy: String) -> Self {
        Self {
            request_id,
            timestamp: Utc::now().timestamp(),
            model_used,
            routing_strategy,
            complexity_score: 0.0,
            tokens_input: 0,
            tokens_output: 0,
            cost_usd: 0.0,
            latency_ms: 0,
            first_token_ms: None,
            cache_hit: false,
            cache_similarity: None,
            compression_ratio: None,
            guardrails_passed: true,
            error: None,
        }
    }
}

#[derive(Debug)]
pub struct Observability {
    metrics: Arc<RwLock<Vec<RequestMetrics>>>,
    daily_stats: Arc<RwLock<DailyStats>>,
    max_metrics_stored: usize,
}

impl Observability {
    pub fn new(max_metrics_stored: usize) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            daily_stats: Arc::new(RwLock::new(DailyStats::new())),
            max_metrics_stored,
        }
    }

    pub fn record(&self, metric: RequestMetrics) {
        let mut metrics = self.metrics.write();

        if metrics.len() >= self.max_metrics_stored {
            metrics.remove(0);
        }

        metrics.push(metric.clone());

        let mut daily = self.daily_stats.write();
        daily.record(&metric);
    }

    pub fn get_metrics(&self, limit: usize) -> Vec<RequestMetrics> {
        let metrics = self.metrics.read();
        metrics.iter().rev().take(limit).cloned().collect()
    }

    pub fn get_summary(&self) -> ObservabilitySummary {
        let metrics = self.metrics.read();

        if metrics.is_empty() {
            return ObservabilitySummary::default();
        }

        let total_requests = metrics.len();
        let total_cost: f64 = metrics.iter().map(|m| m.cost_usd).sum();
        let total_tokens_input: u64 = metrics.iter().map(|m| m.tokens_input as u64).sum();
        let total_tokens_output: u64 = metrics.iter().map(|m| m.tokens_output as u64).sum();
        let cache_hits = metrics.iter().filter(|m| m.cache_hit).count();
        let errors = metrics.iter().filter(|m| m.error.is_some()).count();

        let mut latencies: Vec<u64> = metrics.iter().map(|m| m.latency_ms).collect();
        latencies.sort();

        let avg_latency = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
        let p50_latency = latencies[latencies.len() / 2];
        let p95_latency = latencies[(latencies.len() * 95) / 100];
        let p99_latency = latencies[(latencies.len() * 99) / 100];

        ObservabilitySummary {
            total_requests,
            total_cost_usd: total_cost,
            total_tokens_input: total_tokens_input as u32,
            total_tokens_output: total_tokens_output as u32,
            avg_cost_per_request: total_cost / total_requests as f64,
            avg_latency_ms: avg_latency,
            p50_latency_ms: p50_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            cache_hit_rate: cache_hits as f64 / total_requests as f64,
            error_rate: errors as f64 / total_requests as f64,
        }
    }

    pub fn get_cost_by_model(&self) -> HashMap<String, f64> {
        let metrics = self.metrics.read();
        let mut costs: HashMap<String, f64> = HashMap::new();

        for m in metrics.iter() {
            *costs.entry(m.model_used.clone()).or_insert(0.0) += m.cost_usd;
        }

        costs
    }

    pub fn get_savings(&self) -> SavingsReport {
        let metrics = self.metrics.read();

        let baseline_cost: f64 = metrics
            .iter()
            .map(|m| {
                m.tokens_input as f64 * 15.0 / 1_000_000.0
                    + m.tokens_output as f64 * 15.0 / 1_000_000.0
            })
            .sum();

        let actual_cost: f64 = metrics.iter().map(|m| m.cost_usd).sum();

        let routing_savings = baseline_cost * 0.6;
        let cache_savings = baseline_cost * metrics.iter().filter(|m| m.cache_hit).count() as f64
            / metrics.len().max(1) as f64;
        let compression_savings = baseline_cost
            * metrics
                .iter()
                .filter_map(|m| m.compression_ratio)
                .sum::<f64>()
            / metrics.len().max(1) as f64;

        SavingsReport {
            baseline_cost,
            actual_cost,
            total_savings: baseline_cost - actual_cost,
            savings_percentage: if baseline_cost > 0.0 {
                (baseline_cost - actual_cost) / baseline_cost * 100.0
            } else {
                0.0
            },
            routing_savings,
            cache_savings,
            compression_savings,
        }
    }

    pub fn clear(&self) {
        let mut metrics = self.metrics.write();
        metrics.clear();

        let mut daily = self.daily_stats.write();
        *daily = DailyStats::new();
    }
}

impl Default for Observability {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ObservabilitySummary {
    pub total_requests: usize,
    pub total_cost_usd: f64,
    pub total_tokens_input: u32,
    pub total_tokens_output: u32,
    pub avg_cost_per_request: f64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub cache_hit_rate: f64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct SavingsReport {
    pub baseline_cost: f64,
    pub actual_cost: f64,
    pub total_savings: f64,
    pub savings_percentage: f64,
    pub routing_savings: f64,
    pub cache_savings: f64,
    pub compression_savings: f64,
}

#[derive(Debug)]
struct DailyStats {
    date: String,
    total_requests: u64,
    total_cost: f64,
    total_tokens: u64,
    errors: u64,
}

impl DailyStats {
    fn new() -> Self {
        Self {
            date: Utc::now().format("%Y-%m-%d").to_string(),
            total_requests: 0,
            total_cost: 0.0,
            total_tokens: 0,
            errors: 0,
        }
    }

    fn record(&mut self, metric: &RequestMetrics) {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        if self.date != today {
            *self = Self::new();
            self.date = today;
        }

        self.total_requests += 1;
        self.total_cost += metric.cost_usd;
        self.total_tokens += (metric.tokens_input + metric.tokens_output) as u64;

        if metric.error.is_some() {
            self.errors += 1;
        }
    }
}

pub fn calculate_cost(
    tokens: u32,
    input_cost_per_million: f64,
    output_cost_per_million: f64,
    is_output: bool,
) -> f64 {
    if is_output {
        tokens as f64 * output_cost_per_million / 1_000_000.0
    } else {
        tokens as f64 * input_cost_per_million / 1_000_000.0
    }
}
