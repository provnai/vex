use vex_core::{GenomeExperiment, OptimizationRule};
use vex_persist::{EvolutionStore, SqliteEvolutionStore};
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;

#[tokio::test]
async fn test_evolution_persistence() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup DB
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await?;
    
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

    let store = SqliteEvolutionStore::new(pool);

    // 2. Create Experiment
    let mut experiment = GenomeExperiment::from_raw(
        vec![0.1, 0.2, 0.3],
        vec!["t1".to_string(), "t2".to_string(), "t3".to_string()],
        0.85,
        "Test Persistence"
    );
    experiment.fitness_scores.insert("latency".to_string(), 0.9);

    // 3. Save Experiment
    store.save_experiment(&experiment).await?;

    // 4. Create Rule
    let rule = OptimizationRule::new(
        "Test Rule".to_string(),
        vec!["t1".to_string()],
        0.95,
        10
    );

    // 5. Save Rule
    store.save_rule(&rule).await?;

    // 6. Verify Rule
    let loaded_rules = store.load_rules().await?;
    assert_eq!(loaded_rules.len(), 1);
    assert_eq!(loaded_rules[0].rule_description, "Test Rule");

    // 7. Load Experiment
    let loaded = store.load_recent(10).await?;
    
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, experiment.id);
    assert_eq!(loaded[0].traits, vec![0.1, 0.2, 0.3]);
    assert_eq!(loaded[0].task_summary, "Test Persistence");
    assert!((loaded[0].overall_fitness - 0.85).abs() < 1e-6);

    Ok(())
}
