-- VEX Protocol v0.3.0 — PostgreSQL Initial Schema
-- Migrated from SQLite with Postgres-native types

CREATE TABLE IF NOT EXISTS kv_store (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    created_at BIGINT,
    updated_at BIGINT
);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY NOT NULL,
    parent_id TEXT,
    fitness DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    generation BIGINT NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agents_parent ON agents(parent_id);

CREATE TABLE IF NOT EXISTS contexts (
    packet_id TEXT PRIMARY KEY NOT NULL,
    data BYTEA NOT NULL,
    created_at BIGINT NOT NULL,
    expires_at BIGINT
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    diff TEXT,
    previous_hash TEXT,
    current_hash TEXT NOT NULL,
    signature TEXT,
    created_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_logs(entity_id);
