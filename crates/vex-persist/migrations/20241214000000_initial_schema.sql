-- Add migration script here
CREATE TABLE IF NOT EXISTS kv_store (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    created_at INTEGER,
    updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY NOT NULL,
    parent_id TEXT,
    fitness REAL NOT NULL DEFAULT 0.0,
    generation INTEGER NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL, -- JSON
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_agents_parent ON agents(parent_id);

CREATE TABLE IF NOT EXISTS contexts (
    packet_id TEXT PRIMARY KEY NOT NULL,
    data BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER 
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    diff TEXT, -- JSON
    previous_hash TEXT,
    current_hash TEXT NOT NULL,
    signature TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_audit_entity ON audit_logs(entity_id);
