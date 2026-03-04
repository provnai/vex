-- Vector embeddings for PostgreSQL with native pgvector support
-- Requires: CREATE EXTENSION IF NOT EXISTS vector;

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS vector_embeddings (
    id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    vector vector(1536),  -- Default OpenAI ada-002 dimensions; adjust as needed
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, tenant_id)
);

-- HNSW index for fast approximate nearest-neighbor search
-- Much faster than brute-force scan used in SQLiteVectorStore
CREATE INDEX IF NOT EXISTS idx_vector_hnsw
    ON vector_embeddings
    USING hnsw (vector vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX IF NOT EXISTS idx_vector_tenant ON vector_embeddings(tenant_id);
