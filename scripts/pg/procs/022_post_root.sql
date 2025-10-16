-- Stored procedure: create root thread message
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_post_root_message(
    p_conv UUID,
    p_author UUID,
    p_role rustygpt.message_role DEFAULT 'user',
    p_content TEXT
)
RETURNS TABLE (
    message_id UUID,
    root_id UUID,
    conversation_id UUID,
    depth INT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_member UUID;
    v_message_id UUID;
    v_path LTREE;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF p_author IS NOT NULL AND v_actor IS DISTINCT FROM p_author THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor must match author';
    END IF;

    v_member := COALESCE(p_author, v_actor);

    IF NOT rustygpt.sp_user_can_access(v_member, p_conv) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: author not a participant';
    END IF;

    IF p_content IS NULL OR btrim(p_content) = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.VALIDATION: content required';
    END IF;

    PERFORM 1
    FROM rustygpt.conversations c
    WHERE c.id = p_conv;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.404: conversation not found';
    END IF;

    v_message_id := gen_random_uuid();
    v_path := rustygpt.uuid_to_label(v_message_id);

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
        p_conv,
        NULL,
        v_message_id,
        p_author,
        p_role,
        p_content,
        v_path
    );

    RETURN QUERY SELECT v_message_id, v_message_id, p_conv, 1;
END;
$$;
