-- Migration: drop_image_tag
ALTER TABLE deployments DROP COLUMN image_tag;
