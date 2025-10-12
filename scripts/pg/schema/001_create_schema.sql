-- Core schema bootstrap for RustyGPT
CREATE SCHEMA IF NOT EXISTS rustygpt;

CREATE TABLE IF NOT EXISTS rustygpt.migrations (
    id SERIAL PRIMARY KEY,
    script TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS rustygpt.feature_flags (
    name TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
