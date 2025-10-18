-- Authentication stored procedures
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.is_setup()
RETURNS BOOLEAN
LANGUAGE sql
STABLE
AS $$
    SELECT EXISTS (SELECT 1 FROM rustygpt.users);
$$;

CREATE OR REPLACE FUNCTION rustygpt.init_setup(
    p_username TEXT,
    p_email TEXT,
    p_password_hash TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_user_id UUID;
    v_already BOOLEAN;
BEGIN
    SELECT rustygpt.is_setup() INTO v_already;
    IF v_already THEN
        RETURN FALSE;
    END IF;

    IF p_username IS NULL OR btrim(p_username) = '' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: username required';
    END IF;

    IF p_email IS NULL OR btrim(p_email) = '' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: email required';
    END IF;

    IF p_password_hash IS NULL OR btrim(p_password_hash) = '' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: password hash required';
    END IF;

    INSERT INTO rustygpt.users (username, email, password_hash)
    VALUES (p_username, p_email, p_password_hash)
    RETURNING id INTO v_user_id;

    INSERT INTO rustygpt.user_roles (user_id, role)
    VALUES
        (v_user_id, 'admin'),
        (v_user_id, 'member');

    RETURN TRUE;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_auth_login(
    p_user_id UUID,
    p_token_hash BYTEA,
    p_user_agent TEXT,
    p_ip inet,
    p_client_meta JSONB,
    p_roles TEXT[],
    p_idle_seconds INTEGER,
    p_absolute_seconds INTEGER,
    p_max_sessions INTEGER DEFAULT NULL
)
RETURNS TABLE (
    session_id UUID,
    issued_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    absolute_expires_at TIMESTAMPTZ,
    roles_snapshot TEXT[]
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_now TIMESTAMPTZ := clock_timestamp();
    v_expires TIMESTAMPTZ;
    v_absolute TIMESTAMPTZ;
    v_idle INTEGER := GREATEST(COALESCE(p_idle_seconds, 0), 1);
    v_absolute_secs INTEGER := GREATEST(COALESCE(p_absolute_seconds, 0), 1);
BEGIN
    IF p_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.401: user required';
    END IF;

    v_expires := v_now + make_interval(secs => v_idle);
    v_absolute := v_now + make_interval(secs => v_absolute_secs);

    INSERT INTO rustygpt.user_sessions (
        user_id,
        token_hash,
        issued_at,
        created_at,
        last_seen_at,
        expires_at,
        absolute_expires_at,
        user_agent,
        ip,
        client_meta,
        roles_snapshot,
        requires_rotation,
        rotation_reason
    )
    VALUES (
        p_user_id,
        p_token_hash,
        v_now,
        v_now,
        v_now,
        v_expires,
        v_absolute,
        p_user_agent,
        p_ip,
        COALESCE(p_client_meta, v_session.client_meta, '{}'::JSONB),
        COALESCE(p_roles, '{}'::TEXT[]),
        FALSE,
        NULL
    )
    RETURNING id, issued_at, expires_at, absolute_expires_at, roles_snapshot
      INTO session_id, issued_at, expires_at, absolute_expires_at, roles_snapshot;

    IF COALESCE(p_max_sessions, 0) > 0 THEN
        WITH ranked AS (
            SELECT id
            FROM rustygpt.user_sessions
            WHERE user_id = p_user_id
              AND revoked_at IS NULL
              AND id <> session_id
            ORDER BY issued_at DESC
            OFFSET GREATEST(p_max_sessions - 1, 0)
        )
        UPDATE rustygpt.user_sessions
        SET revoked_at = v_now,
            rotation_reason = COALESCE(rotation_reason, 'max_sessions')
        WHERE id IN (SELECT id FROM ranked);
    END IF;

    RETURN NEXT;
    RETURN;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_auth_refresh(
    p_session_id UUID,
    p_token_hash BYTEA,
    p_user_agent TEXT,
    p_ip inet,
    p_client_meta JSONB,
    p_roles TEXT[],
    p_idle_seconds INTEGER,
    p_now TIMESTAMPTZ DEFAULT clock_timestamp()
)
RETURNS TABLE (
    next_session_id UUID,
    user_id UUID,
    issued_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    absolute_expires_at TIMESTAMPTZ,
    roles_snapshot TEXT[]
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_session RECORD;
    v_idle INTEGER := GREATEST(COALESCE(p_idle_seconds, 0), 1);
    v_expires TIMESTAMPTZ;
BEGIN
    SELECT *
    INTO v_session
    FROM rustygpt.user_sessions
    WHERE id = p_session_id
      AND revoked_at IS NULL
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.401: session invalid';
    END IF;

    IF v_session.expires_at <= p_now THEN
        UPDATE rustygpt.user_sessions
        SET revoked_at = p_now,
            rotation_reason = 'idle_expired'
        WHERE id = v_session.id;
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.401: session expired';
    END IF;

    IF v_session.absolute_expires_at <= p_now THEN
        UPDATE rustygpt.user_sessions
        SET revoked_at = p_now,
            rotation_reason = 'absolute_expired'
        WHERE id = v_session.id;
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.401: session expired';
    END IF;

    UPDATE rustygpt.user_sessions
    SET revoked_at = p_now,
        rotated_at = p_now,
        rotated_by = v_session.user_id,
        rotation_reason = COALESCE(rotation_reason, 'refresh')
    WHERE id = v_session.id;

    v_expires := p_now + make_interval(secs => v_idle);

    INSERT INTO rustygpt.user_sessions (
        user_id,
        token_hash,
        issued_at,
        created_at,
        last_seen_at,
        expires_at,
        absolute_expires_at,
        user_agent,
        ip,
        client_meta,
        roles_snapshot,
        requires_rotation,
        rotation_reason
    )
    VALUES (
        v_session.user_id,
        p_token_hash,
        p_now,
        p_now,
        p_now,
        v_expires,
        v_session.absolute_expires_at,
        COALESCE(p_user_agent, v_session.user_agent),
        COALESCE(p_ip, v_session.ip),
        COALESCE(p_client_meta, '{}'::JSONB),
        COALESCE(p_roles, v_session.roles_snapshot),
        FALSE,
        NULL
    )
    RETURNING id,
              v_session.user_id,
              issued_at,
              expires_at,
              absolute_expires_at,
              roles_snapshot
      INTO next_session_id,
           user_id,
           issued_at,
           expires_at,
           absolute_expires_at,
           roles_snapshot;

    RETURN NEXT;
    RETURN;
END;
$$;

CREATE OR REPLACE PROCEDURE rustygpt.sp_auth_logout(
    IN p_session_id UUID,
    IN p_reason TEXT DEFAULT 'logout',
    IN p_revoker UUID DEFAULT NULL
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
BEGIN
    UPDATE rustygpt.user_sessions
    SET revoked_at = clock_timestamp(),
        revoked_by = COALESCE(p_revoker, user_id),
        rotation_reason = COALESCE(p_reason, rotation_reason)
    WHERE id = p_session_id
      AND revoked_at IS NULL;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_auth_mark_rotation(
    p_user_id UUID,
    p_reason TEXT
)
RETURNS INTEGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_reason TEXT := COALESCE(NULLIF(btrim(p_reason), ''), 'privilege_change');
    v_updated INTEGER;
BEGIN
    UPDATE rustygpt.user_sessions
    SET requires_rotation = TRUE,
        rotation_reason = v_reason
    WHERE user_id = p_user_id
      AND revoked_at IS NULL;

    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN COALESCE(v_updated, 0);
END;
$$;
