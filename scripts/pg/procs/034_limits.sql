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
    v_rps NUMERIC := 5;
    v_burst INTEGER := 10;
    v_interval INTERVAL;
    v_burst_window INTERVAL;
    v_now TIMESTAMPTZ := clock_timestamp();
    v_tat TIMESTAMPTZ;
    v_allow_at TIMESTAMPTZ;
    v_new_tat TIMESTAMPTZ;
BEGIN
    v_actor := rustygpt.sp_require_session_user();
    IF v_actor <> p_user THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    IF NOT rustygpt.sp_user_can_access(p_user, p_conversation) THEN
        RETURN FALSE;
    END IF;

    SELECT
        COALESCE(
            NULLIF((params ->> 'requests_per_second')::NUMERIC, 0),
            v_rps
        ),
        COALESCE(
            NULLIF((params ->> 'burst')::INTEGER, 0),
            v_burst
        )
    INTO v_rps, v_burst
    FROM rustygpt.rate_limit_profiles
    WHERE name = 'conversation.post';

    v_interval := interval '1 second' / v_rps;
    v_burst_window := v_interval * v_burst;

    SELECT tat
    INTO v_tat
    FROM rustygpt.message_rate_limits
    WHERE user_id = p_user
      AND conversation_id = p_conversation
    FOR UPDATE;

    IF NOT FOUND THEN
        v_new_tat := v_now + v_interval;
        INSERT INTO rustygpt.message_rate_limits (user_id, conversation_id, tat, last_seen_at)
        VALUES (p_user, p_conversation, v_new_tat, v_now)
        ON CONFLICT (user_id, conversation_id) DO UPDATE
            SET tat = EXCLUDED.tat,
                last_seen_at = EXCLUDED.last_seen_at;
        RETURN TRUE;
    END IF;

    v_allow_at := v_tat - v_burst_window;

    IF v_now < v_allow_at THEN
        RETURN FALSE;
    END IF;

    v_new_tat := GREATEST(v_tat, v_now) + v_interval;

    UPDATE rustygpt.message_rate_limits
    SET tat = v_new_tat,
        last_seen_at = v_now
    WHERE user_id = p_user
      AND conversation_id = p_conversation;

    RETURN TRUE;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_limits_list_profiles()
RETURNS TABLE (
    profile_id UUID,
    name TEXT,
    algorithm TEXT,
    params JSONB,
    description TEXT,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
    SELECT id, name, algorithm, params, description, created_at, updated_at
    FROM rustygpt.rate_limit_profiles
    ORDER BY name;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_limits_create_profile(
    p_name TEXT,
    p_algorithm TEXT,
    p_params JSONB,
    p_description TEXT
)
RETURNS TABLE (
    profile_id UUID,
    name TEXT,
    algorithm TEXT,
    params JSONB,
    description TEXT,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_name TEXT := btrim(p_name);
    v_algo TEXT := lower(btrim(COALESCE(p_algorithm, 'gcra')));
BEGIN
    IF v_name IS NULL OR v_name = '' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.422: profile name required';
    END IF;

    IF v_algo NOT IN ('gcra', 'token_bucket', 'fixed_window') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.422: unsupported algorithm';
    END IF;

    INSERT INTO rustygpt.rate_limit_profiles (name, algorithm, params, description)
    VALUES (v_name, v_algo, COALESCE(p_params, '{}'::JSONB), NULLIF(btrim(p_description), ''))
    RETURNING id, name, algorithm, params, description, created_at, updated_at
      INTO profile_id, name, algorithm, params, description, created_at, updated_at;

    RETURN NEXT;
    RETURN;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_limits_update_profile(
    p_profile_id UUID,
    p_params JSONB,
    p_description TEXT
)
RETURNS TABLE (
    profile_id UUID,
    name TEXT,
    algorithm TEXT,
    params JSONB,
    description TEXT,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
BEGIN
    UPDATE rustygpt.rate_limit_profiles
    SET params = COALESCE(p_params, params),
        description = NULLIF(btrim(p_description), ''),
        updated_at = clock_timestamp()
    WHERE id = p_profile_id
    RETURNING id, name, algorithm, params, description, created_at, updated_at
      INTO profile_id, name, algorithm, params, description, created_at, updated_at;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: profile not found';
    END IF;

    RETURN NEXT;
    RETURN;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_limits_delete_profile(
    p_profile_id UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_assigned BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM rustygpt.rate_limit_assignments
        WHERE profile_id = p_profile_id
    )
    INTO v_assigned;

    IF v_assigned THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.409: profile assigned to routes';
    END IF;

    DELETE FROM rustygpt.rate_limit_profiles
    WHERE id = p_profile_id;

    RETURN FOUND;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_limits_assign_route(
    p_profile_id UUID,
    p_method TEXT,
    p_path TEXT
)
RETURNS TABLE (
    assignment_id UUID,
    profile_id UUID,
    method TEXT,
    path_pattern TEXT,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_method TEXT := upper(btrim(COALESCE(p_method, 'GET')));
    v_path TEXT := btrim(COALESCE(p_path, ''));
    v_profile rustygpt.rate_limit_profiles%ROWTYPE;
    v_existing rustygpt.rate_limit_assignments%ROWTYPE;
BEGIN
    IF v_method NOT IN ('GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS', 'HEAD') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = format('RGP.422: unsupported HTTP method "%s"', v_method);
    END IF;

    IF v_path = '' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.422: path pattern required';
    END IF;

    IF v_path NOT LIKE '/%' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.422: path pattern must start with "/"';
    END IF;
    IF v_path ~ '\s' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.422: path pattern must not contain whitespace';
    END IF;

    SELECT * INTO v_profile
    FROM rustygpt.rate_limit_profiles
    WHERE id = p_profile_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: rate limit profile not found';
    END IF;

    IF v_profile.algorithm NOT IN ('gcra', 'token_bucket', 'fixed_window') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = format('RGP.422: profile "%s" uses unsupported algorithm "%s"', v_profile.name, v_profile.algorithm);
    END IF;

    SELECT *
    INTO v_existing
    FROM rustygpt.rate_limit_assignments
    WHERE method = v_method
      AND path_pattern = v_path
    FOR UPDATE;

    IF FOUND THEN
        UPDATE rustygpt.rate_limit_assignments
        SET profile_id = p_profile_id,
            updated_at = clock_timestamp()
        WHERE id = v_existing.id
        RETURNING id, profile_id, method, path_pattern, created_at, updated_at
          INTO assignment_id, profile_id, method, path_pattern, created_at, updated_at;
    ELSE
        INSERT INTO rustygpt.rate_limit_assignments (profile_id, method, path_pattern)
        VALUES (p_profile_id, v_method, v_path)
        RETURNING id, profile_id, method, path_pattern, created_at, updated_at
          INTO assignment_id, profile_id, method, path_pattern, created_at, updated_at;
    END IF;

    RETURN NEXT;
    RETURN;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_limits_list_assignments()
RETURNS TABLE (
    assignment_id UUID,
    profile_id UUID,
    profile_name TEXT,
    method TEXT,
    path_pattern TEXT,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
    SELECT a.id,
           a.profile_id,
           p.name,
           a.method,
           a.path_pattern,
           a.created_at,
           a.updated_at
    FROM rustygpt.rate_limit_assignments a
    JOIN rustygpt.rate_limit_profiles p ON p.id = a.profile_id
    ORDER BY upper(a.method), a.path_pattern;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_limits_delete_assignment(
    p_assignment_id UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
BEGIN
    DELETE FROM rustygpt.rate_limit_assignments
    WHERE id = p_assignment_id;

    RETURN FOUND;
END;
$$;
