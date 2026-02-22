-- Add result storage column to jobs table
-- Stores the JSON output of a completed job so callers can poll for results
ALTER TABLE jobs ADD COLUMN result JSON;

-- Add tenant_id index for tenant-scoped job queries  
CREATE INDEX IF NOT EXISTS idx_jobs_tenant ON jobs(tenant_id);
