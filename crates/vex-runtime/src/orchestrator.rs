//! Orchestrator - manages hierarchical agent networks

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use vex_core::{Agent, AgentConfig, MerkleTree, Hash, Fitness, Genome, GeneticOperator, StandardOperator, tournament_select};

use crate::executor::{AgentExecutor, ExecutionResult, ExecutorConfig, LlmBackend};

/// Configuration for the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum depth of agent hierarchy
    pub max_depth: u8,
    /// Number of agents per level
    pub agents_per_level: usize,
    /// Enable evolutionary selection
    pub enable_evolution: bool,
    /// Mutation rate for evolution
    pub mutation_rate: f64,
    /// Executor configuration
    pub executor_config: ExecutorConfig,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            agents_per_level: 2,
            enable_evolution: true,
            mutation_rate: 0.1,
            executor_config: ExecutorConfig::default(),
        }
    }
}

/// Result from orchestrated execution
#[derive(Debug)]
pub struct OrchestrationResult {
    /// Root agent ID
    pub root_agent_id: Uuid,
    /// Final synthesized response
    pub response: String,
    /// Merkle root of all context packets
    pub merkle_root: Hash,
    /// All execution results (agent_id -> result)
    pub agent_results: HashMap<Uuid, ExecutionResult>,
    /// Total levels processed
    pub levels_processed: u8,
    /// Overall confidence
    pub confidence: f64,
}

/// Orchestrator manages hierarchical agent execution
pub struct Orchestrator<L: LlmBackend> {
    /// Configuration
    pub config: OrchestratorConfig,
    /// All agents (id -> agent)
    agents: RwLock<HashMap<Uuid, Agent>>,
    /// Executor
    executor: AgentExecutor<L>,
    /// LLM backend (stored for future use)
    #[allow(dead_code)]
    llm: Arc<L>,
}

impl<L: LlmBackend + 'static> Orchestrator<L> {
    /// Create a new orchestrator
    pub fn new(llm: Arc<L>, config: OrchestratorConfig) -> Self {
        let executor = AgentExecutor::new(llm.clone(), config.executor_config.clone());
        Self {
            config,
            agents: RwLock::new(HashMap::new()),
            executor,
            llm,
        }
    }

    /// Process a query with full hierarchical agent network
    pub async fn process(&self, query: &str) -> Result<OrchestrationResult, String> {
        let mut agents = self.agents.write().await;
        let mut all_results: HashMap<Uuid, ExecutionResult> = HashMap::new();

        // Create root agent
        let root_config = AgentConfig {
            name: "Root".to_string(),
            role: "You are a strategic coordinator. Synthesize information from sub-agents into a coherent response.".to_string(),
            max_depth: self.config.max_depth,
            spawn_shadow: true,
        };
        let root = Agent::new(root_config);
        let root_id = root.id;
        agents.insert(root_id, root);

        // Spawn child agents for research
        let child_configs = vec![
            AgentConfig {
                name: "Researcher".to_string(),
                role: "You are a thorough researcher. Analyze the query and provide detailed findings.".to_string(),
                max_depth: 1,
                spawn_shadow: true,
            },
            AgentConfig {
                name: "Critic".to_string(),
                role: "You are a critical analyzer. Identify potential issues, edge cases, and weaknesses.".to_string(),
                max_depth: 1,
                spawn_shadow: true,
            },
        ];

        // Execute child agents in parallel
        let mut child_results = Vec::new();
        for config in child_configs.into_iter().take(self.config.agents_per_level) {
            let root = agents.get(&root_id).unwrap();
            let mut child = root.spawn_child(config);
            let child_id = child.id;

            // Execute child
            let result = self.executor.execute(&mut child, query).await?;
            child_results.push((child_id, result.clone()));
            all_results.insert(child_id, result);
            agents.insert(child_id, child);
        }

        // Synthesize child results at root level
        let synthesis_prompt = format!(
            "Based on the following research from your sub-agents, provide a comprehensive answer:\n\n\
             Original Query: \"{}\"\n\n\
             Researcher's Findings: \"{}\"\n\n\
             Critic's Analysis: \"{}\"\n\n\
             Synthesize these into a final, well-reasoned response.",
            query,
            child_results.get(0).map(|(_, r)| r.response.as_str()).unwrap_or("N/A"),
            child_results.get(1).map(|(_, r)| r.response.as_str()).unwrap_or("N/A"),
        );

        let root = agents.get_mut(&root_id).unwrap();
        let root_result = self.executor.execute(root, &synthesis_prompt).await?;
        all_results.insert(root_id, root_result.clone());

        // Build Merkle tree from all context packets
        let leaves: Vec<(String, Hash)> = all_results
            .iter()
            .map(|(id, r)| (id.to_string(), r.context.hash.clone()))
            .collect();
        let merkle_tree = MerkleTree::from_leaves(leaves);

        // Calculate overall confidence
        let total_confidence: f64 = all_results.values().map(|r| r.confidence).sum();
        let avg_confidence = total_confidence / all_results.len() as f64;

        // Evolution step (if enabled)
        if self.config.enable_evolution {
            self.evolve_agents(&mut agents, &all_results);
        }

        Ok(OrchestrationResult {
            root_agent_id: root_id,
            response: root_result.response,
            merkle_root: merkle_tree.root_hash().cloned().unwrap_or(Hash::digest(b"empty")),
            agent_results: all_results,
            levels_processed: 2,
            confidence: avg_confidence,
        })
    }

    /// Evolve agents based on fitness - persists evolved genome to fittest agent
    fn evolve_agents(&self, agents: &mut HashMap<Uuid, Agent>, results: &HashMap<Uuid, ExecutionResult>) {
        let operator = StandardOperator;

        // Build population with fitness scores from actual agent genomes
        let population: Vec<(Genome, Fitness)> = agents
            .values()
            .map(|a| {
                let fitness = results.get(&a.id).map(|r| r.confidence).unwrap_or(0.5);
                (a.genome.clone(), Fitness::new(fitness))
            })
            .collect();

        if population.len() < 2 {
            return;
        }

        // Select parents via tournament selection and create offspring
        let parent_a = tournament_select(&population, 2);
        let parent_b = tournament_select(&population, 2);
        let mut offspring = operator.crossover(parent_a, parent_b);
        operator.mutate(&mut offspring, self.config.mutation_rate);

        // Find the fittest agent and apply the evolved genome to it
        // This ensures the best-performing agent gets improved traits for next generation
        if let Some((best_id, _best_fitness)) = results
            .iter()
            .max_by(|a, b| a.1.confidence.partial_cmp(&b.1.confidence).unwrap())
        {
            if let Some(agent) = agents.get_mut(best_id) {
                let old_traits = agent.genome.traits.clone();
                agent.apply_evolved_genome(offspring.clone());
                
                tracing::info!(
                    agent_id = %best_id,
                    old_traits = ?old_traits,
                    new_traits = ?offspring.traits,
                    "Evolved genome applied to fittest agent"
                );
            }
        }
    }

    /// Get agent by ID
    pub async fn get_agent(&self, id: Uuid) -> Option<Agent> {
        self.agents.read().await.get(&id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockLlm;

    #[async_trait]
    impl LlmBackend for MockLlm {
        async fn complete(&self, system: &str, _prompt: &str) -> Result<String, String> {
            if system.contains("researcher") {
                Ok("Research finding: This is a detailed analysis of the topic.".to_string())
            } else if system.contains("critic") {
                Ok("Critical analysis: The main concern is validation of assumptions.".to_string())
            } else {
                Ok("Synthesized response combining all findings into a coherent answer.".to_string())
            }
        }
    }

    #[tokio::test]
    async fn test_orchestrator() {
        let llm = Arc::new(MockLlm);
        let orchestrator = Orchestrator::new(llm, OrchestratorConfig::default());

        let result = orchestrator.process("What is the meaning of life?").await.unwrap();

        assert!(!result.response.is_empty());
        assert!(result.confidence > 0.0);
        assert!(!result.agent_results.is_empty());
    }
}
