//! Genome experiment recording for evolution learning
//!
//! Records trait-fitness pairs as experiments that can be stored
//! in temporal memory for pattern learning.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::evolution::Genome;

/// A single genome experiment result
///
/// Records the genome traits used, the fitness scores achieved,
/// and metadata about the experiment for later analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomeExperiment {
    /// The genome traits that were tested (copies of values, not names)
    pub traits: Vec<f64>,
    /// Trait names for reference
    pub trait_names: Vec<String>,
    /// Fitness scores by metric name (e.g., "task_completion": 0.8)
    pub fitness_scores: HashMap<String, f64>,
    /// Overall fitness (0.0-1.0) - weighted average of metrics
    pub overall_fitness: f64,
    /// Task description (truncated for storage efficiency)
    pub task_summary: String,
    /// Whether this was a successful experiment (fitness > threshold)
    pub successful: bool,
    /// Unique identifier for this experiment
    pub id: uuid::Uuid,
    /// Timestamp when experiment was recorded
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl GenomeExperiment {
    /// Create a new genome experiment from a genome and fitness results
    ///
    /// # Arguments
    /// * `genome` - The genome that was tested
    /// * `fitness_scores` - Individual metric scores
    /// * `overall` - Overall fitness score (0.0-1.0)
    /// * `task` - Task description (will be truncated to 200 chars)
    ///
    /// # Example
    /// ```
    /// use vex_core::{Genome, GenomeExperiment};
    /// use std::collections::HashMap;
    ///
    /// let genome = Genome::new("Test agent");
    /// let mut scores = HashMap::new();
    /// scores.insert("accuracy".to_string(), 0.85);
    ///
    /// let exp = GenomeExperiment::new(&genome, scores, 0.85, "Summarize document");
    /// assert!(exp.successful);
    /// ```
    pub fn new(
        genome: &Genome,
        mut fitness_scores: HashMap<String, f64>,
        overall: f64,
        task: &str,
    ) -> Self {
        // Validate and sanitize fitness scores (security: prevent NaN/Infinity)
        fitness_scores.retain(|k, v| {
            !k.is_empty() &&           // No empty keys
            k.len() < 100 &&           // Prevent memory DoS
            v.is_finite() &&           // No NaN/Infinity
            *v >= 0.0 &&               // Valid range
            *v <= 1.0
        });

        // Sanitize overall fitness
        let overall_fitness = if overall.is_finite() && (0.0..=1.0).contains(&overall) {
            overall
        } else {
            0.5 // Safe default for invalid input
        };

        // Sanitize task summary (security: prevent log injection)
        let sanitized_task: String = task
            .chars()
            .filter(|c| {
                // Allow alphanumeric, whitespace, and safe punctuation
                c.is_alphanumeric()
                    || c.is_whitespace() && *c == ' ' // Only spaces, no CRLF
                    || ".,!?-_:;()[]{}".contains(*c)
            })
            .take(200)
            .collect();

        Self {
            traits: genome.traits.clone(),
            trait_names: genome.trait_names.clone(),
            fitness_scores,
            overall_fitness,
            task_summary: sanitized_task,
            successful: overall_fitness > 0.6,
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create from raw values (for deserialization or testing)
    pub fn from_raw(
        traits: Vec<f64>,
        trait_names: Vec<String>,
        overall_fitness: f64,
        task_summary: &str,
    ) -> Self {
        Self {
            traits,
            trait_names,
            fitness_scores: HashMap::new(),
            overall_fitness,
            task_summary: task_summary.to_string(),
            successful: overall_fitness > 0.6,
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get a specific trait value by name
    pub fn get_trait(&self, name: &str) -> Option<f64> {
        self.trait_names
            .iter()
            .position(|n| n == name)
            .and_then(|i| self.traits.get(i).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_creation() {
        let genome = Genome::new("Test");
        let mut scores = HashMap::new();
        scores.insert("accuracy".to_string(), 0.9);
        scores.insert("coherence".to_string(), 0.8);

        let exp = GenomeExperiment::new(&genome, scores, 0.85, "Test task");

        assert_eq!(exp.traits.len(), 5);
        assert_eq!(exp.overall_fitness, 0.85);
        assert!(exp.successful);
        assert_eq!(exp.task_summary, "Test task");
    }

    #[test]
    fn test_get_trait() {
        let genome = Genome::new("Test");
        let exp = GenomeExperiment::new(&genome, HashMap::new(), 0.5, "Task");

        assert!(exp.get_trait("exploration").is_some());
        assert_eq!(exp.get_trait("exploration"), Some(0.5));
        assert!(exp.get_trait("nonexistent").is_none());
    }

    #[test]
    fn test_success_threshold() {
        let genome = Genome::new("Test");

        let success = GenomeExperiment::new(&genome, HashMap::new(), 0.7, "Task");
        assert!(success.successful);

        let failure = GenomeExperiment::new(&genome, HashMap::new(), 0.5, "Task");
        assert!(!failure.successful);
    }

    // === SECURITY TESTS ===

    #[test]
    fn test_task_injection_sanitized() {
        let genome = Genome::new("test");
        let malicious = "Task\x00\n\rINJECTED\x1b[31mRED";
        let exp = GenomeExperiment::new(&genome, HashMap::new(), 0.5, malicious);

        assert!(!exp.task_summary.contains('\x00'), "Null byte not removed");
        assert!(!exp.task_summary.contains('\n'), "Newline not removed");
        assert!(
            !exp.task_summary.contains('\r'),
            "Carriage return not removed"
        );
        assert!(
            !exp.task_summary.contains('\x1b'),
            "Escape sequence not removed"
        );
    }

    #[test]
    fn test_fitness_nan_validation() {
        let genome = Genome::new("test");
        let mut scores = HashMap::new();
        scores.insert("nan_metric".to_string(), f64::NAN);
        scores.insert("inf_metric".to_string(), f64::INFINITY);
        scores.insert("valid_metric".to_string(), 0.8);
        scores.insert("out_of_range".to_string(), 1.5);

        let exp = GenomeExperiment::new(&genome, scores, f64::NAN, "task");

        // NaN/Inf should be filtered out
        assert!(!exp.fitness_scores.contains_key("nan_metric"));
        assert!(!exp.fitness_scores.contains_key("inf_metric"));
        assert!(!exp.fitness_scores.contains_key("out_of_range"));

        // Valid should be kept
        assert_eq!(exp.fitness_scores.get("valid_metric"), Some(&0.8));

        // Overall fitness should default to 0.5 for NaN
        assert_eq!(exp.overall_fitness, 0.5);
    }

    #[test]
    fn test_fitness_key_length_limit() {
        let genome = Genome::new("test");
        let mut scores = HashMap::new();
        scores.insert("A".repeat(200), 0.5); // Too long
        scores.insert("valid_key".to_string(), 0.8);
        scores.insert("".to_string(), 0.9); // Empty

        let exp = GenomeExperiment::new(&genome, scores, 0.5, "task");

        // Long and empty keys should be filtered
        assert!(exp.fitness_scores.len() == 1);
        assert_eq!(exp.fitness_scores.get("valid_key"), Some(&0.8));
    }
}
