-- Separate last_run/next_run tracking for Level 1 backups
ALTER TABLE destinations ADD COLUMN level1_last_run TEXT;
ALTER TABLE destinations ADD COLUMN level1_next_run TEXT;
