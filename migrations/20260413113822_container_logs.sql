-- Migration: container_logs
CREATE TABLE container_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id INTEGER NOT NULL REFERENCES deployments(id) ON DELETE CASCADE,
    line TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_container_logs_deployment_id ON container_logs(deployment_id);
