use async_trait::async_trait;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
// Removed unused imports
use vex_llm::{LlmError, LlmProvider, LlmRequest, LlmResponse};
use vex_persist::{EvolutionStore, SqliteEvolutionStore};
use vex_runtime::{Orchestrator, OrchestratorConfig};

#[derive(Debug)]
struct MockLlm {
    #[allow(dead_code)]
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
    fn name(&self) -> &str {
        "Mock"
    }
    async fn is_available(&self) -> bool {
        true
    }
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Return a JSON rule response if prompted for consolidation
        if request
            .prompt
            .to_lowercase()
            .contains("extract universal optimization rules")
        {
            return Ok(LlmResponse {
                content: r#"[
                    {
                        "rule": "High exploration aids discovery",
                        "traits": ["exploration"],
                        "confidence": 0.9
                    }
                ]"#
                .to_string(),
                model: "mock".to_string(),
                tokens_used: Some(10),
                latency_ms: 10,
                trace_root: None,
            });
        }
        Ok(LlmResponse {
            content: "Mock response".to_string(),
            model: "mock".to_string(),
            tokens_used: Some(10),
            latency_ms: 10,
            trace_root: None,
        })
    }
}

#[tokio::test]
async fn test_consolidation_flow() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup DB
    let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await?;

    // Create tables (since we use raw store, we must init schema)
    sqlx::query(
        r#"
        CREATE TABLE evolution_experiments (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            traits TEXT NOT NULL,
            trait_names TEXT NOT NULL,
            fitness_scores TEXT NOT NULL,
            task_summary TEXT NOT NULL,
            overall_fitness REAL NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE optimization_rules (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
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
    let tenant_id = "test-tenant-123";

    // 2. Setup Orchestrator
    let llm = Arc::new(MockLlm { responses: vec![] });
    let config = OrchestratorConfig {
        enable_self_correction: true,
        ..Default::default()
    };

    let orchestrator = Orchestrator::new(llm, config, Some(store.clone()));

    // 3. Fill Memory manually (> 50)
    for _i in 0..75 {
        let _ = orchestrator.process(tenant_id, "test query").await;
    }

    // 4. Verify Rules Persistence
    // The consolidation happens async at end of process.
    // Check store for rules.
    let rules = store.load_rules(tenant_id).await?;

    // We expect at least one consolidation event occurred
    assert!(
        !rules.is_empty(),
        "Rules should be generated after 75 iterations"
    );
    assert_eq!(rules[0].rule_description, "High exploration aids discovery");

    Ok(())
}
