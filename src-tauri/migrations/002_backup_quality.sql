-- SHA-256 checksum column for integrity verification
ALTER TABLE copy_logs ADD COLUMN checksum TEXT;

-- Exclusion patterns for destinations (.gitignore-style, stored as JSON array)
ALTER TABLE destinations ADD COLUMN exclusions_json TEXT NOT NULL DEFAULT '[]';
