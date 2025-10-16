-- Stored procedures for conversation membership and invitations
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_add_participant(
    p_conversation UUID,
    p_user UUID,
    p_role TEXT DEFAULT 'member'
)
RETURNS rustygpt.conversation_role
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_actor_role rustygpt.conversation_role;
    v_role rustygpt.conversation_role;
    v_existing rustygpt.conversation_participants%ROWTYPE;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF p_user IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: user required';
    END IF;

    SELECT role
    INTO v_actor_role
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = p_conversation
      AND cp.user_id = v_actor
      AND cp.left_at IS NULL;

    IF v_actor_role IS NULL OR v_actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: insufficient role';
    END IF;

    v_role := COALESCE(p_role::rustygpt.conversation_role, 'member');

    SELECT *
    INTO v_existing
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = p_conversation
      AND cp.user_id = p_user
    ORDER BY cp.joined_at DESC
    LIMIT 1;

    IF v_existing.user_id IS NOT NULL AND v_existing.left_at IS NULL THEN
        IF v_existing.role <> v_role THEN
            UPDATE rustygpt.conversation_participants
            SET role = v_role
            WHERE conversation_id = p_conversation
              AND user_id = p_user
              AND left_at IS NULL;
        END IF;
        RETURN v_role;
    END IF;

    INSERT INTO rustygpt.conversation_participants (
        conversation_id,
        user_id,
        role,
        joined_at
    ) VALUES (
        p_conversation,
        p_user,
        v_role,
        now()
    );

    RETURN v_role;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_remove_participant(
    p_conversation UUID,
    p_user UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_actor_role rustygpt.conversation_role;
    v_target rustygpt.conversation_participants%ROWTYPE;
    v_owner_count INTEGER;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    SELECT role
    INTO v_actor_role
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = p_conversation
      AND cp.user_id = v_actor
      AND cp.left_at IS NULL;

    IF v_actor_role IS NULL OR v_actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: insufficient role';
    END IF;

    SELECT *
    INTO v_target
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = p_conversation
      AND cp.user_id = p_user
      AND cp.left_at IS NULL;

    IF v_target.user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: participant not found';
    END IF;

    IF v_target.role = 'owner' THEN
        SELECT COUNT(*)
        INTO v_owner_count
        FROM rustygpt.conversation_participants cp
        WHERE cp.conversation_id = p_conversation
          AND cp.role = 'owner'
          AND cp.left_at IS NULL
          AND cp.user_id <> p_user;

        IF v_owner_count = 0 THEN
            RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.409: cannot remove last owner';
        END IF;
    END IF;

    UPDATE rustygpt.conversation_participants
    SET left_at = now()
    WHERE conversation_id = p_conversation
      AND user_id = p_user
      AND left_at IS NULL;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_create_invite(
    p_conversation UUID,
    p_email CITEXT,
    p_role TEXT,
    p_ttl_seconds INTEGER
)
RETURNS TABLE (
    token TEXT,
    expires_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_actor_role rustygpt.conversation_role;
    v_role rustygpt.conversation_role;
    v_token TEXT;
    v_expiry TIMESTAMPTZ;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF p_email IS NULL OR btrim(p_email::TEXT) = '' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: email required';
    END IF;

    IF COALESCE(p_ttl_seconds, 0) <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: ttl must be positive';
    END IF;

    SELECT role
    INTO v_actor_role
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = p_conversation
      AND cp.user_id = v_actor
      AND cp.left_at IS NULL;

    IF v_actor_role IS NULL OR v_actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: insufficient role';
    END IF;

    v_role := COALESCE(p_role::rustygpt.conversation_role, 'member');
    v_token := encode(gen_random_bytes(24), 'base64');
    v_expiry := now() + make_interval(secs => p_ttl_seconds);

    IF EXISTS (
        SELECT 1
        FROM rustygpt.conversation_invites ci
        WHERE ci.conversation_id = p_conversation
          AND ci.invited_email = p_email
          AND ci.role = v_role
          AND ci.revoked_at IS NULL
          AND ci.accepted_at IS NULL
          AND ci.expires_at > now()
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.409: active invite exists';
    END IF;

    INSERT INTO rustygpt.conversation_invites (
        conversation_id,
        invited_email,
        role,
        invited_by,
        token,
        expires_at
    ) VALUES (
        p_conversation,
        p_email,
        v_role,
        v_actor,
        v_token,
        v_expiry
    );

    RETURN QUERY SELECT v_token, v_expiry;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_accept_invite(
    p_token TEXT,
    p_user UUID
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_invite rustygpt.conversation_invites%ROWTYPE;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    IF v_actor <> p_user THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    SELECT *
    INTO v_invite
    FROM rustygpt.conversation_invites
    WHERE token = p_token;

    IF v_invite.id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: invite not found';
    END IF;

    IF v_invite.revoked_at IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.410: invite revoked';
    END IF;

    IF v_invite.accepted_at IS NOT NULL THEN
        RETURN v_invite.conversation_id;
    END IF;

    IF v_invite.expires_at <= now() THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.410: invite expired';
    END IF;

    UPDATE rustygpt.conversation_participants
    SET role = v_invite.role,
        joined_at = now(),
        left_at = NULL
    WHERE conversation_id = v_invite.conversation_id
      AND user_id = p_user
      AND left_at IS NULL;

    IF NOT FOUND THEN
        INSERT INTO rustygpt.conversation_participants (
            conversation_id,
            user_id,
            role,
            joined_at
        ) VALUES (
            v_invite.conversation_id,
            p_user,
            v_invite.role,
            now()
        );
    END IF;

    UPDATE rustygpt.conversation_invites
    SET accepted_by = p_user,
        accepted_at = now()
    WHERE id = v_invite.id;

    RETURN v_invite.conversation_id;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_revoke_invite(
    p_token TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_actor UUID;
    v_actor_role rustygpt.conversation_role;
    v_invite rustygpt.conversation_invites%ROWTYPE;
BEGIN
    v_actor := rustygpt.sp_require_session_user();

    SELECT *
    INTO v_invite
    FROM rustygpt.conversation_invites
    WHERE token = p_token;

    IF v_invite.id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: invite not found';
    END IF;

    SELECT role
    INTO v_actor_role
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = v_invite.conversation_id
      AND cp.user_id = v_actor
      AND cp.left_at IS NULL;

    IF v_actor_role IS NULL OR v_actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: insufficient role';
    END IF;

    UPDATE rustygpt.conversation_invites
    SET revoked_at = now()
    WHERE id = v_invite.id;
END;
$$;
