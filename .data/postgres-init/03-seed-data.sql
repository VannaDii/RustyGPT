-- Seed data for development/testing
-- Create test users
INSERT INTO users (id, username, email, password_hash)
VALUES (
    '550e8400-e29b-41d4-a716-446655440001',
    'alice',
    'alice@example.com',
    '$argon2id$v=19$m=65536,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwPW6/mK9b5T2S4'
  ),
  (
    '550e8400-e29b-41d4-a716-446655440002',
    'bob',
    'bob@example.com',
    '$argon2id$v=19$m=65536,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwPW6/mK9b5T2S4'
  ),
  (
    '550e8400-e29b-41d4-a716-446655440003',
    'charlie',
    'charlie@example.com',
    '$argon2id$v=19$m=65536,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwPW6/mK9b5T2S4'
  ) ON CONFLICT (email) DO NOTHING;
-- Create test conversations
INSERT INTO conversations (id, title, created_by)
VALUES (
    '660e8400-e29b-41d4-a716-446655440001',
    'General Discussion',
    '550e8400-e29b-41d4-a716-446655440001'
  ),
  (
    '660e8400-e29b-41d4-a716-446655440002',
    'Project Planning',
    '550e8400-e29b-41d4-a716-446655440001'
  ) ON CONFLICT (id) DO NOTHING;
-- Add participants to conversations
INSERT INTO conversation_participants (conversation_id, user_id)
VALUES (
    '660e8400-e29b-41d4-a716-446655440001',
    '550e8400-e29b-41d4-a716-446655440001'
  ),
  (
    '660e8400-e29b-41d4-a716-446655440001',
    '550e8400-e29b-41d4-a716-446655440002'
  ),
  (
    '660e8400-e29b-41d4-a716-446655440001',
    '550e8400-e29b-41d4-a716-446655440003'
  ),
  (
    '660e8400-e29b-41d4-a716-446655440002',
    '550e8400-e29b-41d4-a716-446655440001'
  ),
  (
    '660e8400-e29b-41d4-a716-446655440002',
    '550e8400-e29b-41d4-a716-446655440002'
  ) ON CONFLICT (conversation_id, user_id) DO NOTHING;
-- Create test messages
INSERT INTO messages (
    id,
    conversation_id,
    sender_id,
    content,
    message_type
  )
VALUES (
    '770e8400-e29b-41d4-a716-446655440001',
    '660e8400-e29b-41d4-a716-446655440001',
    '550e8400-e29b-41d4-a716-446655440001',
    'Hello everyone! Welcome to the chat.',
    'user'
  ),
  (
    '770e8400-e29b-41d4-a716-446655440002',
    '660e8400-e29b-41d4-a716-446655440001',
    '550e8400-e29b-41d4-a716-446655440002',
    'Hi Alice! Thanks for setting this up.',
    'user'
  ),
  (
    '770e8400-e29b-41d4-a716-446655440003',
    '660e8400-e29b-41d4-a716-446655440001',
    '550e8400-e29b-41d4-a716-446655440003',
    'Great to be here!',
    'user'
  ),
  (
    '770e8400-e29b-41d4-a716-446655440004',
    '660e8400-e29b-41d4-a716-446655440002',
    '550e8400-e29b-41d4-a716-446655440001',
    'Let''s discuss the project timeline.',
    'user'
  ),
  (
    '770e8400-e29b-41d4-a716-446655440005',
    '660e8400-e29b-41d4-a716-446655440002',
    '550e8400-e29b-41d4-a716-446655440002',
    'I think we should start with the database schema.',
    'user'
  ) ON CONFLICT (id) DO NOTHING;