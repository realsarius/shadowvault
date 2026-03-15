CREATE TABLE IF NOT EXISTS sources (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('File','Directory')),
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS destinations (
    id              TEXT PRIMARY KEY,
    source_id       TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    path            TEXT NOT NULL,
    schedule_json   TEXT NOT NULL,
    retention_json  TEXT NOT NULL,
    enabled         INTEGER NOT NULL DEFAULT 1,
    last_run        TEXT,
    last_status     TEXT,
    next_run        TEXT
);

CREATE TABLE IF NOT EXISTS copy_logs (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id        TEXT NOT NULL,
    destination_id   TEXT NOT NULL,
    source_path      TEXT NOT NULL,
    destination_path TEXT NOT NULL,
    started_at       TEXT NOT NULL,
    ended_at         TEXT,
    status           TEXT NOT NULL,
    bytes_copied     INTEGER,
    files_copied     INTEGER,
    error_message    TEXT,
    trigger          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO settings VALUES
    ('run_on_startup', 'false'),
    ('minimize_to_tray', 'true'),
    ('theme', 'dark'),
    ('log_retention_days', '30'),
    ('language', 'tr');

CREATE INDEX IF NOT EXISTS idx_logs_source    ON copy_logs(source_id);
CREATE INDEX IF NOT EXISTS idx_logs_dest      ON copy_logs(destination_id);
CREATE INDEX IF NOT EXISTS idx_logs_started   ON copy_logs(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_dest_source_id ON destinations(source_id);
