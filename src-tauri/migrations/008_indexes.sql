-- Performance indexes for copy_logs queries
CREATE INDEX IF NOT EXISTS idx_copy_logs_source_started
    ON copy_logs (source_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_copy_logs_started
    ON copy_logs (started_at DESC);
