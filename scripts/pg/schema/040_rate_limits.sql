-- Rate limiting support tables
SET search_path TO rustygpt, public;

CREATE TABLE IF NOT EXISTS rustygpt.rate_limit_profiles (
    name TEXT PRIMARY KEY,
    requests_per_second NUMERIC NOT NULL CHECK (requests_per_second > 0),
    burst INTEGER NOT NULL CHECK (burst > 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS rustygpt.message_rate_limits (
    user_id UUID NOT NULL,
    conversation_id UUID NOT NULL,
    tat TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, conversation_id)
);
