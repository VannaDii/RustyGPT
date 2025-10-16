-- Stored procedures for message lifecycle management
SET search_path TO rustygpt, public;

CREATE OR REPLACE FUNCTION rustygpt.sp_soft_delete_message(
    p_message UUID,
    p_actor UUID,
    p_reason TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_session UUID;
    v_message RECORD;
    v_role rustygpt.conversation_role;
BEGIN
    v_session := rustygpt.sp_require_session_user();
    IF v_session <> p_actor THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    SELECT
        m.id,
        m.conversation_id,
        m.author_user_id,
        m.deleted_at
    INTO v_message
    FROM rustygpt.messages m
    WHERE m.id = p_message;

    IF v_message.id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: message not found';
    END IF;

    SELECT role
    INTO v_role
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = v_message.conversation_id
      AND cp.user_id = p_actor
      AND cp.left_at IS NULL;

    IF v_role IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: not a participant';
    END IF;

    IF p_actor <> v_message.author_user_id AND v_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: insufficient role';
    END IF;

    IF v_message.deleted_at IS NOT NULL THEN
        RETURN;
    END IF;

    UPDATE rustygpt.messages
    SET deleted_at = now(),
        deleted_by = p_actor,
        edit_reason = p_reason
    WHERE id = p_message;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_restore_message(
    p_message UUID,
    p_actor UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_session UUID;
    v_message RECORD;
    v_role rustygpt.conversation_role;
BEGIN
    v_session := rustygpt.sp_require_session_user();
    IF v_session <> p_actor THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    SELECT
        m.id,
        m.conversation_id,
        m.deleted_at
    INTO v_message
    FROM rustygpt.messages m
    WHERE m.id = p_message;

    IF v_message.id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: message not found';
    END IF;

    IF v_message.deleted_at IS NULL THEN
        RETURN;
    END IF;

    SELECT role
    INTO v_role
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = v_message.conversation_id
      AND cp.user_id = p_actor
      AND cp.left_at IS NULL;

    IF v_role IS NULL OR v_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: insufficient role';
    END IF;

    UPDATE rustygpt.messages
    SET deleted_at = NULL,
        deleted_by = NULL,
        edit_reason = NULL
    WHERE id = p_message;
END;
$$;

CREATE OR REPLACE FUNCTION rustygpt.sp_edit_message(
    p_message UUID,
    p_actor UUID,
    p_content TEXT,
    p_reason TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
DECLARE
    v_session UUID;
    v_message RECORD;
    v_role rustygpt.conversation_role;
BEGIN
    v_session := rustygpt.sp_require_session_user();
    IF v_session <> p_actor THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: actor mismatch';
    END IF;

    IF p_content IS NULL OR btrim(p_content) = '' THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.VALIDATION: content required';
    END IF;

    SELECT
        m.id,
        m.conversation_id,
        m.author_user_id,
        m.deleted_at
    INTO v_message
    FROM rustygpt.messages m
    WHERE m.id = p_message;

    IF v_message.id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.404: message not found';
    END IF;

    IF v_message.deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.409: message deleted';
    END IF;

    SELECT role
    INTO v_role
    FROM rustygpt.conversation_participants cp
    WHERE cp.conversation_id = v_message.conversation_id
      AND cp.user_id = p_actor
      AND cp.left_at IS NULL;

    IF v_role IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: not a participant';
    END IF;

    IF p_actor <> v_message.author_user_id AND v_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'RGP.403: insufficient role';
    END IF;

    UPDATE rustygpt.messages
    SET content = p_content,
        edited_at = now(),
        edited_by = p_actor,
        edit_reason = p_reason
    WHERE id = p_message;
END;
$$;
