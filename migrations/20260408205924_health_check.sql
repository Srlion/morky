-- Migration: health_check
ALTER TABLE apps ADD COLUMN health_check_path TEXT NOT NULL DEFAULT '';
