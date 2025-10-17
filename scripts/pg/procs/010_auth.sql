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
