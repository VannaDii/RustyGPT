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
    v_profile RECORD;
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

    SELECT requests_per_second, burst
    INTO v_profile
    FROM rustygpt.rate_limit_profiles
    WHERE name = 'conversation.post';

    IF v_profile.requests_per_second IS NULL OR v_profile.requests_per_second <= 0 THEN
        v_profile.requests_per_second := 5;
    END IF;

    IF v_profile.burst IS NULL OR v_profile.burst <= 0 THEN
        v_profile.burst := 10;
    END IF;

    v_interval := interval '1 second' / v_profile.requests_per_second;
    v_burst_window := v_interval * v_profile.burst;

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
