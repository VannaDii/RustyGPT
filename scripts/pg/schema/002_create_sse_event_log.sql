CREATE TABLE IF NOT EXISTS rustygpt.sse_event_log (
    user_id UUID NOT NULL,
    sequence BIGINT NOT NULL,
    event_id TEXT NOT NULL,
    event_name TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, sequence)
);

CREATE INDEX IF NOT EXISTS idx_sse_event_log_user_created_at
    ON rustygpt.sse_event_log (user_id, created_at DESC);
