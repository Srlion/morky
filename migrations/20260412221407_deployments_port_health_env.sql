-- Migration: deployments_port_health_env
ALTER TABLE deployments ADD COLUMN port INTEGER;
ALTER TABLE deployments ADD COLUMN health_check_path TEXT NOT NULL DEFAULT '';
ALTER TABLE deployments ADD COLUMN env_vars TEXT;
