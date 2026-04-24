-- Migration: build_log_lines
CREATE TABLE build_log_lines (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id INTEGER NOT NULL REFERENCES deployments(id) ON DELETE CASCADE,
    line TEXT NOT NULL
);

CREATE INDEX idx_build_log_lines_deployment ON build_log_lines(deployment_id);

ALTER TABLE deployments DROP COLUMN build_log;
