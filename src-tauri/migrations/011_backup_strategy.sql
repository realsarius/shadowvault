-- Level 1 (incremental) scheduling fields
ALTER TABLE destinations ADD COLUMN level1_enabled INTEGER DEFAULT 0;
ALTER TABLE destinations ADD COLUMN level1_schedule_json TEXT;
ALTER TABLE destinations ADD COLUMN level1_type TEXT DEFAULT 'Cumulative';
