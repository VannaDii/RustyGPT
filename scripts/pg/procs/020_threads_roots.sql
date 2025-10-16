-- Stored procedure: list thread roots for a conversation
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_list_threads(
    p_conv UUID,
    p_after TIMESTAMPTZ DEFAULT NULL,
    p_limit INT DEFAULT 50
)
RETURNS TABLE (
    root_id UUID,
    root_author UUID,
    root_excerpt TEXT,
    created_at TIMESTAMPTZ,
    last_activity_at TIMESTAMPTZ,
    message_count BIGINT,
    participant_count BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_limit INT;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF NOT rustygpt.sp_user_can_access(v_actor, p_conv) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor not authorized for conversation';
    END IF;

    v_limit := COALESCE(NULLIF(p_limit, 0), 50);
    IF v_limit < 0 THEN
        v_limit := 50;
    END IF;

    RETURN QUERY
    WITH thread_stats AS (
        SELECT
            root.id AS root_id,
            root.author_user_id AS root_author,
            root.content,
            root.created_at,
            MAX(all_msgs.created_at) AS last_activity_at,
            COUNT(all_msgs.*) AS message_count
        FROM rustygpt.messages root
        JOIN rustygpt.messages all_msgs
            ON all_msgs.root_message_id = root.id
        WHERE root.conversation_id = p_conv
          AND root.root_message_id = root.id
          AND root.parent_message_id IS NULL
          AND (p_after IS NULL OR root.created_at > p_after)
        GROUP BY root.id, root.author_user_id, root.content, root.created_at
    ),
    participant_totals AS (
        SELECT
            cp.conversation_id,
            COUNT(*) FILTER (WHERE cp.left_at IS NULL) AS participant_count
        FROM rustygpt.conversation_participants cp
        WHERE cp.conversation_id = p_conv
        GROUP BY cp.conversation_id
    )
    SELECT
        ts.root_id,
        ts.root_author,
        left(btrim(ts.content), 240) AS root_excerpt,
        ts.created_at,
        ts.last_activity_at,
        ts.message_count,
        COALESCE(pt.participant_count, 0) AS participant_count
    FROM thread_stats ts
    LEFT JOIN participant_totals pt
        ON pt.conversation_id = p_conv
    ORDER BY ts.last_activity_at DESC, ts.created_at DESC
    LIMIT v_limit;
END;
$$;
