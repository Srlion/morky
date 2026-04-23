-- Migration: drop_jobs_unique_key
DROP INDEX IF EXISTS idx_jobs_unique;
ALTER TABLE jobs DROP COLUMN unique_key;
