-- Stored procedure: reply to a message within a thread
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_reply_message(
    p_parent_msg UUID,
    p_author UUID,
    p_role rustygpt.message_role DEFAULT 'user',
    p_content TEXT
)
RETURNS TABLE (
    message_id UUID,
    root_id UUID,
    conversation_id UUID,
    parent_id UUID,
    depth INT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_member UUID;
    v_parent RECORD;
    v_message_id UUID;
    v_path LTREE;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF p_author IS NOT NULL AND v_actor IS DISTINCT FROM p_author THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor must match author';
    END IF;

    IF p_content IS NULL OR btrim(p_content) = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.VALIDATION: content required';
    END IF;

    SELECT
        m.id,
        m.conversation_id,
        m.root_message_id,
        m.path
    INTO v_parent
    FROM rustygpt.messages m
    WHERE m.id = p_parent_msg;

    IF v_parent IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.404: parent message not found';
    END IF;

    v_member := COALESCE(p_author, v_actor);

    IF NOT rustygpt.sp_user_can_access(v_member, v_parent.conversation_id) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: author not a participant';
    END IF;

    IF NOT rustygpt.sp_user_can_post(v_member, v_parent.conversation_id) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.429: message rate limit exceeded';
    END IF;

    v_message_id := gen_random_uuid();
    v_path := v_parent.path || rustygpt.uuid_to_label(v_message_id);

    INSERT INTO rustygpt.messages (
        id,
        conversation_id,
        parent_message_id,
        root_message_id,
        author_user_id,
        role,
        content,
        path
    )
    VALUES (
        v_message_id,
        v_parent.conversation_id,
        v_parent.id,
        v_parent.root_message_id,
        p_author,
        p_role,
        p_content,
        v_path
    );

    RETURN QUERY SELECT v_message_id, v_parent.root_message_id, v_parent.conversation_id, v_parent.id, nlevel(v_path);
END;
$$;
