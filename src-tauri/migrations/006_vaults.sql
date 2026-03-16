CREATE TABLE IF NOT EXISTS vaults (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    algorithm   TEXT NOT NULL DEFAULT 'AES-256-GCM',
    vault_path  TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    last_opened TEXT
);
