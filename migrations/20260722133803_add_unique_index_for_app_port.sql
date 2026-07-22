-- Migration: add_unique_index_for_app_port
CREATE UNIQUE INDEX IF NOT EXISTS idx_apps_port ON apps(port);
