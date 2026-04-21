-- Migration: cleanup_settings
-- Migration: cleanup_settings
CREATE TABLE cleanup_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    auto_cleanup_enabled BOOLEAN NOT NULL DEFAULT 0,
    -- each target can be toggled independently
    clean_containers BOOLEAN NOT NULL DEFAULT 1,
    clean_images BOOLEAN NOT NULL DEFAULT 1,
    clean_volumes BOOLEAN NOT NULL DEFAULT 0,
    clean_buildkit BOOLEAN NOT NULL DEFAULT 0,
    buildkit_keep_storage_gb REAL NOT NULL DEFAULT 2.0,
    -- schedule
    cleanup_interval_hours INTEGER NOT NULL DEFAULT 24,
    last_cleanup_at INTEGER,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

INSERT INTO cleanup_settings (id) VALUES (1);
