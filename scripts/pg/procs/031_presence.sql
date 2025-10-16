-- Stored procedures for presence and typing states
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_heartbeat(
    p_user UUID,
    p_status TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_status rustygpt.presence_status := 'online';
BEGIN
    v_actor := rustygpt.sp_require_session_user();
    IF v_actor <> p_user THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    IF p_status IS NOT NULL THEN
        BEGIN
            v_status := p_status::rustygpt.presence_status;
        EXCEPTION WHEN others THEN
            RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: invalid status';
        END;
    END IF;

    INSERT INTO rustygpt.presence (user_id, last_seen_at, status)
    VALUES (p_user, now(), v_status)
    ON CONFLICT (user_id) DO UPDATE
        SET last_seen_at = EXCLUDED.last_seen_at,
            status = EXCLUDED.status;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_set_typing(
    p_conversation UUID,
    p_root UUID,
    p_user UUID,
    p_seconds INTEGER
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_expires TIMESTAMPTZ;
BEGIN
    v_actor := rustygpt.sp_require_session_user();
    IF v_actor <> p_user THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    IF NOT rustygpt.sp_user_can_access(p_user, p_conversation) THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: not a participant';
    END IF;

    IF p_seconds IS NULL OR p_seconds <= 0 THEN
        DELETE FROM rustygpt.typing_states
        WHERE conversation_id = p_conversation
          AND user_id = p_user
          AND root_message_id = p_root;
        RETURN;
    END IF;

    v_expires := now() + make_interval(secs => p_seconds);

    INSERT INTO rustygpt.typing_states (
        conversation_id,
        root_message_id,
        user_id,
        started_at,
        expires_at
    ) VALUES (
        p_conversation,
        p_root,
        p_user,
        now(),
        v_expires
    )
    ON CONFLICT (conversation_id, root_message_id, user_id) DO UPDATE
        SET started_at = EXCLUDED.started_at,
            expires_at = EXCLUDED.expires_at;
END;
$$;
