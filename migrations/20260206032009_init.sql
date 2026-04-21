-- Migration: init
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    -- null for OAuth-only users (e.g. GitHub)
    password_hash TEXT,
    email_verified_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch ())
);

CREATE TABLE sessions (
    -- random token stored in cookie
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    ip TEXT,
    user_agent TEXT,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch ())
);

CREATE INDEX idx_sessions_user ON sessions (user_id);

CREATE INDEX idx_sessions_expires ON sessions (expires_at);

CREATE TABLE projects (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    env_vars TEXT DEFAULT '',
    created_at INTEGER NOT NULL DEFAULT (unixepoch ())
);

CREATE TABLE git_sources (
    id INTEGER PRIMARY KEY,
    provider TEXT NOT NULL, -- 'github'
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    provider_data TEXT NOT NULL DEFAULT '{}', -- JSON blob, each provider stores its own shape
    created_at INTEGER NOT NULL DEFAULT (unixepoch ())
);

CREATE TABLE session_store (id TEXT PRIMARY KEY, data BLOB NOT NULL);

CREATE TABLE apps (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    env_vars TEXT DEFAULT '',
    git_source_id INTEGER REFERENCES git_sources (id) ON DELETE SET NULL,
    repo TEXT,
    branch TEXT NOT NULL DEFAULT 'main',
    status TEXT NOT NULL DEFAULT 'idle',
    build_method TEXT NOT NULL DEFAULT 'railpack', -- 'railpack' | 'dockerfile'
    dockerfile_path TEXT NOT NULL DEFAULT 'Dockerfile',
    port INTEGER NOT NULL DEFAULT 3000,
    domain TEXT,
    current_deployment_id INTEGER REFERENCES deployments (id),
    created_at INTEGER NOT NULL DEFAULT (unixepoch ())
);

CREATE INDEX idx_apps_project ON apps (project_id);

CREATE TABLE deployments (
    id INTEGER PRIMARY KEY,
    app_id INTEGER NOT NULL REFERENCES apps (id) ON DELETE CASCADE,
    -- snapshot at deploy time
    commit_sha TEXT NOT NULL,
    commit_message TEXT DEFAULT '',
    branch TEXT NOT NULL,
    build_method TEXT NOT NULL,
    dockerfile_path TEXT,
    -- the docker image ref, kept for rollback
    image_tag TEXT, -- e.g. app-3:deploy-17
    status TEXT NOT NULL DEFAULT 'building',
    build_log TEXT DEFAULT '',
    error TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch ()),
    finished_at INTEGER
);

CREATE INDEX idx_deployments_app ON deployments (app_id);

CREATE INDEX idx_deployments_status ON deployments (status);
