CREATE TABLE IF NOT EXISTS rustygpt.sse_event_log (
    id BIGSERIAL PRIMARY KEY,
    conversation_id UUID NOT NULL,
    sequence BIGINT NOT NULL,
    event_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    root_message_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        event_type IN (
            'presence.update',
            'typing.update',
            'unread.update',
            'membership.changed',
            'thread.new',
            'thread.activity',
            'message.delta',
            'message.done',
            'error'
        )
    ),
    UNIQUE (conversation_id, sequence)
);

CREATE INDEX IF NOT EXISTS idx_sse_event_log_user_created_at
    ON rustygpt.sse_event_log (conversation_id, created_at DESC);
