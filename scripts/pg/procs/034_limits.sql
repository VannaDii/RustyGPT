-- Stored procedures for conversational rate limits
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_user_can_post(
    p_user UUID,
    p_conversation UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_allowed BOOLEAN := TRUE;
    v_recent BIGINT;
    v_limit CONSTANT INTEGER := 30; -- placeholder until configurable limits are wired
BEGIN
    v_actor := rustygpt.sp_require_session_user();
    IF v_actor <> p_user THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    IF NOT rustygpt.sp_user_can_access(p_user, p_conversation) THEN
        RETURN FALSE;
    END IF;

    SELECT COUNT(*)
    INTO v_recent
    FROM rustygpt.messages m
    WHERE m.conversation_id = p_conversation
      AND m.author_user_id = p_user
      AND m.created_at >= now() - INTERVAL '1 minute';

    IF v_recent >= v_limit THEN
        v_allowed := FALSE;
    END IF;

    RETURN v_allowed;
END;
$$;
