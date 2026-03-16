-- Backup encryption support
ALTER TABLE destinations ADD COLUMN encrypt INTEGER NOT NULL DEFAULT 0;
ALTER TABLE destinations ADD COLUMN encrypt_password_enc TEXT;
ALTER TABLE destinations ADD COLUMN encrypt_salt TEXT;
