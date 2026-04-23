-- Migration: domain_path
ALTER TABLE apps ADD COLUMN domain_path TEXT;
