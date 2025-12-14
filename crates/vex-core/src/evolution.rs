//! Evolutionary operators for VEX agents
//!
//! Provides genetic algorithm primitives for agent evolution:
//! - Genome representation
//! - Crossover operators
//! - Mutation operators
//! - Fitness evaluation

use rand::Rng;
use serde::{Deserialize, Serialize};

/// A fitness score (higher is better)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Fitness(pub f64);

impl Fitness {
    /// Create a new fitness score
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Perfect fitness (1.0)
    pub fn perfect() -> Self {
        Self(1.0)
    }

    /// Zero fitness
    pub fn zero() -> Self {
        Self(0.0)
    }

    /// Get the raw value
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// A genome representing agent strategy/behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    /// System prompt/role (the "DNA")
    pub prompt: String,
    /// Strategy parameters (0.0 - 1.0 each)
    pub traits: Vec<f64>,
    /// Labels for each trait
    pub trait_names: Vec<String>,
}

impl Genome {
    /// Create a new genome with default traits
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            traits: vec![0.5; 5], // Default 5 traits at 0.5
            trait_names: vec![
                "exploration".to_string(),
                "precision".to_string(),
                "creativity".to_string(),
                "skepticism".to_string(),
                "verbosity".to_string(),
            ],
        }
    }

    /// Create genome with custom traits
    pub fn with_traits(prompt: &str, traits: Vec<(String, f64)>) -> Self {
        let (names, values): (Vec<_>, Vec<_>) = traits.into_iter().unzip();
        Self {
            prompt: prompt.to_string(),
            traits: values,
            trait_names: names,
        }
    }

    /// Get a named trait value
    pub fn get_trait(&self, name: &str) -> Option<f64> {
        self.trait_names
            .iter()
            .position(|n| n == name)
            .map(|i| self.traits[i])
    }

    /// Set a named trait value
    pub fn set_trait(&mut self, name: &str, value: f64) {
        if let Some(i) = self.trait_names.iter().position(|n| n == name) {
            self.traits[i] = value.clamp(0.0, 1.0);
        }
    }

    /// Convert genome traits to LLM parameters
    /// Maps:
    /// - exploration → temperature (0.0-1.0 → 0.1-1.5)
    /// - precision → top_p (0.0-1.0 → 0.5-1.0, inverted for precision)
    /// - creativity → presence_penalty (0.0-1.0 → 0.0-1.0)
    /// - skepticism → frequency_penalty (0.0-1.0 → 0.0-0.5)
    /// - verbosity → max_tokens scaling (0.0-1.0 → 0.5-2.0x multiplier)
    pub fn to_llm_params(&self) -> LlmParams {
        let exploration = self.get_trait("exploration").unwrap_or(0.5);
        let precision = self.get_trait("precision").unwrap_or(0.5);
        let creativity = self.get_trait("creativity").unwrap_or(0.5);
        let skepticism = self.get_trait("skepticism").unwrap_or(0.5);
        let verbosity = self.get_trait("verbosity").unwrap_or(0.5);

        LlmParams {
            // Higher exploration = higher temperature (more random)
            temperature: 0.1 + exploration * 1.4,
            // Higher precision = lower top_p (more focused on best tokens)
            top_p: 1.0 - (precision * 0.5),
            // Creativity adds presence penalty to encourage novel topics
            presence_penalty: creativity,
            // Skepticism adds frequency penalty to reduce repetition
            frequency_penalty: skepticism * 0.5,
            // Verbosity scales max_tokens (0.5x to 2.0x of base)
            max_tokens_multiplier: 0.5 + verbosity * 1.5,
        }
    }
}

/// LLM inference parameters derived from genome traits
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct LlmParams {
    /// Controls randomness (0.0 = deterministic, 2.0 = very random)
    pub temperature: f64,
    /// Nucleus sampling threshold (1.0 = all tokens, 0.5 = top 50% probability mass)
    pub top_p: f64,
    /// Penalty for tokens already in context (-2.0 to 2.0)
    pub presence_penalty: f64,
    /// Penalty for token frequency (-2.0 to 2.0)
    pub frequency_penalty: f64,
    /// Multiplier for max_tokens (applied to base value)
    pub max_tokens_multiplier: f64,
}

impl LlmParams {
    /// Apply multiplier to a base max_tokens value
    pub fn max_tokens(&self, base: u32) -> u32 {
        ((base as f64) * self.max_tokens_multiplier) as u32
    }

    /// Default conservative params (low temperature, high precision)
    pub fn conservative() -> Self {
        Self {
            temperature: 0.3,
            top_p: 0.9,
            presence_penalty: 0.0,
            frequency_penalty: 0.1,
            max_tokens_multiplier: 1.0,
        }
    }

    /// Creative params (higher temperature, lower precision)
    pub fn creative() -> Self {
        Self {
            temperature: 1.2,
            top_p: 0.95,
            presence_penalty: 0.5,
            frequency_penalty: 0.0,
            max_tokens_multiplier: 1.5,
        }
    }
}

/// Trait for genetic operators
pub trait GeneticOperator {
    /// Perform crossover between two parent genomes
    fn crossover(&self, parent_a: &Genome, parent_b: &Genome) -> Genome;

    /// Mutate a genome with given mutation rate
    fn mutate(&self, genome: &mut Genome, mutation_rate: f64);
}

/// Standard genetic operator implementation
#[derive(Debug, Clone, Default)]
pub struct StandardOperator;

impl GeneticOperator for StandardOperator {
    fn crossover(&self, parent_a: &Genome, parent_b: &Genome) -> Genome {
        let mut rng = rand::thread_rng();

        // Single-point crossover for traits
        let crossover_point = rng.gen_range(0..parent_a.traits.len());
        let mut child_traits = Vec::with_capacity(parent_a.traits.len());

        for i in 0..parent_a.traits.len() {
            if i < crossover_point {
                child_traits.push(parent_a.traits[i]);
            } else {
                child_traits.push(parent_b.traits[i]);
            }
        }

        // Randomly pick one parent's prompt (or could combine them)
        let prompt = if rng.gen_bool(0.5) {
            parent_a.prompt.clone()
        } else {
            parent_b.prompt.clone()
        };

        Genome {
            prompt,
            traits: child_traits,
            trait_names: parent_a.trait_names.clone(),
        }
    }

    fn mutate(&self, genome: &mut Genome, mutation_rate: f64) {
        let mut rng = rand::thread_rng();

        for trait_val in &mut genome.traits {
            if rng.gen_bool(mutation_rate) {
                // Gaussian mutation
                let delta: f64 = rng.gen_range(-0.2..0.2);
                *trait_val = (*trait_val + delta).clamp(0.0, 1.0);
            }
        }
    }
}

/// Select parents from a population based on fitness (tournament selection)
pub fn tournament_select(population: &[(Genome, Fitness)], tournament_size: usize) -> &Genome {
    let mut rng = rand::thread_rng();
    let mut best: Option<&(Genome, Fitness)> = None;

    for _ in 0..tournament_size {
        let idx = rng.gen_range(0..population.len());
        let candidate = &population[idx];

        if best.is_none() || candidate.1 > best.unwrap().1 {
            best = Some(candidate);
        }
    }

    &best.unwrap().0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genome_traits() {
        let mut genome = Genome::new("Test agent");
        assert_eq!(genome.get_trait("exploration"), Some(0.5));

        genome.set_trait("exploration", 0.8);
        assert_eq!(genome.get_trait("exploration"), Some(0.8));
    }

    #[test]
    fn test_crossover() {
        let parent_a =
            Genome::with_traits("A", vec![("a".to_string(), 0.0), ("b".to_string(), 0.0)]);
        let parent_b =
            Genome::with_traits("B", vec![("a".to_string(), 1.0), ("b".to_string(), 1.0)]);

        let operator = StandardOperator;
        let child = operator.crossover(&parent_a, &parent_b);

        // Child should have traits from both parents
        assert!(child.traits.iter().all(|&t| t == 0.0 || t == 1.0));
    }

    #[test]
    fn test_mutation() {
        let mut genome = Genome::new("Test");
        let original_traits = genome.traits.clone();

        let operator = StandardOperator;
        operator.mutate(&mut genome, 1.0); // 100% mutation rate

        // At least some traits should have changed
        assert!(genome
            .traits
            .iter()
            .zip(original_traits.iter())
            .any(|(a, b)| a != b));
    }
}
