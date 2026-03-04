-- Add result column to jobs table (Postgres)
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS result TEXT;
