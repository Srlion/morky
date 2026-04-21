-- Migration: apps_volume_path
ALTER TABLE apps ADD COLUMN volume_path TEXT NOT NULL DEFAULT '';
ALTER TABLE deployments ADD COLUMN volume_path TEXT NOT NULL DEFAULT '';
