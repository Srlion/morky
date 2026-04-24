-- Migration: proxy_ip_header
ALTER TABLE settings ADD COLUMN proxy_ip_header TEXT NOT NULL DEFAULT 'CF-Connecting-IP';
