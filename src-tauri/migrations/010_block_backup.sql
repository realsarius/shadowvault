-- Track backup level and snapshot ID in copy logs
ALTER TABLE copy_logs ADD COLUMN backup_level TEXT;
ALTER TABLE copy_logs ADD COLUMN snapshot_id TEXT;
