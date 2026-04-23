-- Migration: container_logs_enabled
ALTER TABLE apps ADD COLUMN container_logs_enabled BOOLEAN NOT NULL DEFAULT 1;
