//! Orchestrator - manages hierarchical agent networks

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use vex_core::{
    tournament_select, Agent, AgentConfig, Fitness, GeneticOperator, Genome, Hash, MerkleTree,
    StandardOperator,
};

use crate::executor::{AgentExecutor, ExecutionResult, ExecutorConfig};
use vex_llm::LlmProvider;

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
    /// Maximum age for tracked agents before cleanup (prevents memory leaks)
    pub max_agent_age: Duration,
    /// Enable self-correcting genome evolution
    pub enable_self_correction: bool,
    /// Minimum fitness improvement to accept change
    pub improvement_threshold: f64,
    /// Number of tasks before reflection
    pub reflect_every_n_tasks: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            agents_per_level: 2,
            enable_evolution: true,
            mutation_rate: 0.1,
            executor_config: ExecutorConfig::default(),
            max_agent_age: Duration::from_secs(3600), // 1 hour default
            enable_self_correction: false,
            improvement_threshold: 0.02,
            reflect_every_n_tasks: 5,
        }
    }
}

use vex_anchor::{AnchorBackend, AnchorMetadata, AnchorReceipt};

/// Result from orchestrated execution
#[derive(Debug)]
pub struct OrchestrationResult {
    /// Root agent ID
    pub root_agent_id: Uuid,
    /// Final synthesized response
    pub response: String,
    /// Merkle root of all context packets
    pub merkle_root: Hash,
    /// Aggregated trace root from all agents
    pub trace_root: Option<Hash>,
    /// All execution results (agent_id -> result)
    pub agent_results: HashMap<Uuid, ExecutionResult>,
    /// Anchor receipts from blockchain backends
    pub anchor_receipts: Vec<AnchorReceipt>,
    /// Total levels processed
    pub levels_processed: u8,
    /// Overall confidence
    pub confidence: f64,
}

/// Tracked agent with creation timestamp for TTL-based cleanup
#[derive(Clone)]
struct TrackedAgent {
    agent: Agent,
    _tenant_id: String,
    created_at: Instant,
}

/// Orchestrator manages hierarchical agent execution
pub struct Orchestrator<L: LlmProvider> {
    /// Configuration
    pub config: OrchestratorConfig,
    /// All agents (id -> tracked agent with timestamp)
    agents: RwLock<HashMap<Uuid, TrackedAgent>>,
    /// Executor
    executor: AgentExecutor<L>,
    /// Anchoring backends (Blockchain, Cloud, etc)
    anchors: Vec<Arc<dyn AnchorBackend>>,
    /// LLM backend (stored for future use)
    #[allow(dead_code)]
    llm: Arc<L>,
    /// Evolution memory for self-correction (optional)
    evolution_memory: Option<RwLock<vex_core::EvolutionMemory>>,
    /// Reflection agent for LLM-based suggestions (optional)
    reflection_agent: Option<vex_adversarial::ReflectionAgent<L>>,
    /// Persistence layer for cross-session learning (optional)
    persistence_layer: Option<Arc<dyn vex_persist::EvolutionStore>>,
}

impl<L: LlmProvider + 'static> Orchestrator<L> {
    /// Create a new orchestrator
    pub fn new(
        llm: Arc<L>,
        config: OrchestratorConfig,
        persistence_layer: Option<Arc<dyn vex_persist::EvolutionStore>>,
    ) -> Self {
        let executor = AgentExecutor::new(llm.clone(), config.executor_config.clone());
        let evolution_memory = if config.enable_self_correction {
            Some(RwLock::new(vex_core::EvolutionMemory::new()))
        } else {
            None
        };
        let reflection_agent = if config.enable_self_correction {
            Some(vex_adversarial::ReflectionAgent::new(llm.clone()))
        } else {
            None
        };
        Self {
            config,
            agents: RwLock::new(HashMap::new()),
            executor,
            anchors: Vec::new(),
            llm,
            evolution_memory,
            reflection_agent,
            persistence_layer,
        }
    }

    /// Add an anchoring backend
    pub fn add_anchor(&mut self, anchor: Arc<dyn AnchorBackend>) {
        self.anchors.push(anchor);
    }

    /// Cleanup expired agents to prevent memory leaks
    /// Returns the number of agents removed
    pub async fn cleanup_expired(&self) -> usize {
        let mut agents = self.agents.write().await;
        let before = agents.len();
        agents.retain(|_, tracked| tracked.created_at.elapsed() < self.config.max_agent_age);
        let removed = before - agents.len();
        if removed > 0 {
            tracing::info!(
                removed = removed,
                remaining = agents.len(),
                "Cleaned up expired agents"
            );
        }
        removed
    }

    /// Get current agent count
    pub async fn agent_count(&self) -> usize {
        self.agents.read().await.len()
    }

    /// Process a query with full hierarchical agent network
    pub async fn process(
        &self,
        tenant_id: &str,
        query: &str,
    ) -> Result<OrchestrationResult, String> {
        // Create root agent
        let root_config = AgentConfig {
            name: "Root".to_string(),
            role: "You are a strategic coordinator. Synthesize information from sub-agents into a coherent response.".to_string(),
            max_depth: self.config.max_depth,
            spawn_shadow: true,
        };
        let mut root = Agent::new(root_config);
        let root_id = root.id;

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

        // Execute child agents in parallel (no lock held during await)
        let mut execution_futures = Vec::new();
        for config in child_configs.into_iter().take(self.config.agents_per_level) {
            let mut child = root.spawn_child(config);
            let executor = self.executor.clone();
            let query_str = query.to_string();

            execution_futures.push(tokio::spawn(async move {
                let result = executor.execute(&mut child, &query_str).await;
                (child.id, child, result)
            }));
        }

        let task_results = futures::future::join_all(execution_futures).await;

        // Re-acquire lock to update agents map
        let mut agents = self.agents.write().await;

        let mut all_results: HashMap<Uuid, ExecutionResult> = HashMap::new();
        let mut child_results = Vec::new();
        for task_result in task_results {
            let (child_id, child, result) = task_result.map_err(|e| e.to_string())?;
            let execution_result: ExecutionResult = result?;

            child_results.push((child_id, execution_result.clone()));
            all_results.insert(child_id, execution_result);
            agents.insert(
                child_id,
                TrackedAgent {
                    agent: child,
                    _tenant_id: tenant_id.to_string(),
                    created_at: Instant::now(),
                },
            );
        }

        // Drop lock before root synthesis
        drop(agents);

        // Synthesize child results at root level
        let synthesis_prompt = format!(
            "Based on the following research from your sub-agents, provide a comprehensive answer:\n\n\
             Original Query: \"{}\"\n\n\
             Researcher's Findings: \"{}\"\n\n\
             Critic's Analysis: \"{}\"\n\n\
             Synthesize these into a final, well-reasoned response.",
            query,
            child_results.first().map(|(_, r)| r.response.as_str()).unwrap_or("N/A"),
            child_results.get(1).map(|(_, r)| r.response.as_str()).unwrap_or("N/A"),
        );

        let root_result = self.executor.execute(&mut root, &synthesis_prompt).await?;
        all_results.insert(root_id, root_result.clone());

        // Re-acquire lock to update root and run evolution
        let mut agents = self.agents.write().await;

        // Insert root after children are handled
        agents.insert(
            root_id,
            TrackedAgent {
                agent: root,
                _tenant_id: tenant_id.to_string(),
                created_at: Instant::now(),
            },
        );

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
            if self.config.enable_self_correction {
                self.evolve_agents_self_correcting(tenant_id, &mut agents, &all_results)
                    .await;
            } else {
                self.evolve_agents(tenant_id, &mut agents, &all_results);
            }
        }

        // Build trace merkle tree from agent trace roots
        let trace_leaves: Vec<(String, Hash)> = all_results
            .iter()
            .filter_map(|(id, r)| r.trace_root.clone().map(|tr| (id.to_string(), tr)))
            .collect();
        let trace_merkle = MerkleTree::from_leaves(trace_leaves);

        // Anchoring Step
        let mut anchor_receipts = Vec::new();
        if let Some(root_hash) = merkle_tree.root_hash() {
            let metadata = AnchorMetadata::new(tenant_id, all_results.len() as u64);
            for anchor in &self.anchors {
                match anchor.anchor(root_hash, metadata.clone()).await {
                    Ok(receipt) => anchor_receipts.push(receipt),
                    Err(e) => tracing::warn!("Anchoring to {} failed: {}", anchor.name(), e),
                }
            }
        }

        Ok(OrchestrationResult {
            root_agent_id: root_id,
            response: root_result.response,
            merkle_root: merkle_tree
                .root_hash()
                .cloned()
                .unwrap_or(Hash::digest(b"empty")),
            trace_root: trace_merkle.root_hash().cloned(),
            agent_results: all_results,
            anchor_receipts,
            levels_processed: 2,
            confidence: avg_confidence,
        })
    }

    /// Evolve agents based on fitness - persists evolved genome to fittest agent
    fn evolve_agents(
        &self,
        _tenant_id: &str,
        agents: &mut HashMap<Uuid, TrackedAgent>,
        results: &HashMap<Uuid, ExecutionResult>,
    ) {
        let operator = StandardOperator;

        // Build population with fitness scores from actual agent genomes
        let population: Vec<(Genome, Fitness)> = agents
            .values()
            .map(|tracked| {
                let fitness = results
                    .get(&tracked.agent.id)
                    .map(|r| r.confidence)
                    .unwrap_or(0.5);
                (tracked.agent.genome.clone(), Fitness::new(fitness))
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

        // Find the least fit agent and apply the evolved genome to it (Elitism)
        // We preserve the 'best' and replace the 'worst' to ensure no regression.
        if let Some((worst_id, _worst_fitness)) = results.iter().min_by(|a, b| {
            a.1.confidence
                .partial_cmp(&b.1.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            if let Some(tracked) = agents.get_mut(worst_id) {
                let old_traits = tracked.agent.genome.traits.clone();
                tracked.agent.apply_evolved_genome(offspring.clone());

                tracing::info!(
                    agent_id = %worst_id,
                    old_traits = ?old_traits,
                    new_traits = ?offspring.traits,
                    "Evolved genome applied to least fit agent (Elitism preserved fittest)"
                );
            }
        }
    }

    /// Self-correcting evolution using temporal memory and statistical learning
    ///
    /// This enhances basic evolution with:
    /// - Temporal memory of past experiments  
    /// - Statistical correlation learning (Pearson)
    /// - Intelligent trait adjustment suggestions
    ///
    /// # Modular Design
    /// Users can override this for custom strategies.
    async fn evolve_agents_self_correcting(
        &self,
        tenant_id: &str,
        agents: &mut HashMap<Uuid, TrackedAgent>,
        results: &HashMap<Uuid, ExecutionResult>,
    ) {
        // Require evolution memory
        let memory = match &self.evolution_memory {
            Some(mem) => mem,
            None => {
                tracing::warn!("Self-correction enabled but memory not initialized");
                return self.evolve_agents(tenant_id, agents, results);
            }
        };

        // Record experiments to memory and collect for persistence
        let experiments_to_save: Vec<vex_core::GenomeExperiment> = {
            let mut memory_guard = memory.write().await;
            let mut experiments = Vec::new();

            for (id, result) in results {
                if let Some(tracked) = agents.get(id) {
                    let mut fitness_scores = std::collections::HashMap::new();
                    fitness_scores.insert("confidence".to_string(), result.confidence);

                    let experiment = vex_core::GenomeExperiment::new(
                        &tracked.agent.genome,
                        fitness_scores,
                        result.confidence,
                        &format!("Depth {}", tracked.agent.depth),
                    );
                    memory_guard.record(experiment.clone());
                    experiments.push(experiment);
                }
            }
            experiments
        }; // Release lock here

        // Save to persistence (async, no lock held)
        if let Some(store) = &self.persistence_layer {
            for experiment in experiments_to_save {
                if let Err(e) = store.save_experiment(tenant_id, &experiment).await {
                    tracing::warn!("Failed to persist evolution experiment: {}", e);
                }
            }
        }

        // Find best performer
        let best = results.iter().max_by(|a, b| {
            a.1.confidence
                .partial_cmp(&b.1.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some((best_id, best_result)) = best {
            if let Some(tracked) = agents.get_mut(best_id) {
                // Get suggestions using dual-mode learning (statistical + LLM)
                let suggestions = if let Some(ref reflection) = self.reflection_agent {
                    // Use ReflectionAgent for LLM + statistical suggestions
                    let memory_guard = memory.read().await;

                    let reflection_result = reflection
                        .reflect(
                            &tracked.agent,
                            &format!("Orchestrated task at depth {}", tracked.agent.depth),
                            &best_result.response,
                            best_result.confidence,
                            &memory_guard,
                        )
                        .await;

                    drop(memory_guard);

                    // Convert to trait adjustments format
                    reflection_result
                        .adjustments
                        .into_iter()
                        .map(|(name, current, suggested)| {
                            vex_core::TraitAdjustment {
                                trait_name: name,
                                current_value: current,
                                suggested_value: suggested,
                                correlation: 0.5, // From LLM, not pure statistical
                                confidence: reflection_result.expected_improvement,
                            }
                        })
                        .collect()
                } else {
                    // Fallback to statistical-only if ReflectionAgent not available
                    let memory_guard = memory.read().await;
                    let suggestions = memory_guard.suggest_adjustments(&tracked.agent.genome);
                    drop(memory_guard);
                    suggestions
                };

                if !suggestions.is_empty() {
                    let old_traits = tracked.agent.genome.traits.clone();

                    // Apply suggestions with high confidence
                    for (i, name) in tracked.agent.genome.trait_names.iter().enumerate() {
                        if let Some(sugg) = suggestions.iter().find(|s| &s.trait_name == name) {
                            if sugg.confidence >= 0.3 {
                                tracked.agent.genome.traits[i] = sugg.suggested_value;
                            }
                        }
                    }

                    if old_traits != tracked.agent.genome.traits {
                        let source = if self.reflection_agent.is_some() {
                            "LLM + Statistical"
                        } else {
                            "Statistical"
                        };

                        tracing::info!(
                            agent_id = %best_id,
                            old_traits = ?old_traits,
                            new_traits = ?tracked.agent.genome.traits,
                            suggestions = suggestions.len(),
                            source = source,
                            "Self-correcting genome applied"
                        );
                    }
                } else {
                    // Fallback to standard evolution
                    self.evolve_agents(tenant_id, agents, results);
                }
            }
        }

        // Periodically check for memory consolidation
        self.maybe_consolidate_memory(tenant_id).await;
    }

    /// Check if memory needs consolidation and perform it if necessary
    async fn maybe_consolidate_memory(&self, tenant_id: &str) {
        let memory = match &self.evolution_memory {
            Some(m) => m,
            None => return,
        };

        let reflection = match &self.reflection_agent {
            Some(r) => r,
            None => return,
        };

        // Check buffer size (Read lock)
        // Maintain a safety buffer of 20 recent experiments for statistical continuity
        let (count, snapshot, batch_size) = {
            let guard = memory.read().await;
            if guard.len() >= 70 {
                (guard.len(), guard.get_experiments_oldest(50), 50)
            } else {
                (0, Vec::new(), 0)
            }
        };

        if count >= 70 {
            tracing::info!(
                "Consolidating evolution memory ({} items, batch execution)...",
                batch_size
            );

            // 1. Extract rules using LLM
            let consolidation_result = reflection.consolidate_memory(&snapshot).await;

            let success = consolidation_result.is_ok();

            match consolidation_result {
                Ok(rules) => {
                    if !rules.is_empty() {
                        // 2. Save rules to persistence
                        if let Some(store) = &self.persistence_layer {
                            for rule in &rules {
                                if let Err(e) = store.save_rule(tenant_id, rule).await {
                                    tracing::warn!("Failed to save optimization rule: {}", e);
                                }
                            }
                        }

                        tracing::info!(
                            "Consolidated memory into {} rules. Draining batch.",
                            rules.len()
                        );
                    } else {
                        tracing::info!(
                            "Consolidation completed with no patterns found. Draining batch."
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Consolidation failed: {}", e);
                }
            }

            // 3. Manage Memory (Write lock)
            let mut guard = memory.write().await;

            // If success (even if no rules), remove the processed batch
            if success {
                guard.drain_oldest(batch_size);
            }

            // Overflow Protection: Hard cap at 100 to prevent DoS
            if guard.len() > 100 {
                let excess = guard.len() - 100;
                tracing::warn!(
                    "Memory overflow ({} > 100). Evicting {} oldest items.",
                    guard.len(),
                    excess
                );
                guard.drain_oldest(excess);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[derive(Debug)]
    struct MockLlm;

    #[async_trait]
    impl LlmBackend for MockLlm {
        async fn complete(&self, system: &str, _prompt: &str) -> Result<String, String> {
            if system.contains("researcher") {
                Ok("Research finding: This is a detailed analysis of the topic.".to_string())
            } else if system.contains("critic") {
                Ok("Critical analysis: The main concern is validation of assumptions.".to_string())
            } else {
                Ok(
                    "Synthesized response combining all findings into a coherent answer."
                        .to_string(),
                )
            }
        }
    }

    #[async_trait]
    impl vex_llm::LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "MockLLM"
        }

        async fn is_available(&self) -> bool {
            true
        }

        async fn complete(
            &self,
            _request: vex_llm::LlmRequest,
        ) -> Result<vex_llm::LlmResponse, vex_llm::LlmError> {
            Ok(vex_llm::LlmResponse {
                content: "Mock response".to_string(),
                model: "mock".to_string(),
                tokens_used: Some(10),
                latency_ms: 10,
                trace_root: None,
            })
        }
    }

    #[tokio::test]
    async fn test_orchestrator() {
        let llm = Arc::new(MockLlm);
        let orchestrator = Orchestrator::new(llm, OrchestratorConfig::default(), None);

        let result = orchestrator
            .process("test-tenant", "What is the meaning of life?")
            .await
            .unwrap();

        assert!(!result.response.is_empty());
        assert!(result.confidence > 0.0);
        assert!(!result.agent_results.is_empty());
    }
}
