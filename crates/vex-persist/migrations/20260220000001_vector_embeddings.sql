-- Migration: Add vector_embeddings table for persistent RAG support
-- Applied: 2026-02-20

CREATE TABLE IF NOT EXISTS vector_embeddings (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    vector BLOB NOT NULL, -- Store as binary f32 array
    metadata JSON NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vector_tenant ON vector_embeddings(tenant_id);
