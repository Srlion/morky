-- Migration: settings_and_domains

CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- enforce single row
    panel_domain TEXT,                     -- e.g. panel.yourdomain.com
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Seed the single row
INSERT INTO settings (id) VALUES (1);
