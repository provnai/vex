use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A semantic rule extracted from a batch of experiments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRule {
    /// Unique ID of the rule
    pub id: Uuid,
    /// Human-readable description of the rule
    pub rule_description: String,
    /// List of traits this rule affects (e.g., ["aggression", "analysis_depth"])
    pub affected_traits: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Number of experiments that supported this rule
    pub source_experiments_count: usize,
    /// When this rule was created
    pub created_at: DateTime<Utc>,
}

impl OptimizationRule {
    pub fn new(description: String, traits: Vec<String>, confidence: f64, count: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            rule_description: description,
            affected_traits: traits,
            confidence: confidence.clamp(0.0, 1.0),
            source_experiments_count: count,
            created_at: Utc::now(),
        }
    }
}
