-- Migration: create_jobs
CREATE TABLE IF NOT EXISTS jobs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    payload     TEXT    NOT NULL DEFAULT '{}',
    unique_key  TEXT,
    status      TEXT    NOT NULL DEFAULT 'pending',
    attempts    INTEGER NOT NULL DEFAULT 0,
    error       TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch ()),
    started_at  INTEGER,
    finished_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_jobs_poll
    ON jobs (status, created_at) WHERE status = 'pending';

CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_unique
    ON jobs (name, unique_key) WHERE unique_key IS NOT NULL AND status IN ('pending', 'running');
