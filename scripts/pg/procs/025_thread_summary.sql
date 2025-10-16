-- Stored procedure: fetch summary for a specific thread root
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_get_thread_summary(
    p_root UUID
)
RETURNS TABLE (
    conversation_id UUID,
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
    v_conversation UUID;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    SELECT m.conversation_id
    INTO v_conversation
    FROM rustygpt.messages m
    WHERE m.id = p_root
      AND m.root_message_id = m.id;

    IF v_conversation IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.404: thread root not found';
    END IF;

    IF NOT rustygpt.sp_user_can_access(v_actor, v_conversation) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor not authorized for conversation';
    END IF;

    RETURN QUERY
    WITH root AS (
        SELECT *
        FROM rustygpt.messages
        WHERE id = p_root
    ),
    stats AS (
        SELECT
            MAX(created_at) AS last_activity_at,
            COUNT(*) AS message_count
        FROM rustygpt.messages
        WHERE root_message_id = p_root
    ),
    participants AS (
        SELECT COUNT(*) AS participant_count
        FROM rustygpt.conversation_participants cp
        WHERE cp.conversation_id = v_conversation
          AND cp.left_at IS NULL
    )
    SELECT
        v_conversation,
        root.id,
        root.author_user_id,
        LEFT(BTRIM(root.content), 240),
        root.created_at,
        stats.last_activity_at,
        stats.message_count,
        COALESCE(participants.participant_count, 0)
    FROM root
    CROSS JOIN stats
    LEFT JOIN participants ON TRUE;
END;
$$;
