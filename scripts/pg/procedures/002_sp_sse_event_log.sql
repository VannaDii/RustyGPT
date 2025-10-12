-- Stored procedures and helper functions for SSE event persistence.

CREATE OR REPLACE PROCEDURE rustygpt.sp_record_sse_event(
    IN p_user_id UUID,
    IN p_sequence BIGINT,
    IN p_event_id TEXT,
    IN p_event_name TEXT,
    IN p_payload TEXT
)
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO rustygpt.sse_event_log (user_id, sequence, event_id, event_name, payload)
    VALUES (p_user_id, p_sequence, p_event_id, p_event_name, p_payload)
    ON CONFLICT (user_id, sequence) DO UPDATE
    SET event_id = EXCLUDED.event_id,
        event_name = EXCLUDED.event_name,
        payload = EXCLUDED.payload,
        created_at = NOW();
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.fn_load_recent_sse_events(
    p_user_id UUID,
    p_limit INTEGER
)
RETURNS TABLE (
    sequence BIGINT,
    event_id TEXT,
    event_name TEXT,
    payload TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT sequence, event_id, event_name, payload
    FROM rustygpt.sse_event_log
    WHERE user_id = p_user_id
    ORDER BY sequence ASC
    LIMIT p_limit;
$$;

CREATE OR REPLACE FUNCTION rustygpt.fn_load_sse_events_after(
    p_user_id UUID,
    p_last_sequence BIGINT,
    p_limit INTEGER
)
RETURNS TABLE (
    sequence BIGINT,
    event_id TEXT,
    event_name TEXT,
    payload TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT sequence, event_id, event_name, payload
    FROM rustygpt.sse_event_log
    WHERE user_id = p_user_id
      AND sequence > p_last_sequence
    ORDER BY sequence ASC
    LIMIT p_limit;
$$;

CREATE OR REPLACE PROCEDURE rustygpt.sp_prune_sse_events(
    IN p_user_id UUID,
    IN p_max_events INTEGER,
    IN p_prune_batch INTEGER
)
LANGUAGE plpgsql
AS $$
BEGIN
    WITH ranked AS (
        SELECT sequence,
               ROW_NUMBER() OVER (ORDER BY sequence DESC) AS position
        FROM rustygpt.sse_event_log
        WHERE user_id = p_user_id
    )
    DELETE FROM rustygpt.sse_event_log
    WHERE user_id = p_user_id
      AND sequence IN (
        SELECT sequence
        FROM ranked
        WHERE position > p_max_events
        LIMIT p_prune_batch
    );
END;
$$;
