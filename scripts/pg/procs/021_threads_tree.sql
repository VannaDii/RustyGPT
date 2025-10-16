-- Stored procedure: fetch thread subtree as ordered slice
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_get_thread_subtree(
    p_root UUID,
    p_cursor_path TEXT DEFAULT NULL,
    p_limit INT DEFAULT 200
)
RETURNS TABLE (
    id UUID,
    root_id UUID,
    parent_id UUID,
    conversation_id UUID,
    author_user_id UUID,
    role rustygpt.message_role,
    content TEXT,
    path TEXT,
    depth INT,
    created_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_root RECORD;
    v_limit INT;
    v_cursor LTREE;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    SELECT
        m.id,
        m.conversation_id
    INTO v_root
    FROM rustygpt.messages m
    WHERE m.id = p_root
      AND m.root_message_id = m.id;

    IF v_root IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.404: thread root not found';
    END IF;

    IF NOT rustygpt.sp_user_can_access(v_actor, v_root.conversation_id) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor not authorized for conversation';
    END IF;

    v_limit := COALESCE(NULLIF(p_limit, 0), 200);
    IF v_limit < 0 THEN
        v_limit := 200;
    END IF;

    IF p_cursor_path IS NOT NULL THEN
        BEGIN
            v_cursor := p_cursor_path::ltree;
        EXCEPTION WHEN OTHERS THEN
            RAISE EXCEPTION USING
                ERRCODE = 'P0001',
                MESSAGE = 'RGP.VALIDATION: invalid cursor_path';
        END;
    END IF;

    RETURN QUERY
    SELECT
        msg.id,
        msg.root_message_id AS root_id,
        msg.parent_message_id AS parent_id,
        msg.conversation_id,
        msg.author_user_id,
        msg.role,
        msg.content,
        msg.path::TEXT AS path,
        msg.depth,
        msg.created_at
    FROM rustygpt.messages msg
    WHERE msg.root_message_id = p_root
      AND (v_cursor IS NULL OR msg.path > v_cursor)
    ORDER BY msg.path
    LIMIT v_limit;
END;
$$;
