-- Stored procedures and helper functions for SSE event persistence.

CREATE OR REPLACE PROCEDURE rustygpt.sp_record_sse_event(
    IN p_conversation_id UUID,
    IN p_sequence BIGINT,
    IN p_event_id TEXT,
    IN p_event_type TEXT,
    IN p_payload JSONB,
    IN p_root_message_id UUID DEFAULT NULL
)
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO rustygpt.sse_event_log (
        conversation_id,
        sequence,
        event_id,
        event_type,
        payload,
        root_message_id
    )
    VALUES (
        p_conversation_id,
        p_sequence,
        p_event_id,
        p_event_type,
        p_payload,
        p_root_message_id
    )
    ON CONFLICT (conversation_id, sequence) DO UPDATE
    SET event_id = EXCLUDED.event_id,
        event_type = EXCLUDED.event_type,
        payload = EXCLUDED.payload,
        root_message_id = EXCLUDED.root_message_id,
        created_at = NOW();
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_sse_replay(
    p_conversation_id UUID,
    p_since TIMESTAMPTZ,
    p_limit INTEGER
)
RETURNS TABLE (
    id BIGINT,
    sequence BIGINT,
    event_id TEXT,
    event_type TEXT,
    payload JSONB,
    root_message_id UUID,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT id,
           sequence,
           event_id,
           event_type,
           payload,
           root_message_id,
           created_at
    FROM rustygpt.sse_event_log
    WHERE conversation_id = p_conversation_id
      AND (
          p_since IS NULL
          OR created_at > p_since
      )
    ORDER BY created_at ASC, id ASC
    LIMIT (
        CASE
            WHEN p_limit IS NULL OR p_limit <= 0 THEN 500
            ELSE p_limit
        END
    );
$$;

CREATE OR REPLACE PROCEDURE rustygpt.sp_prune_sse_events(
    IN p_conversation_id UUID,
    IN p_retention_seconds INTEGER,
    IN p_prune_batch INTEGER,
    IN p_hard_limit INTEGER DEFAULT NULL
)
LANGUAGE plpgsql
AS $$
DECLARE
    v_effective_batch INTEGER := GREATEST(p_prune_batch, 1);
    v_cutoff TIMESTAMPTZ;
BEGIN
    IF COALESCE(p_retention_seconds, 0) > 0 THEN
        v_cutoff := clock_timestamp() - make_interval(secs => p_retention_seconds);

        WITH candidates AS (
            SELECT id
            FROM rustygpt.sse_event_log
            WHERE conversation_id = p_conversation_id
              AND created_at < v_cutoff
            ORDER BY created_at ASC
            LIMIT v_effective_batch
        )
        DELETE FROM rustygpt.sse_event_log
        WHERE id IN (SELECT id FROM candidates);
    END IF;

    IF p_hard_limit IS NOT NULL AND p_hard_limit > 0 THEN
        WITH overflow AS (
            SELECT id
            FROM rustygpt.sse_event_log
            WHERE conversation_id = p_conversation_id
            ORDER BY created_at DESC, id DESC
            OFFSET p_hard_limit
            LIMIT v_effective_batch
        )
        DELETE FROM rustygpt.sse_event_log
        WHERE id IN (SELECT id FROM overflow);
    END IF;
END;
$$;
