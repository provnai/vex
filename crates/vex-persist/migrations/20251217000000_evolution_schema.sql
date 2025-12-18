-- Migration to add tables for Self-Correcting Genome feature

-- Table to store raw experiments (episodic memory)
CREATE TABLE IF NOT EXISTS evolution_experiments (
    id TEXT PRIMARY KEY,
    traits TEXT NOT NULL, -- JSON array of trait values
    trait_names TEXT NOT NULL, -- JSON array of trait names
    fitness_scores TEXT NOT NULL, -- JSON map of fitness components
    task_summary TEXT NOT NULL, -- Description of the task
    overall_fitness REAL NOT NULL,
    created_at DATETIME NOT NULL
);

-- Table to store consolidated semantic rules (long-term memory)
CREATE TABLE IF NOT EXISTS optimization_rules (
    id TEXT PRIMARY KEY,
    rule_description TEXT NOT NULL,
    affected_traits TEXT NOT NULL, -- JSON array of trait names
    confidence REAL NOT NULL,
    source_count INTEGER NOT NULL, -- Number of experiments this rule is based on
    created_at DATETIME NOT NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_experiments_created_at ON evolution_experiments(created_at);
CREATE INDEX IF NOT EXISTS idx_rules_confidence_created ON optimization_rules(confidence DESC, created_at DESC);
