-- Rate limiting support tables
SET search_path TO rustygpt, public;

CREATE TABLE IF NOT EXISTS rustygpt.rate_limit_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    algorithm TEXT NOT NULL DEFAULT 'gcra',
    params JSONB NOT NULL DEFAULT '{}'::JSONB,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(name) <> '')
);

CREATE TABLE IF NOT EXISTS rustygpt.rate_limit_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_id UUID NOT NULL REFERENCES rustygpt.rate_limit_profiles(id) ON DELETE CASCADE,
    method TEXT NOT NULL,
    path_pattern TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (method, path_pattern),
    CHECK (btrim(method) <> ''),
    CHECK (method = upper(method)),
    CHECK (path_pattern <> '')
);

CREATE TABLE IF NOT EXISTS rustygpt.message_rate_limits (
    user_id UUID NOT NULL,
    conversation_id UUID NOT NULL,
    tat TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, conversation_id)
);
