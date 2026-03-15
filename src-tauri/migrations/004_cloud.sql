-- Cloud destination support
ALTER TABLE destinations ADD COLUMN destination_type TEXT NOT NULL DEFAULT 'Local';
ALTER TABLE destinations ADD COLUMN cloud_config_json TEXT;
