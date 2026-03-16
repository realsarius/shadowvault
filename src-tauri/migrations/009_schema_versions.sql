-- Schema version tracking table.
-- Records every migration that has been applied so the current DB schema
-- version is human-readable without inspecting _sqlx_migrations directly.
CREATE TABLE IF NOT EXISTS schema_versions (
    version    INTEGER PRIMARY KEY,
    description TEXT    NOT NULL,
    applied_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Backfill all previously applied migrations.
INSERT OR IGNORE INTO schema_versions (version, description) VALUES
    (1, 'Initial schema'),
    (2, 'Backup quality'),
    (3, 'Incremental backups'),
    (4, 'Cloud storage'),
    (5, 'OAuth cloud storage'),
    (6, 'Vaults'),
    (7, 'Backup encryption'),
    (8, 'DB indexes'),
    (9, 'Schema version tracking');
