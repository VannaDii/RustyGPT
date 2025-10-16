-- Threaded conversations schema changes for RustyGPT
SET search_path TO rustygpt, public;

-- Ensure required extensions are available
CREATE EXTENSION IF NOT EXISTS ltree;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Helper functions --------------------------------------------------------

CREATE OR REPLACE FUNCTION rustygpt.uuid_to_label(p_id UUID)
RETURNS LTREE
LANGUAGE SQL
IMMUTABLE
SECURITY DEFINER
SET search_path = rustygpt, public
AS $$
    SELECT ('m' || replace(p_id::text, '-', ''))::ltree;
$$;

-- Enum types ---------------------------------------------------------------

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_type typ
        JOIN pg_namespace nsp ON nsp.oid = typ.typnamespace
        WHERE typ.typname = 'conversation_role'
          AND nsp.nspname = 'rustygpt'
    ) THEN
        CREATE TYPE rustygpt.conversation_role AS ENUM ('owner', 'admin', 'member', 'viewer');
    END IF;
END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_type typ
        JOIN pg_namespace nsp ON nsp.oid = typ.typnamespace
        WHERE typ.typname = 'message_role'
          AND nsp.nspname = 'rustygpt'
    ) THEN
        CREATE TYPE rustygpt.message_role AS ENUM ('user', 'assistant', 'system', 'tool');
    END IF;
END;
$$;

-- Conversations ------------------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    is_group BOOLEAN NOT NULL DEFAULT FALSE,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS rustygpt.conversation_participants (
    conversation_id UUID NOT NULL REFERENCES rustygpt.conversations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    role rustygpt.conversation_role NOT NULL DEFAULT 'member',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at TIMESTAMPTZ,
    PRIMARY KEY (conversation_id, user_id, joined_at)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_conversation_participants_active
    ON rustygpt.conversation_participants (conversation_id, user_id)
    WHERE left_at IS NULL;

-- Messages -----------------------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES rustygpt.conversations(id) ON DELETE CASCADE,
    parent_message_id UUID REFERENCES rustygpt.messages(id) ON DELETE SET NULL,
    root_message_id UUID NOT NULL REFERENCES rustygpt.messages(id) ON DELETE CASCADE,
    author_user_id UUID,
    role rustygpt.message_role NOT NULL,
    content TEXT NOT NULL,
    path LTREE NOT NULL,
    depth INT GENERATED ALWAYS AS (nlevel(path)) STORED,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_root
    ON rustygpt.messages (conversation_id, root_message_id);

CREATE INDEX IF NOT EXISTS idx_messages_path
    ON rustygpt.messages USING GIST (path);

CREATE UNIQUE INDEX IF NOT EXISTS ux_messages_conversation_path
    ON rustygpt.messages (conversation_id, path);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_created_at
    ON rustygpt.messages (conversation_id, created_at DESC);

-- Message chunks -----------------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.message_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES rustygpt.messages(id) ON DELETE CASCADE,
    idx INT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_message_chunks_message_idx
    ON rustygpt.message_chunks (message_id, idx);

-- Typing states ------------------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.typing_states (
    conversation_id UUID NOT NULL REFERENCES rustygpt.conversations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (conversation_id, user_id)
);

-- Row Level Security -------------------------------------------------------

ALTER TABLE rustygpt.conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE rustygpt.conversation_participants ENABLE ROW LEVEL SECURITY;
ALTER TABLE rustygpt.messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE rustygpt.message_chunks ENABLE ROW LEVEL SECURITY;

DO $policy$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'rustygpt'
          AND tablename = 'conversations'
          AND policyname = 'conversations_participant_access'
    ) THEN
        CREATE POLICY conversations_participant_access ON rustygpt.conversations
            USING (
                EXISTS (
                    SELECT 1
                    FROM rustygpt.conversation_participants cp
                    WHERE cp.conversation_id = id
                      AND cp.user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
                      AND cp.left_at IS NULL
                )
            );
    END IF;
END;
$policy$;

DO $policy$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'rustygpt'
          AND tablename = 'conversation_participants'
          AND policyname = 'conversation_participants_members_only'
    ) THEN
        CREATE POLICY conversation_participants_members_only ON rustygpt.conversation_participants
            USING (
                EXISTS (
                    SELECT 1
                    FROM rustygpt.conversation_participants cp
                    WHERE cp.conversation_id = conversation_id
                      AND cp.user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
                      AND cp.left_at IS NULL
                )
            );
    END IF;
END;
$policy$;

DO $policy$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'rustygpt'
          AND tablename = 'messages'
          AND policyname = 'messages_participant_access'
    ) THEN
        CREATE POLICY messages_participant_access ON rustygpt.messages
            USING (
                EXISTS (
                    SELECT 1
                    FROM rustygpt.conversation_participants cp
                    WHERE cp.conversation_id = messages.conversation_id
                      AND cp.user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
                      AND cp.left_at IS NULL
                )
            );
    END IF;
END;
$policy$;

DO $policy$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'rustygpt'
          AND tablename = 'message_chunks'
          AND policyname = 'message_chunks_participant_access'
    ) THEN
        CREATE POLICY message_chunks_participant_access ON rustygpt.message_chunks
            USING (
                EXISTS (
                    SELECT 1
                    FROM rustygpt.messages m
                    JOIN rustygpt.conversation_participants cp
                        ON cp.conversation_id = m.conversation_id
                    WHERE m.id = message_chunks.message_id
                      AND cp.user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
                      AND cp.left_at IS NULL
                )
            );
    END IF;
END;
$policy$;
