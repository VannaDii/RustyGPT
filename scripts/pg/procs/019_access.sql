-- Helper access control stored procedures
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_user_can_access(
    p_user UUID,
    p_conv UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_exists BOOLEAN;
BEGIN
    IF p_user IS NULL OR p_conv IS NULL THEN
        RETURN FALSE;
    END IF;

    SELECT EXISTS (
        SELECT 1
        FROM rustygpt.conversation_participants cp
        WHERE cp.conversation_id = p_conv
          AND cp.user_id = p_user
          AND cp.left_at IS NULL
    )
    INTO v_exists;

    RETURN COALESCE(v_exists, FALSE);
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_require_session_user()
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_user UUID;
BEGIN
    v_user := NULLIF(current_setting('app.current_user_id', true), '')::uuid;
    IF v_user IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.401: session user not set';
    END IF;
    RETURN v_user;
END;
$$;
