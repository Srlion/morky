CREATE TABLE build_log_lines (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id INTEGER NOT NULL REFERENCES deployments(id),
    line TEXT NOT NULL
);

CREATE INDEX idx_build_log_lines_deployment_id ON build_log_lines(deployment_id);
