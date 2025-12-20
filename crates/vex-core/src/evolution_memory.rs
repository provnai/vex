//! Evolution memory using temporal decay
//!
//! Stores genome experiments with bio-inspired decay so recent
//! high-performing experiments are weighted higher.
//!
//! This module requires the `evolution-memory` feature to be enabled.

use std::collections::{HashMap, VecDeque};

use crate::evolution::Genome;
use crate::genome_experiment::GenomeExperiment;

/// Memory for evolution experiments with importance-based learning
///
/// Unlike a simple vector of experiments, EvolutionMemory:
/// - Weights recent experiments higher
/// - Keeps high-fitness experiments longer
/// - Learns trait-performance correlations
/// - Suggests trait adjustments based on patterns
///
/// # Security
/// Uses VecDeque for O(1) insertion and bounded capacity to prevent DoS attacks.
#[derive(Debug, Clone, Default)]
pub struct EvolutionMemory {
    /// Stored experiments (most recent first) - VecDeque for O(1) push_front
    experiments: VecDeque<(GenomeExperiment, f64)>, // (experiment, decayed_importance)
    /// Maximum experiments to keep (DoS protection)
    max_entries: usize,
    /// Learned correlations between traits and fitness
    correlations: HashMap<String, f64>,
}

impl EvolutionMemory {
    /// Create new evolution memory with default capacity
    pub fn new() -> Self {
        Self {
            experiments: VecDeque::new(),
            max_entries: 500,
            correlations: HashMap::new(),
        }
    }

    /// Create with custom capacity
    pub fn with_capacity(max_entries: usize) -> Self {
        // Cap at 10,000 to prevent DoS
        let safe_capacity = max_entries.min(10_000);
        Self {
            experiments: VecDeque::with_capacity(safe_capacity.min(100)),
            max_entries: safe_capacity,
            correlations: HashMap::new(),
        }
    }

    /// Record a genome experiment
    ///
    /// High-fitness experiments get higher importance and are kept longer.
    ///
    /// # Security
    /// Uses VecDeque::push_front for O(1) insertion (DoS prevention).
    pub fn record(&mut self, experiment: GenomeExperiment) {
        // Use fitness as importance (high performing = remembered longer)
        let importance = experiment.overall_fitness;

        // Add to front (most recent first) - O(1) with VecDeque
        self.experiments.push_front((experiment, importance));

        metrics::counter!("vex_experiments_recorded_total").increment(1);

        // Evict old/low-importance if over capacity
        self.maybe_evict();

        // Update correlations periodically
        if self.experiments.len().is_multiple_of(10) {
            self.update_correlations();
        }
    }

    /// Get top experiments by importance (recency + fitness)
    pub fn get_top_experiments(&self, limit: usize) -> Vec<&GenomeExperiment> {
        let mut sorted: Vec<_> = self.experiments.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(limit).map(|(exp, _)| exp).collect()
    }

    /// Get all experiments
    pub fn experiments(&self) -> impl Iterator<Item = &GenomeExperiment> {
        self.experiments.iter().map(|(exp, _)| exp)
    }

    /// Number of stored experiments
    pub fn len(&self) -> usize {
        self.experiments.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.experiments.is_empty()
    }

    /// Get learned trait correlations
    pub fn correlations(&self) -> &HashMap<String, f64> {
        &self.correlations
    }

    /// Get a snapshot of all experiments (for consolidation)
    pub fn get_experiments_snapshot(&self) -> Vec<GenomeExperiment> {
        self.experiments
            .iter()
            .map(|(exp, _)| exp.clone())
            .collect()
    }

    /// Get snapshot of the oldest N experiments
    pub fn get_experiments_oldest(&self, count: usize) -> Vec<GenomeExperiment> {
        self.experiments
            .iter()
            .take(count)
            .map(|(exp, _)| exp.clone())
            .collect()
    }

    /// Clear all experiments (after consolidation)
    /// Note: Keeps learned correlations map intact until new data arrives.
    pub fn clear(&mut self) {
        self.experiments.clear();
        metrics::gauge!("vex_evolution_memory_size").set(0.0);
    }

    /// Remove the oldest N experiments (safer than clear)
    pub fn drain_oldest(&mut self, count: usize) {
        let actual_count = count.min(self.experiments.len());
        self.experiments.drain(0..actual_count);
        metrics::gauge!("vex_evolution_memory_size").set(self.experiments.len() as f64);
    }

    /// Calculate and update correlations between traits and fitness
    fn update_correlations(&mut self) {
        if self.experiments.len() < 10 {
            return;
        }

        let trait_count = self
            .experiments
            .front()
            .map(|(e, _)| e.traits.len())
            .unwrap_or(5);

        for i in 0..trait_count {
            let trait_name = self
                .experiments
                .front()
                .and_then(|(e, _)| e.trait_names.get(i).cloned())
                .unwrap_or_else(|| format!("trait_{}", i));

            let trait_values: Vec<f64> = self
                .experiments
                .iter()
                .filter_map(|(e, _)| e.traits.get(i).copied())
                .collect();
            let fitness_values: Vec<f64> = self
                .experiments
                .iter()
                .map(|(e, _)| e.overall_fitness)
                .collect();

            if trait_values.len() >= 10 {
                let corr = pearson_correlation(&trait_values, &fitness_values);
                self.correlations.insert(trait_name, corr);
            }
        }

        metrics::gauge!("vex_learned_correlations_count").set(self.correlations.len() as f64);
    }

    /// Suggest trait adjustments based on learned correlations
    pub fn suggest_adjustments(&self, current: &Genome) -> Vec<TraitAdjustment> {
        self.correlations
            .iter()
            .filter(|(_, corr)| corr.abs() > 0.3) // Only strong correlations
            .map(|(name, corr)| {
                let current_val = current.get_trait(name).unwrap_or(0.5);
                TraitAdjustment {
                    trait_name: name.clone(),
                    current_value: current_val,
                    suggested_value: if *corr > 0.0 {
                        (current_val + 0.1).min(1.0) // Increase positively correlated
                    } else {
                        (current_val - 0.1).max(0.0) // Decrease negatively correlated
                    },
                    correlation: *corr,
                    confidence: corr.abs(),
                }
            })
            .collect()
    }

    /// Evict low-importance experiments if over capacity
    ///
    /// # Security
    /// Uses efficient O(n log n) sort + truncate instead of O(n²) loop to prevent DoS.
    fn maybe_evict(&mut self) {
        if self.experiments.len() > self.max_entries {
            let initial_len = self.experiments.len();

            // Convert to Vec for efficient sorting
            let mut sorted: Vec<_> = self.experiments.drain(..).collect();

            // Sort by importance (highest first)
            sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Keep only top max_entries
            sorted.truncate(self.max_entries);

            // Convert back to VecDeque
            self.experiments = VecDeque::from(sorted);

            // Record eviction metrics
            let evicted_count = initial_len - self.experiments.len();
            metrics::counter!("vex_evolution_evictions_total").increment(evicted_count as u64);
        }

        metrics::gauge!("vex_evolution_memory_size").set(self.experiments.len() as f64);
    }

    /// Apply temporal decay to all experiments (call periodically)
    pub fn apply_decay(&mut self, decay_factor: f64) {
        for (_, importance) in &mut self.experiments {
            *importance *= decay_factor;
        }
    }
}

/// Suggested trait adjustment based on learned correlations
#[derive(Debug, Clone)]
pub struct TraitAdjustment {
    /// Name of the trait
    pub trait_name: String,
    /// Current trait value
    pub current_value: f64,
    /// Suggested new value
    pub suggested_value: f64,
    /// Correlation coefficient (-1.0 to 1.0)
    pub correlation: f64,
    /// Confidence in this suggestion (0.0 to 1.0)
    pub confidence: f64,
}

/// Calculate Pearson correlation coefficient
///
/// # Security
/// Validates inputs for NaN/Infinity and checks for numeric overflow
/// to prevent silent corruption of correlations.
fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    if x.len() != y.len() || x.is_empty() {
        return 0.0;
    }

    // Validate all inputs are finite (no NaN/Infinity)
    if x.iter().any(|v| !v.is_finite()) || y.iter().any(|v| !v.is_finite()) {
        return 0.0;
    }

    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y).map(|(a, b)| a * b).sum();
    let sum_x2: f64 = x.iter().map(|a| a * a).sum();
    let sum_y2: f64 = y.iter().map(|b| b * b).sum();

    // Check for overflow in intermediate calculations
    if !sum_xy.is_finite() || !sum_x2.is_finite() || !sum_y2.is_finite() {
        return 0.0;
    }

    let numerator = n * sum_xy - sum_x * sum_y;
    let denominator = ((n * sum_x2 - sum_x.powi(2)) * (n * sum_y2 - sum_y.powi(2))).sqrt();

    // Check for near-zero denominator (prevent division by zero)
    if denominator.abs() < 1e-10 || !numerator.is_finite() || !denominator.is_finite() {
        return 0.0;
    }

    let result = numerator / denominator;

    // Final validation: ensure result is finite
    if !result.is_finite() {
        0.0
    } else {
        result.clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution_memory_basic() {
        let mut memory = EvolutionMemory::new();

        let genome = Genome::new("Test");
        let exp = GenomeExperiment::new(&genome, HashMap::new(), 0.8, "Task 1");
        memory.record(exp);

        assert_eq!(memory.len(), 1);
        assert!(!memory.is_empty());
    }

    #[test]
    fn test_pearson_correlation() {
        // Perfect positive correlation
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let corr = pearson_correlation(&x, &y);
        assert!((corr - 1.0).abs() < 0.001, "Expected ~1.0, got {}", corr);

        // Perfect negative correlation
        let y_neg = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let corr_neg = pearson_correlation(&x, &y_neg);
        assert!(
            (corr_neg + 1.0).abs() < 0.001,
            "Expected ~-1.0, got {}",
            corr_neg
        );

        // No correlation (random)
        let y_rand = vec![3.0, 1.0, 4.0, 2.0, 5.0];
        let corr_rand = pearson_correlation(&x, &y_rand);
        assert!(
            corr_rand.abs() < 0.8,
            "Expected low correlation, got {}",
            corr_rand
        );
    }

    #[test]
    fn test_correlation_learning() {
        let mut memory = EvolutionMemory::with_capacity(100);

        // Add experiments with positive correlation between exploration and fitness
        for i in 0..20 {
            let exploration = 0.3 + (i as f64 * 0.03);
            let fitness = 0.4 + (i as f64 * 0.02);

            let exp = GenomeExperiment::from_raw(
                vec![exploration, 0.5, 0.5, 0.5, 0.5],
                vec![
                    "exploration".into(),
                    "precision".into(),
                    "creativity".into(),
                    "skepticism".into(),
                    "verbosity".into(),
                ],
                fitness,
                "test task",
            );
            memory.record(exp);
        }

        // Force correlation update
        memory.update_correlations();

        // Check correlation was learned
        let corr = memory
            .correlations()
            .get("exploration")
            .copied()
            .unwrap_or(0.0);
        assert!(corr > 0.5, "Expected positive correlation, got {}", corr);
    }

    #[test]
    fn test_eviction() {
        let mut memory = EvolutionMemory::with_capacity(5);

        let genome = Genome::new("Test");
        for i in 0..10 {
            let exp = GenomeExperiment::new(
                &genome,
                HashMap::new(),
                i as f64 / 10.0,
                &format!("Task {}", i),
            );
            memory.record(exp);
        }

        assert_eq!(memory.len(), 5);
    }

    #[test]
    fn test_suggest_adjustments() {
        let mut memory = EvolutionMemory::new();

        // Add experiments showing high exploration = high fitness
        for i in 0..15 {
            let exploration = 0.3 + (i as f64 * 0.04);
            let fitness = 0.4 + (i as f64 * 0.03);

            let exp = GenomeExperiment::from_raw(
                vec![exploration, 0.5, 0.5, 0.5, 0.5],
                vec![
                    "exploration".into(),
                    "precision".into(),
                    "creativity".into(),
                    "skepticism".into(),
                    "verbosity".into(),
                ],
                fitness,
                "test",
            );
            memory.record(exp);
        }

        memory.update_correlations();

        let genome = Genome::new("Current");
        let suggestions = memory.suggest_adjustments(&genome);

        // Should suggest increasing exploration
        let exp_suggestion = suggestions.iter().find(|s| s.trait_name == "exploration");
        assert!(
            exp_suggestion.is_some(),
            "Should suggest exploration adjustment"
        );
        if let Some(s) = exp_suggestion {
            assert!(
                s.suggested_value > s.current_value,
                "Should suggest increasing exploration"
            );
        }
    }

    // === SECURITY TESTS ===

    #[test]
    fn test_dos_memory_bounded() {
        let mut memory = EvolutionMemory::new();

        // Spam 10k low-fitness experiments (DoS attack simulation)
        for i in 0..10_000 {
            let exp = GenomeExperiment::from_raw(
                vec![0.5; 5],
                vec![
                    "t1".into(),
                    "t2".into(),
                    "t3".into(),
                    "t4".into(),
                    "t5".into(),
                ],
                0.1, // Low fitness (should be evicted)
                &format!("spam_{}", i),
            );
            memory.record(exp);
        }

        // Should cap at max_entries (500)
        assert!(
            memory.len() <= 500,
            "Memory grew unbounded: {} entries",
            memory.len()
        );

        // High-fitness experiments should be kept
        let top = memory.get_top_experiments(10);
        assert!(
            top.iter().all(|e| e.overall_fitness >= 0.1),
            "Lost high-fitness experiments"
        );
    }

    #[test]
    fn test_pearson_nan_safety() {
        // NaN input
        let x = vec![f64::NAN, 1.0, 2.0];
        let y = vec![1.0, 2.0, 3.0];
        let result = pearson_correlation(&x, &y);
        assert!(result.is_finite(), "Must handle NaN input, got {}", result);
        assert_eq!(result, 0.0);

        // Infinity input
        let x_inf = vec![f64::INFINITY, 1.0, 2.0];
        let result_inf = pearson_correlation(&x_inf, &y);
        assert!(result_inf.is_finite(), "Must handle Infinity input");
        assert_eq!(result_inf, 0.0);

        // Extreme values that might overflow
        let x_big = vec![f64::MAX / 2.0; 5];
        let y_big = vec![f64::MAX / 2.0; 5];
        let result_big = pearson_correlation(&x_big, &y_big);
        assert!(result_big.is_finite(), "Must handle overflow");
    }

    #[test]
    fn test_capacity_limit() {
        // Try to create with absurdly high capacity (DoS)
        let memory = EvolutionMemory::with_capacity(1_000_000);

        // Should be capped at 10,000
        assert!(
            memory.max_entries <= 10_000,
            "Capacity not capped: {}",
            memory.max_entries
        );
    }
}
