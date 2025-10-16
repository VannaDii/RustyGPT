-- Stored procedures for unread tracking
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_mark_thread_read(
    p_conversation UUID,
    p_root UUID,
    p_user UUID,
    p_path TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_root RECORD;
    v_path LTREE;
BEGIN
    v_actor := rustygpt.sp_require_session_user();
    IF v_actor <> p_user THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    IF NOT rustygpt.sp_user_can_access(p_user, p_conversation) THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: not a participant';
    END IF;

    SELECT m.conversation_id, m.path
    INTO v_root
    FROM rustygpt.messages m
    WHERE m.id = p_root;

    IF v_root.conversation_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: root message not found';
    END IF;

    IF v_root.conversation_id <> p_conversation THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.400: root not in conversation';
    END IF;

    IF p_path IS NOT NULL THEN
        BEGIN
            v_path := p_path::ltree;
        EXCEPTION WHEN others THEN
            RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: invalid path';
        END;
    ELSE
        SELECT MAX(path)
        INTO v_path
        FROM rustygpt.messages
        WHERE root_message_id = p_root
          AND deleted_at IS NULL;

        IF v_path IS NULL THEN
            v_path := v_root.path;
        END IF;
    END IF;

    INSERT INTO rustygpt.message_read_watermarks (
        conversation_id,
        user_id,
        root_message_id,
        last_read_path,
        last_read_at
    ) VALUES (
        p_conversation,
        p_user,
        p_root,
        v_path,
        now()
    )
    ON CONFLICT (conversation_id, user_id, root_message_id) DO UPDATE
        SET last_read_path = CASE
                WHEN EXCLUDED.last_read_path > rustygpt.message_read_watermarks.last_read_path
                    THEN EXCLUDED.last_read_path
                ELSE rustygpt.message_read_watermarks.last_read_path
            END,
            last_read_at = EXCLUDED.last_read_at;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_get_unread_summary(
    p_conversation UUID,
    p_user UUID
)
RETURNS TABLE (
    root_id UUID,
    unread INTEGER
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
BEGIN
    v_actor := rustygpt.sp_require_session_user();
    IF v_actor <> p_user THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    IF NOT rustygpt.sp_user_can_access(p_user, p_conversation) THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: not a participant';
    END IF;

    RETURN QUERY
    SELECT
        m.root_message_id AS root_id,
        SUM(
            CASE
                WHEN mw.last_read_path IS NULL OR m.path > mw.last_read_path THEN 1
                ELSE 0
            END
        )::INTEGER AS unread
    FROM rustygpt.messages m
    JOIN rustygpt.conversation_participants cp
      ON cp.conversation_id = m.conversation_id
     AND cp.user_id = p_user
     AND cp.left_at IS NULL
    LEFT JOIN rustygpt.message_read_watermarks mw
      ON mw.conversation_id = m.conversation_id
     AND mw.user_id = p_user
     AND mw.root_message_id = m.root_message_id
    WHERE m.conversation_id = p_conversation
      AND m.deleted_at IS NULL
      AND m.created_at >= cp.joined_at
    GROUP BY m.root_message_id;
END;
$$;
