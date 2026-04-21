-- Migration: port_unique
CREATE UNIQUE INDEX apps_port_unique ON apps(port);
CREATE UNIQUE INDEX apps_domain_unique ON apps(domain) WHERE domain IS NOT NULL;
