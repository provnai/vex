-- Evolution schema for PostgreSQL
-- Agent genome experiments and optimization rules

CREATE TABLE IF NOT EXISTS evolution_experiments (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    traits TEXT NOT NULL,
    trait_names TEXT NOT NULL,
    fitness_scores TEXT NOT NULL,
    task_summary TEXT NOT NULL,
    overall_fitness DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_evolution_tenant ON evolution_experiments(tenant_id, created_at DESC);

CREATE TABLE IF NOT EXISTS optimization_rules (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    rule_description TEXT NOT NULL,
    affected_traits TEXT NOT NULL,
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    source_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_rules_tenant ON optimization_rules(tenant_id, confidence DESC);
