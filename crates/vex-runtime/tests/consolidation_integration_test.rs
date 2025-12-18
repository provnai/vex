use std::sync::Arc;
use vex_core::{GenomeExperiment, OptimizationRule};
use vex_persist::{SqliteEvolutionStore, EvolutionStore};
use vex_runtime::{Orchestrator, OrchestratorConfig};
use vex_llm::{LlmProvider, LlmRequest, LlmResponse, LlmError};
use async_trait::async_trait;
use sqlx::sqlite::SqlitePoolOptions;

#[derive(Debug)]
struct MockLlm {
    responses: Vec<String>,
}

#[async_trait]
impl vex_runtime::executor::LlmBackend for MockLlm {
    async fn complete(&self, _system: &str, _prompt: &str) -> Result<String, String> {
        Ok("Mock completion".to_string())
    }
}

#[async_trait]
impl LlmProvider for MockLlm {
    fn name(&self) -> &str { "Mock" }
    async fn is_available(&self) -> bool { true }
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Return a JSON rule response if prompted for consolidation
        if request.prompt.to_lowercase().contains("extract universal optimization rules") {
            return Ok(LlmResponse {
                content: r#"[
                    {
                        "rule": "High exploration aids discovery",
                        "traits": ["exploration"],
                        "confidence": 0.9
                    }
                ]"#.to_string(),
                model: "mock".to_string(),
                tokens_used: Some(10),
                latency_ms: 10,
            });
        }
        Ok(LlmResponse {
            content: "Mock response".to_string(),
            model: "mock".to_string(),
            tokens_used: Some(10),
            latency_ms: 10,
        })
    }
}

#[tokio::test]
async fn test_consolidation_flow() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup DB
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await?;
    
    // Create tables (since we use raw store, we must init schema)
    sqlx::query(
        r#"
        CREATE TABLE evolution_experiments (
            id TEXT PRIMARY KEY,
            traits TEXT NOT NULL,
            trait_names TEXT NOT NULL,
            fitness_scores TEXT NOT NULL,
            task_summary TEXT NOT NULL,
            overall_fitness REAL NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE optimization_rules (
            id TEXT PRIMARY KEY,
            rule_description TEXT NOT NULL,
            affected_traits TEXT NOT NULL,
            confidence REAL NOT NULL,
            source_count INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(&pool)
    .await?;

    let store = Arc::new(SqliteEvolutionStore::new(pool));

    // 2. Setup Orchestrator
    let llm = Arc::new(MockLlm { responses: vec![] });
    let mut config = OrchestratorConfig::default();
    config.enable_self_correction = true;
    
    let orchestrator = Orchestrator::new(llm, config, Some(store.clone()));

    // 3. Fill Memory manually (> 50)
    // We need to access memory. Orchestrator doesn't expose it publicly easily?
    // ReflectionAgent reads it.
    // But we want to test `maybe_consolidate_memory`.
    // Since `evolution_memory` field is private, we can't inject data directly unless we use `process` or `evolve`.
    // But running `process` 50 times is slow.
    // Inspect source: `Orchestrator` struct fields are private.
    // EXCEPT `config` is public.
    // We can't access `evolution_memory`.
    // However, `evolve_agents_self_correcting` adds to memory.
    // We can simulate evolution by calling `process`?
    // 50 calls with mock LLM is fast.
    
    for _i in 0..75 {
        let _ = orchestrator.process("test query").await;
    }

    // 4. Verify Rules Persistence
    // The consolidation happens async at end of process.
    // Check store for rules.
    let rules = store.load_rules().await?;
    
    // We expect at least one consolidation event occurred
    assert!(!rules.is_empty(), "Rules should be generated after 55 iterations");
    assert_eq!(rules[0].rule_description, "High exploration aids discovery");

    Ok(())
}
