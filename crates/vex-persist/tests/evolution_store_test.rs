use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use vex_core::{GenomeExperiment, OptimizationRule};
use vex_persist::{EvolutionStore, SqliteEvolutionStore};

#[tokio::test]
async fn test_evolution_isolation() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup DB
    let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await?;

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

    let store = SqliteEvolutionStore::new(pool);
    let t1 = "tenant-1";
    let t2 = "tenant-2";

    // 2. Create and Save Experiment for T1
    let mut experiment = GenomeExperiment::from_raw(
        vec![0.1, 0.2, 0.3],
        vec!["t1".to_string(), "t2".to_string(), "t3".to_string()],
        0.85,
        "T1 Persistence",
    );
    experiment.fitness_scores.insert("latency".to_string(), 0.9);
    store.save_experiment(t1, &experiment).await?;

    // 3. Create and Save Experiment for T2
    let experiment2 = GenomeExperiment::from_raw(
        vec![0.9, 0.8, 0.7],
        vec!["t1".to_string()],
        0.95,
        "T2 Persistence",
    );
    store.save_experiment(t2, &experiment2).await?;

    // 4. Create and Save Rules
    let rule1 = OptimizationRule::new("T1 Rule".to_string(), vec!["t1".to_string()], 0.95, 10);
    store.save_rule(t1, &rule1).await?;

    let rule2 = OptimizationRule::new("T2 Rule".to_string(), vec!["t2".to_string()], 0.99, 5);
    store.save_rule(t2, &rule2).await?;

    // 5. Verify Isolation - Rules
    let rules1 = store.load_rules(t1).await?;
    let rules2 = store.load_rules(t2).await?;
    assert_eq!(rules1.len(), 1);
    assert_eq!(rules2.len(), 1);
    assert_eq!(rules1[0].rule_description, "T1 Rule");
    assert_eq!(rules2[0].rule_description, "T2 Rule");

    // 6. Verify Isolation - Experiments
    let experiments1 = store.load_recent(t1, 10).await?;
    let experiments2 = store.load_recent(t2, 10).await?;

    assert_eq!(experiments1.len(), 1);
    assert_eq!(experiments2.len(), 1);
    assert_eq!(experiments1[0].id, experiment.id);
    assert_eq!(experiments2[0].id, experiment2.id);
    assert_eq!(experiments1[0].task_summary, "T1 Persistence");
    assert_eq!(experiments2[0].task_summary, "T2 Persistence");

    Ok(())
}
