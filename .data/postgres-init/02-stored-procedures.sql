-- Stored Procedure for User Signup
CREATE OR REPLACE FUNCTION register_user(username TEXT, email TEXT, password_hash TEXT) RETURNS UUID AS $$
DECLARE new_user_id UUID;
BEGIN
INSERT INTO users (username, email, password_hash)
VALUES (username, email, password_hash)
RETURNING id INTO new_user_id;
RETURN new_user_id;
END;
$$ LANGUAGE plpgsql;
-- Stored Procedure for User Login
CREATE OR REPLACE FUNCTION authenticate_user(username TEXT, password_hash TEXT) RETURNS BOOLEAN AS $$
DECLARE
  user_exists BOOLEAN;
BEGIN
  SELECT EXISTS (
    SELECT 1
    FROM users
    WHERE (
      users.username = authenticate_user.username
      OR users.email = authenticate_user.username
    )
    AND users.password_hash = authenticate_user.password_hash
  ) INTO user_exists;

  RETURN user_exists;
END;
$$ LANGUAGE plpgsql;
-- Stored Procedure for OAuth Registration
CREATE OR REPLACE FUNCTION register_oauth_user(email TEXT, apple_id TEXT, github_id TEXT) RETURNS UUID AS $$
DECLARE new_user_id UUID;
BEGIN
INSERT INTO users (username, email, apple_id, github_id)
VALUES (email, email, apple_id, github_id) ON CONFLICT (email) DO
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
    username TEXT,
    email TEXT,
    apple_id TEXT,
    github_id TEXT
  ) AS $$ BEGIN RETURN QUERY
SELECT users.id,
  users.username,
  users.email,
  users.apple_id,
  users.github_id
FROM users
WHERE users.id = user_id;
END;
$$ LANGUAGE plpgsql;
-- Stored Procedure for setup check
CREATE OR REPLACE FUNCTION is_setup() RETURNS BOOLEAN AS $$
BEGIN
  RETURN EXISTS (
    SELECT 1
    FROM users
    LIMIT 1
  );
END;
$$ LANGUAGE plpgsql;
-- Stored Procedure for Initial Setup
CREATE OR REPLACE FUNCTION init_setup(username TEXT, email TEXT, password_hash TEXT) RETURNS BOOLEAN AS $$
DECLARE
  new_user_id UUID;
BEGIN
  INSERT INTO users (username, email, password_hash)
  VALUES (username, email, password_hash)
  RETURNING id INTO new_user_id;

  RETURN new_user_id IS NOT NULL;
END;
$$ LANGUAGE plpgsql;