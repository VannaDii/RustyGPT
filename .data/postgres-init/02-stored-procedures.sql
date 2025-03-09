-- Stored Procedure for User Signup
CREATE OR REPLACE FUNCTION register_user(email TEXT, password_hash TEXT) RETURNS UUID AS $$
DECLARE new_user_id UUID;
BEGIN
INSERT INTO users (email, password_hash)
VALUES (email, password_hash)
RETURNING id INTO new_user_id;
RETURN new_user_id;
END;
$$ LANGUAGE plpgsql;

-- Stored Procedure for User Login
CREATE OR REPLACE FUNCTION authenticate_user(email TEXT) RETURNS TABLE(id UUID, password_hash TEXT) AS $$ BEGIN RETURN QUERY
SELECT users.id,
  users.password_hash
FROM users
WHERE users.email = email;
END;
$$ LANGUAGE plpgsql;

-- Stored Procedure for OAuth Registration
CREATE OR REPLACE FUNCTION register_oauth_user(email TEXT, apple_id TEXT, github_id TEXT) RETURNS UUID AS $$
DECLARE new_user_id UUID;
BEGIN
INSERT INTO users (email, apple_id, github_id)
VALUES (email, apple_id, github_id) ON CONFLICT (email) DO
UPDATE
SET apple_id = COALESCE(EXCLUDED.apple_id, users.apple_id),
  github_id = COALESCE(EXCLUDED.github_id, users.github_id)
RETURNING id INTO new_user_id;
RETURN new_user_id;
END;
$$ LANGUAGE plpgsql;
-- Stored Procedure for Fetching User by ID
CREATE OR REPLACE FUNCTION get_user_by_id_proc(user_id UUID) RETURNS TABLE(
    id UUID,
    email TEXT,
    apple_id TEXT,
    github_id TEXT
  ) AS $$ BEGIN RETURN QUERY
SELECT users.id,
  users.email,
  users.apple_id,
  users.github_id
FROM users
WHERE users.id = user_id;
END;
$$ LANGUAGE plpgsql;
