//! Multi-dimensional fitness evaluation
//!
//! Provides rich fitness scoring beyond simple confidence values.
//! Evaluates task completion, accuracy, coherence, efficiency, and calibration.

use async_trait::async_trait;
use std::collections::HashMap;

/// Result of fitness evaluation with multiple metrics
#[derive(Debug, Clone, Default)]
pub struct FitnessReport {
    /// Overall fitness score (0.0-1.0)
    pub overall: f64,
    /// Individual metric scores
    pub metrics: HashMap<String, f64>,
}

impl FitnessReport {
    /// Create a simple report with just an overall score
    pub fn simple(overall: f64) -> Self {
        Self {
            overall: overall.clamp(0.0, 1.0),
            metrics: HashMap::new(),
        }
    }

    /// Create from individual metrics with weights
    ///
    /// # Example
    /// ```
    /// use vex_core::fitness::FitnessReport;
    /// use std::collections::HashMap;
    ///
    /// let mut metrics = HashMap::new();
    /// metrics.insert("accuracy".to_string(), 0.9);
    /// metrics.insert("coherence".to_string(), 0.8);
    ///
    /// let mut weights = HashMap::new();
    /// weights.insert("accuracy".to_string(), 0.6);
    /// weights.insert("coherence".to_string(), 0.4);
    ///
    /// let report = FitnessReport::from_weighted(metrics, &weights);
    /// assert!((report.overall - 0.86).abs() < 0.01);
    /// ```
    pub fn from_weighted(metrics: HashMap<String, f64>, weights: &HashMap<String, f64>) -> Self {
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for (name, score) in &metrics {
            let weight = weights.get(name).copied().unwrap_or(1.0);
            weighted_sum += score * weight;
            total_weight += weight;
        }

        let overall = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.5
        };

        Self {
            overall: overall.clamp(0.0, 1.0),
            metrics,
        }
    }

    /// Add a metric to the report
    pub fn add_metric(&mut self, name: &str, score: f64) {
        self.metrics.insert(name.to_string(), score.clamp(0.0, 1.0));
    }

    /// Recalculate overall from metrics with equal weights
    pub fn recalculate_overall(&mut self) {
        if self.metrics.is_empty() {
            return;
        }
        let sum: f64 = self.metrics.values().sum();
        self.overall = sum / self.metrics.len() as f64;
    }
}

/// Context for fitness evaluation
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    /// The original task/prompt
    pub task: String,
    /// Expected outcome (if known)
    pub expected_outcome: Option<String>,
    /// Additional context
    pub metadata: HashMap<String, String>,
}

impl EvaluationContext {
    /// Create new context with task
    pub fn new(task: &str) -> Self {
        Self {
            task: task.to_string(),
            expected_outcome: None,
            metadata: HashMap::new(),
        }
    }

    /// Add expected outcome
    pub fn with_expected(mut self, expected: &str) -> Self {
        self.expected_outcome = Some(expected.to_string());
        self
    }
}

/// Trait for fitness evaluators
///
/// Implementations can use LLM-as-judge, heuristics, or other methods
/// to evaluate response quality.
#[async_trait]
pub trait FitnessEvaluator: Send + Sync {
    /// Evaluate agent response and return fitness report
    async fn evaluate(
        &self,
        response: &str,
        context: &EvaluationContext,
    ) -> FitnessReport;
}

/// Default metric weights (sum to 1.0)
pub fn default_weights() -> HashMap<String, f64> {
    let mut weights = HashMap::new();
    weights.insert("task_completion".to_string(), 0.30);
    weights.insert("factual_accuracy".to_string(), 0.25);
    weights.insert("coherence".to_string(), 0.15);
    weights.insert("efficiency".to_string(), 0.15);
    weights.insert("confidence_calibration".to_string(), 0.15);
    weights
}

/// Simple heuristic-based evaluator (no LLM required)
#[derive(Debug, Clone, Default)]
pub struct HeuristicEvaluator;

#[async_trait]
impl FitnessEvaluator for HeuristicEvaluator {
    async fn evaluate(
        &self,
        response: &str,
        context: &EvaluationContext,
    ) -> FitnessReport {
        let mut metrics = HashMap::new();

        // Task completion: check if response is non-empty and substantial
        let completion = if response.len() > 50 {
            0.8
        } else if response.len() > 10 {
            0.5
        } else {
            0.2
        };
        metrics.insert("task_completion".to_string(), completion);

        // Coherence: check sentence structure (simple heuristic)
        let sentences = response.matches('.').count();
        let words = response.split_whitespace().count();
        let avg_sentence_len = if sentences > 0 { words / sentences } else { words };
        let coherence = if (10..40).contains(&avg_sentence_len) {
            0.8
        } else if avg_sentence_len < 60 {
            0.6
        } else {
            0.4
        };
        metrics.insert("coherence".to_string(), coherence);

        // Efficiency: penalize overly verbose responses
        let task_words = context.task.split_whitespace().count();
        let response_ratio = words as f64 / (task_words.max(10) as f64);
        let efficiency = if response_ratio < 5.0 {
            0.9
        } else if response_ratio < 10.0 {
            0.7
        } else {
            0.5
        };
        metrics.insert("efficiency".to_string(), efficiency);

        // Expected match (if available)
        if let Some(expected) = &context.expected_outcome {
            let expected_lower = expected.to_lowercase();
            let response_lower = response.to_lowercase();
            let accuracy = if response_lower.contains(&expected_lower) 
                || expected_lower.contains(&response_lower) {
                0.9
            } else {
                // Check word overlap
                let expected_words: std::collections::HashSet<_> = 
                    expected_lower.split_whitespace().collect();
                let response_words: std::collections::HashSet<_> = 
                    response_lower.split_whitespace().collect();
                let overlap = expected_words.intersection(&response_words).count();
                let total = expected_words.len().max(1);
                (overlap as f64 / total as f64).min(0.9)
            };
            metrics.insert("factual_accuracy".to_string(), accuracy);
        }

        FitnessReport::from_weighted(metrics, &default_weights())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitness_report_simple() {
        let report = FitnessReport::simple(0.85);
        assert_eq!(report.overall, 0.85);
        assert!(report.metrics.is_empty());
    }

    #[test]
    fn test_fitness_report_weighted() {
        let mut metrics = HashMap::new();
        metrics.insert("a".to_string(), 1.0);
        metrics.insert("b".to_string(), 0.5);

        let mut weights = HashMap::new();
        weights.insert("a".to_string(), 0.5);
        weights.insert("b".to_string(), 0.5);

        let report = FitnessReport::from_weighted(metrics, &weights);
        assert!((report.overall - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_add_metric() {
        let mut report = FitnessReport::simple(0.5);
        report.add_metric("test", 0.9);
        assert_eq!(report.metrics.get("test"), Some(&0.9));
    }

    #[tokio::test]
    async fn test_heuristic_evaluator() {
        let evaluator = HeuristicEvaluator;
        let context = EvaluationContext::new("Explain quantum computing");
        
        let response = "Quantum computing uses quantum bits or qubits. \
            Unlike classical bits that are 0 or 1, qubits can be in superposition. \
            This allows quantum computers to process many possibilities simultaneously.";

        let report = evaluator.evaluate(response, &context).await;
        
        assert!(report.overall > 0.5);
        assert!(report.metrics.contains_key("task_completion"));
        assert!(report.metrics.contains_key("coherence"));
    }

    #[tokio::test]
    async fn test_evaluator_with_expected() {
        let evaluator = HeuristicEvaluator;
        let context = EvaluationContext::new("What is 2+2?")
            .with_expected("4");
        
        let report = evaluator.evaluate("The answer is 4", &context).await;
        
        assert!(*report.metrics.get("factual_accuracy").unwrap_or(&0.0) > 0.5);
    }
}
