CREATE TABLE jobs (
    id TEXT PRIMARY KEY NOT NULL,
    job_type TEXT NOT NULL,
    payload JSON NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    priority INTEGER NOT NULL DEFAULT 0,
    run_at DATETIME NOT NULL,
    locked_at DATETIME,
    locked_by TEXT,
    retries INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_jobs_processing ON jobs(status, run_at);
CREATE INDEX idx_jobs_locked ON jobs(locked_at) WHERE locked_at IS NOT NULL;
