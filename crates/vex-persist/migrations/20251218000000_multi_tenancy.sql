-- Migration: Add tenant_id to support multi-tenancy across all stores

-- 1. Agents table
ALTER TABLE agents ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
CREATE INDEX idx_agents_tenant ON agents(tenant_id);

-- 2. Contexts table
ALTER TABLE contexts ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
CREATE INDEX idx_contexts_tenant ON contexts(tenant_id);

-- 3. Audit logs
ALTER TABLE audit_logs ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
CREATE INDEX idx_audit_limit ON audit_logs(tenant_id);

-- 4. Evolution experiments
ALTER TABLE evolution_experiments ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
CREATE INDEX idx_evolution_tenant ON evolution_experiments(tenant_id);

-- 5. Optimization rules
ALTER TABLE optimization_rules ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
CREATE INDEX idx_rules_tenant ON optimization_rules(tenant_id);

-- 6. Jobs (Queue)
ALTER TABLE jobs ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
CREATE INDEX idx_jobs_tenant ON jobs(tenant_id);

-- 7. KV store
-- Note: KV store uses key prefixing in code, but adding tenant_id column for future-proofing or unified queries
ALTER TABLE kv_store ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
