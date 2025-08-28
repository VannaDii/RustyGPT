-- Stored procedures for user authentication and management
-- User Registration with username
CREATE OR REPLACE FUNCTION register_user(
    p_username TEXT,
    p_email TEXT,
    p_password_hash TEXT
  ) RETURNS UUID AS $$
DECLARE new_user_id UUID;
BEGIN
INSERT INTO users (username, email, password_hash)
VALUES (p_username, p_email, p_password_hash)
RETURNING id INTO new_user_id;

RETURN new_user_id;
EXCEPTION
WHEN unique_violation THEN -- Check which constraint was violated for better error messages
IF EXISTS(
  SELECT 1
  FROM users
  WHERE username = p_username
) THEN RAISE EXCEPTION 'error.user.username_exists?username=%',
p_username;
ELSE RAISE EXCEPTION 'error.user.email_exists?email=%',
p_email;
END IF;
END;
$$ LANGUAGE plpgsql;
-- User Authentication by email
CREATE OR REPLACE FUNCTION authenticate_user(p_email TEXT) RETURNS TABLE(id UUID, username TEXT, password_hash TEXT) AS $$
BEGIN
RETURN QUERY
SELECT users.id,
  users.username,
  users.password_hash
FROM users
WHERE users.email = p_email;
END;
$$ LANGUAGE plpgsql;

-- User Authentication by username
CREATE OR REPLACE FUNCTION authenticate_user_by_username(p_username TEXT) RETURNS TABLE(id UUID, email TEXT, password_hash TEXT) AS $$ BEGIN RETURN QUERY
SELECT users.id,
  users.email,
  users.password_hash
FROM users
WHERE users.username = p_username;
END;
$$ LANGUAGE plpgsql;
-- Unified User Authentication by username or email
CREATE OR REPLACE FUNCTION authenticate_user_unified(p_username_or_email TEXT) RETURNS TABLE(
    id UUID,
    username TEXT,
    email TEXT,
    password_hash TEXT
  ) AS $$ BEGIN RETURN QUERY
SELECT users.id,
  users.username,
  users.email,
  users.password_hash
FROM users
WHERE users.email = p_username_or_email
  OR users.username = p_username_or_email;
END;
$$ LANGUAGE plpgsql;
-- OAuth User Registration/Update
CREATE OR REPLACE FUNCTION register_oauth_user(
    p_username TEXT,
    p_email TEXT,
    p_apple_id TEXT DEFAULT NULL,
    p_github_id TEXT DEFAULT NULL
  ) RETURNS UUID AS $$
DECLARE user_id UUID;
BEGIN
INSERT INTO users (username, email, apple_id, github_id)
VALUES (p_username, p_email, p_apple_id, p_github_id) ON CONFLICT (email) DO
UPDATE
SET apple_id = COALESCE(EXCLUDED.apple_id, users.apple_id),
  github_id = COALESCE(EXCLUDED.github_id, users.github_id),
  updated_at = NOW()
RETURNING id INTO user_id;
RETURN user_id;
EXCEPTION
WHEN unique_violation THEN -- Handle username conflict separately from email conflict
IF EXISTS(
  SELECT 1
  FROM users
  WHERE username = p_username
    AND email != p_email
) THEN RAISE EXCEPTION 'error.user.username_exists?username=%',
p_username;
ELSE RAISE EXCEPTION 'error.user.oauth_conflict?email=%',
p_email;
END IF;
END;
$$ LANGUAGE plpgsql;
-- Get User by ID
CREATE OR REPLACE FUNCTION get_user_by_id(p_user_id UUID) RETURNS TABLE(
    id UUID,
    username TEXT,
    email TEXT,
    apple_id TEXT,
github_id TEXT,
created_at TIMESTAMP
) AS $$ BEGIN RETURN QUERY
SELECT users.id,
  users.username,
  users.email,
  users.apple_id,
  users.github_id,
  users.created_at
FROM users
WHERE users.id = p_user_id;
END;
$$ LANGUAGE plpgsql;
-- Stored procedures for conversations
-- Create Conversation (simplified version for compatibility)
CREATE OR REPLACE FUNCTION create_conversation(p_title TEXT, p_created_by UUID) RETURNS UUID AS $$
DECLARE new_conversation_id UUID;
BEGIN -- Create the conversation
INSERT INTO conversations (title, created_by)
VALUES (p_title, p_created_by)
RETURNING id INTO new_conversation_id;
-- Add creator as participant
INSERT INTO conversation_participants (conversation_id, user_id)
VALUES (new_conversation_id, p_created_by);
RETURN new_conversation_id;
END;
$$ LANGUAGE plpgsql;
-- Create Conversation with participants (advanced version)
CREATE OR REPLACE FUNCTION create_conversation_with_participants(
    p_title TEXT,
    p_created_by UUID,
    p_participant_ids UUID []
  ) RETURNS UUID AS $$
DECLARE new_conversation_id UUID;
participant_id UUID;
BEGIN -- Create the conversation
INSERT INTO conversations (title, created_by)
VALUES (p_title, p_created_by)
RETURNING id INTO new_conversation_id;
-- Add creator as participant
INSERT INTO conversation_participants (conversation_id, user_id)
VALUES (new_conversation_id, p_created_by);
-- Add other participants
FOREACH participant_id IN ARRAY p_participant_ids LOOP
INSERT INTO conversation_participants (conversation_id, user_id)
VALUES (new_conversation_id, participant_id) ON CONFLICT DO NOTHING;
-- Skip if already exists
END LOOP;
RETURN new_conversation_id;
END;
$$ LANGUAGE plpgsql;
-- Add participant to conversation
CREATE OR REPLACE FUNCTION add_conversation_participant(
    p_conversation_id UUID,
    p_user_id UUID
  ) RETURNS VOID AS $$ BEGIN
INSERT INTO conversation_participants (conversation_id, user_id)
VALUES (p_conversation_id, p_user_id) ON CONFLICT DO NOTHING;
END;
$$ LANGUAGE plpgsql;
-- Get User Conversations (with full details)
CREATE OR REPLACE FUNCTION get_user_conversations(p_user_id UUID) RETURNS TABLE(
    id UUID,
    title TEXT,
    created_by UUID,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    participant_count BIGINT,
    last_message_at TIMESTAMP
  ) AS $$ BEGIN RETURN QUERY
SELECT c.id,
  c.title,
  c.created_by,
  c.created_at,
  c.updated_at,
  COUNT(cp.user_id) as participant_count,
  MAX(m.created_at) as last_message_at
FROM conversations c
  JOIN conversation_participants cp ON c.id = cp.conversation_id
  LEFT JOIN messages m ON c.id = m.conversation_id
WHERE cp.user_id = p_user_id
GROUP BY c.id,
  c.title,
  c.created_by,
  c.created_at,
  c.updated_at
ORDER BY COALESCE(MAX(m.created_at), c.updated_at) DESC;
END;
$$ LANGUAGE plpgsql;
-- List User Conversations (simplified for Rust service compatibility)
CREATE OR REPLACE FUNCTION list_user_conversations(p_user_id UUID) RETURNS TABLE(
    id UUID,
    title TEXT,
    created_at TIMESTAMP
  ) AS $$ BEGIN RETURN QUERY
SELECT c.id,
  c.title,
  c.created_at
FROM conversations c
  JOIN conversation_participants cp ON c.id = cp.conversation_id
WHERE cp.user_id = p_user_id
ORDER BY c.updated_at DESC;
END;
$$ LANGUAGE plpgsql;
-- Get Conversation Details
CREATE OR REPLACE FUNCTION get_conversation(p_conversation_id UUID, p_user_id UUID) RETURNS TABLE(
    id UUID,
    title TEXT,
    created_at TIMESTAMP
  ) AS $$ BEGIN -- Check if user is participant
  IF NOT EXISTS (
    SELECT 1
    FROM conversation_participants
    WHERE conversation_id = p_conversation_id
      AND user_id = p_user_id
  ) THEN RAISE EXCEPTION 'User % is not a participant in conversation %',
  p_user_id,
  p_conversation_id;
END IF;
RETURN QUERY
SELECT c.id,
  c.title,
  c.created_at
FROM conversations c
WHERE c.id = p_conversation_id;
END;
$$ LANGUAGE plpgsql;
-- Stored procedures for messages
-- Create Message (simplified for Rust service compatibility)
CREATE OR REPLACE FUNCTION create_message(
    p_conversation_id UUID,
    p_sender_id UUID,
    p_content TEXT,
    p_message_type TEXT
  ) RETURNS UUID AS $$
DECLARE new_message_id UUID;
BEGIN -- Check if user is participant in conversation
IF NOT EXISTS (
  SELECT 1
  FROM conversation_participants
  WHERE conversation_id = p_conversation_id
    AND user_id = p_sender_id
) THEN RAISE EXCEPTION 'User % is not a participant in conversation %',
p_sender_id,
p_conversation_id;
END IF;
-- Create the message
INSERT INTO messages (
    conversation_id,
    sender_id,
    content,
    message_type,
    parent_id
  )
VALUES (
    p_conversation_id,
    p_sender_id,
    p_content,
    p_message_type,
    NULL -- Default to no parent
  )
RETURNING id INTO new_message_id;
-- Update conversation updated_at
UPDATE conversations
SET updated_at = NOW()
WHERE id = p_conversation_id;
RETURN new_message_id;
END;
$$ LANGUAGE plpgsql;
-- Create Message with parent (advanced version)
CREATE OR REPLACE FUNCTION create_message_with_parent(
    p_conversation_id UUID,
    p_sender_id UUID,
    p_content TEXT,
    p_message_type TEXT DEFAULT 'user',
    p_parent_id UUID DEFAULT NULL
  ) RETURNS UUID AS $$
DECLARE new_message_id UUID;
BEGIN -- Check if user is participant in conversation
IF NOT EXISTS (
  SELECT 1
  FROM conversation_participants
  WHERE conversation_id = p_conversation_id
    AND user_id = p_sender_id
) THEN RAISE EXCEPTION 'User % is not a participant in conversation %',
p_sender_id,
p_conversation_id;
END IF;
-- Create the message
INSERT INTO messages (
    conversation_id,
    sender_id,
    content,
    message_type,
    parent_id
  )
VALUES (
    p_conversation_id,
    p_sender_id,
    p_content,
    p_message_type,
    p_parent_id
  )
RETURNING id INTO new_message_id;
-- Update conversation updated_at
UPDATE conversations
SET updated_at = NOW()
WHERE id = p_conversation_id;
RETURN new_message_id;
END;
$$ LANGUAGE plpgsql;
-- Get Conversation Messages (simplified for Rust service compatibility)
CREATE OR REPLACE FUNCTION get_conversation_messages(
    p_conversation_id UUID,
    p_user_id UUID
  ) RETURNS TABLE(
    id UUID,
    conversation_id UUID,
    sender_id UUID,
    content TEXT,
    message_type TEXT,
    created_at TIMESTAMP
  ) AS $$ BEGIN -- Check if user is participant
  IF NOT EXISTS (
    SELECT 1
    FROM conversation_participants
    WHERE conversation_id = p_conversation_id
      AND user_id = p_user_id
  ) THEN RAISE EXCEPTION 'User % is not a participant in conversation %',
  p_user_id,
  p_conversation_id;
END IF;
RETURN QUERY
SELECT m.id,
  m.conversation_id,
  m.sender_id,
  m.content,
  m.message_type,
  m.created_at
FROM messages m
WHERE m.conversation_id = p_conversation_id
ORDER BY m.created_at ASC;
END;
$$ LANGUAGE plpgsql;
-- Get Conversation Messages with details (advanced version)
CREATE OR REPLACE FUNCTION get_conversation_messages_with_details(
    p_conversation_id UUID,
    p_user_id UUID,
    p_limit INTEGER DEFAULT 50,
    p_offset INTEGER DEFAULT 0
  ) RETURNS TABLE(
    id UUID,
    sender_id UUID,
    content TEXT,
    message_type TEXT,
    parent_id UUID,
    created_at TIMESTAMP,
    sender_email TEXT
  ) AS $$ BEGIN -- Check if user is participant
  IF NOT EXISTS (
    SELECT 1
    FROM conversation_participants
    WHERE conversation_id = p_conversation_id
      AND user_id = p_user_id
  ) THEN RAISE EXCEPTION 'User % is not a participant in conversation %',
  p_user_id,
  p_conversation_id;
END IF;
RETURN QUERY
SELECT m.id,
  m.sender_id,
  m.content,
  m.message_type,
  m.parent_id,
  m.created_at,
  u.email as sender_email
FROM messages m
  JOIN users u ON m.sender_id = u.id
WHERE m.conversation_id = p_conversation_id
ORDER BY m.created_at DESC
LIMIT p_limit OFFSET p_offset;
END;
$$ LANGUAGE plpgsql;
-- Get Message by ID (simplified for Rust service compatibility)
CREATE OR REPLACE FUNCTION get_message(p_message_id UUID, p_user_id UUID) RETURNS TABLE(
    id UUID,
    conversation_id UUID,
    sender_id UUID,
    content TEXT,
    message_type TEXT,
    created_at TIMESTAMP
  ) AS $$ BEGIN -- Check if user is participant in the conversation containing this message
  IF NOT EXISTS (
    SELECT 1
    FROM messages m
      JOIN conversation_participants cp ON m.conversation_id = cp.conversation_id
    WHERE m.id = p_message_id
      AND cp.user_id = p_user_id
  ) THEN RAISE EXCEPTION 'User % does not have access to message %',
  p_user_id,
  p_message_id;
END IF;
RETURN QUERY
SELECT m.id,
  m.conversation_id,
  m.sender_id,
  m.content,
  m.message_type,
  m.created_at
FROM messages m
WHERE m.id = p_message_id;
END;
$$ LANGUAGE plpgsql;
-- Get Message with details (advanced version)
CREATE OR REPLACE FUNCTION get_message_with_details(p_message_id UUID, p_user_id UUID) RETURNS TABLE(
    id UUID,
    conversation_id UUID,
    sender_id UUID,
    content TEXT,
    message_type TEXT,
    parent_id UUID,
    created_at TIMESTAMP
  ) AS $$ BEGIN -- Check if user is participant in the conversation containing this message
  IF NOT EXISTS (
    SELECT 1
    FROM messages m
      JOIN conversation_participants cp ON m.conversation_id = cp.conversation_id
    WHERE m.id = p_message_id
      AND cp.user_id = p_user_id
  ) THEN RAISE EXCEPTION 'User % does not have access to message %',
  p_user_id,
  p_message_id;
END IF;
RETURN QUERY
SELECT m.id,
  m.conversation_id,
  m.sender_id,
  m.content,
  m.message_type,
  m.parent_id,
  m.created_at
FROM messages m
WHERE m.id = p_message_id;
END;
$$ LANGUAGE plpgsql;
-- Legacy compatibility procedures for setup and initial auth
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
CREATE OR REPLACE FUNCTION init_setup(
    p_username TEXT,
    p_email TEXT,
    p_password_hash TEXT
  ) RETURNS BOOLEAN AS $$
DECLARE
  new_user_id UUID;
BEGIN
  INSERT INTO users (username, email, password_hash)
VALUES (p_username, p_email, p_password_hash)
  RETURNING id INTO new_user_id;

  RETURN new_user_id IS NOT NULL;
EXCEPTION
WHEN unique_violation THEN -- Handle unique constraint violations with proper error messages
IF EXISTS(
  SELECT 1
  FROM users
  WHERE username = p_username
) THEN RAISE EXCEPTION 'error.setup.username_exists?username=%',
p_username;
ELSE RAISE EXCEPTION 'error.setup.email_exists?email=%',
p_email;
END IF;
END;
$$ LANGUAGE plpgsql;