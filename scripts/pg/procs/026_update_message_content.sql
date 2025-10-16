-- Stored procedure: update message content for streaming completion
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_update_message_content(
    p_message UUID,
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

    UPDATE rustygpt.messages
    SET content = p_content,
        updated_at = NOW()
    WHERE id = p_message;
END;
$$;
