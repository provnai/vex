use async_trait::async_trait;
use thiserror::Error;
use vex_core::{GenomeExperiment, OptimizationRule};

#[derive(Debug, Error)]
pub enum EvolutionStoreError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[async_trait]
pub trait EvolutionStore: Send + Sync {
    /// Save an experiment to persistent storage
    async fn save_experiment(&self, experiment: &GenomeExperiment) -> Result<(), EvolutionStoreError>;

    /// Load recent experiments
    async fn load_recent(&self, limit: usize) -> Result<Vec<GenomeExperiment>, EvolutionStoreError>;

    /// Save an optimization rule (semantic lesson)
    async fn save_rule(&self, rule: &OptimizationRule) -> Result<(), EvolutionStoreError>;

    /// Load available optimization rules
    async fn load_rules(&self) -> Result<Vec<OptimizationRule>, EvolutionStoreError>;
}

/// SQL implementation of EvolutionStore
#[cfg(feature = "sqlite")]
pub struct SqliteEvolutionStore {
    pool: sqlx::SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SqliteEvolutionStore {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl EvolutionStore for SqliteEvolutionStore {
    async fn save_experiment(&self, experiment: &GenomeExperiment) -> Result<(), EvolutionStoreError> {
        let traits_json = serde_json::to_string(&experiment.traits)?;
        let trait_names_json = serde_json::to_string(&experiment.trait_names)?;
        let fitness_json = serde_json::to_string(&experiment.fitness_scores)?;
        let task_summary = &experiment.task_summary;
        let overall_fitness = experiment.overall_fitness;

        sqlx::query(
            r#"
            INSERT INTO evolution_experiments (
                id, traits, trait_names, fitness_scores, task_summary, overall_fitness, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind(experiment.id.to_string())
        .bind(traits_json)
        .bind(trait_names_json)
        .bind(fitness_json)
        .bind(task_summary)
        .bind(overall_fitness)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load_recent(&self, limit: usize) -> Result<Vec<GenomeExperiment>, EvolutionStoreError> {
        use sqlx::Row;
        
        let rows = sqlx::query(
            r#"
            SELECT 
                id, traits, trait_names, fitness_scores, task_summary, overall_fitness, created_at
            FROM evolution_experiments
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut experiments = Vec::new();
        for row in rows {
            let traits_str: String = row.try_get("traits")?;
            let trait_names_str: String = row.try_get("trait_names")?;
            let fitness_scores_str: String = row.try_get("fitness_scores")?;
            let id_str: String = row.try_get("id")?;

            let traits = serde_json::from_str(&traits_str)?;
            let trait_names = serde_json::from_str(&trait_names_str)?;
            let fitness_scores = serde_json::from_str(&fitness_scores_str)?;
            
            // We reconstruct the experiment
            let exp = GenomeExperiment {
                id: uuid::Uuid::parse_str(&id_str).unwrap_or_default(),
                traits,
                trait_names,
                fitness_scores,
                task_summary: row.try_get("task_summary")?,
                overall_fitness: row.try_get("overall_fitness")?,
                timestamp: chrono::Utc::now(), // Use current time as parsing SQL datetime can be tricky without types
                successful: row.try_get::<f64, _>("overall_fitness")? > 0.6,
            };
            experiments.push(exp);
        }

        Ok(experiments)
    }

    async fn save_rule(&self, rule: &OptimizationRule) -> Result<(), EvolutionStoreError> {
        let traits_json = serde_json::to_string(&rule.affected_traits)?;

        sqlx::query(
            r#"
            INSERT INTO optimization_rules (
                id, rule_description, affected_traits, confidence, source_count, created_at
            ) VALUES (?, ?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind(rule.id.to_string())
        .bind(&rule.rule_description)
        .bind(traits_json)
        .bind(rule.confidence)
        .bind(rule.source_experiments_count as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load_rules(&self) -> Result<Vec<OptimizationRule>, EvolutionStoreError> {
        use sqlx::Row;

        let rows = sqlx::query(
            r#"
            SELECT 
                id, rule_description, affected_traits, confidence, source_count, created_at
            FROM optimization_rules
            ORDER BY confidence DESC, created_at DESC
            LIMIT 50
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            let id_str: String = row.try_get("id")?;
            let traits_str: String = row.try_get("affected_traits")?;

            let rules_obj = OptimizationRule {
                id: uuid::Uuid::parse_str(&id_str).unwrap_or_default(),
                rule_description: row.try_get("rule_description")?,
                affected_traits: serde_json::from_str(&traits_str)?,
                confidence: row.try_get("confidence")?,
                source_experiments_count: row.try_get::<i64, _>("source_count")? as usize,
                created_at: chrono::Utc::now(), // Simplified
            };
            rules.push(rules_obj);
        }

        Ok(rules)
    }
}
