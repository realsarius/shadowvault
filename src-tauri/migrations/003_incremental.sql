-- Incremental backup flag: when enabled, only files modified since last_run are copied
ALTER TABLE destinations ADD COLUMN incremental INTEGER NOT NULL DEFAULT 0;
