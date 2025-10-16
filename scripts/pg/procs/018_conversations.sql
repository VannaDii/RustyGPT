-- Conversation management stored procedures
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_create_conversation(
    p_title TEXT,
    p_is_group BOOLEAN,
    p_creator UUID
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_conversation_id UUID;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF v_actor IS DISTINCT FROM p_creator THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor must match creator';
    END IF;

    IF p_title IS NULL OR btrim(p_title) = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.VALIDATION: title required';
    END IF;

    v_conversation_id := gen_random_uuid();

    INSERT INTO rustygpt.conversations (id, title, is_group, created_by)
    VALUES (v_conversation_id, p_title, COALESCE(p_is_group, FALSE), v_actor)
    RETURNING id INTO v_conversation_id;

    INSERT INTO rustygpt.conversation_participants (conversation_id, user_id, role, left_at)
    VALUES (v_conversation_id, v_actor, 'owner', NULL)
    ON CONFLICT (conversation_id, user_id, joined_at) DO NOTHING;

    RETURN v_conversation_id;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_add_participant(
    p_conv UUID,
    p_user UUID,
    p_role rustygpt.conversation_role
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_actor_role rustygpt.conversation_role;
    v_active_membership BOOLEAN;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF NOT rustygpt.sp_user_can_access(v_actor, p_conv) THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: actor not a participant';
    END IF;

    SELECT cp.role
    INTO v_actor_role
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = p_conv
      AND cp.user_id = v_actor
      AND cp.left_at IS NULL;

    IF v_actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.403: insufficient role to add participant';
    END IF;

    SELECT EXISTS (
        SELECT 1
        FROM rustygpt.conversation_participants cp
        WHERE cp.conversation_id = p_conv
          AND cp.user_id = p_user
          AND cp.left_at IS NULL
    )
    INTO v_active_membership;

    IF v_active_membership THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P0001',
            MESSAGE = 'RGP.VALIDATION: participant already active';
    END IF;

    -- Reactivate existing membership if present
    UPDATE rustygpt.conversation_participants cp
    SET left_at = NULL,
        role = p_role,
        joined_at = now()
    WHERE cp.conversation_id = p_conv
      AND cp.user_id = p_user
      AND cp.left_at IS NOT NULL;

    IF NOT FOUND THEN
        INSERT INTO rustygpt.conversation_participants (conversation_id, user_id, role, left_at)
        VALUES (p_conv, p_user, p_role, NULL);
    END IF;
END;
$$;
