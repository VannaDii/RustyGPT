-- Stored procedures for streaming message chunks
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_append_message_chunk(
    p_message UUID,
    p_idx INT,
    p_content TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_msg RECORD;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF p_idx IS NULL OR p_idx < 0 THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.VALIDATION: idx must be >= 0';
    END IF;

    IF p_content IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.VALIDATION: content required';
    END IF;

    SELECT
        m.id,
        m.conversation_id
    INTO v_msg
    FROM rustygpt.messages m
    WHERE m.id = p_message;

    IF v_msg IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.404: message not found';
    END IF;

    IF NOT rustygpt.sp_user_can_access(v_actor, v_msg.conversation_id) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor not authorized for conversation';
    END IF;

    INSERT INTO rustygpt.message_chunks (message_id, idx, content)
    VALUES (p_message, p_idx, p_content)
    ON CONFLICT (message_id, idx) DO UPDATE
    SET content = EXCLUDED.content;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_list_message_chunks(
    p_message UUID,
    p_from_idx INT DEFAULT 0,
    p_limit INT DEFAULT 500
)
RETURNS SETOF rustygpt.message_chunks
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_msg RECORD;
    v_limit INT;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    SELECT
        m.id,
        m.conversation_id
    INTO v_msg
    FROM rustygpt.messages m
    WHERE m.id = p_message;

    IF v_msg IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.404: message not found';
    END IF;

    IF NOT rustygpt.sp_user_can_access(v_actor, v_msg.conversation_id) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor not authorized for conversation';
    END IF;

    v_limit := COALESCE(NULLIF(p_limit, 0), 500);
    IF v_limit < 0 THEN
        v_limit := 500;
    END IF;

    RETURN QUERY
    SELECT mc.*
    FROM rustygpt.message_chunks mc
    WHERE mc.message_id = p_message
      AND mc.idx >= GREATEST(COALESCE(p_from_idx, 0), 0)
    ORDER BY mc.idx
    LIMIT v_limit;
END;
$$;
