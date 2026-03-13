-- Migration: Add GIN index to vector_embeddings metadata for fast JSONB filtering
-- Applied: 2026-03-12

CREATE INDEX IF NOT EXISTS idx_vector_metadata_gin ON vector_embeddings USING GIN (metadata);
