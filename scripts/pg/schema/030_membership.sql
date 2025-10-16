-- Membership, invitations, presence, and unread tracking schema additions
SET search_path TO rustygpt, public;

-- Extend messages with lifecycle columns
ALTER TABLE rustygpt.messages
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deleted_by UUID,
    ADD COLUMN IF NOT EXISTS edited_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS edited_by UUID,
    ADD COLUMN IF NOT EXISTS edit_reason TEXT;

-- Extend typing states to track root messages per participant
ALTER TABLE rustygpt.typing_states
    ADD COLUMN IF NOT EXISTS root_message_id UUID;

ALTER TABLE rustygpt.typing_states
    DROP CONSTRAINT IF EXISTS typing_states_pkey;

ALTER TABLE rustygpt.typing_states
    ADD CONSTRAINT typing_states_pkey PRIMARY KEY (conversation_id, root_message_id, user_id);

-- Conversation invites -------------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.conversation_invites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES rustygpt.conversations(id) ON DELETE CASCADE,
    invited_email CITEXT NOT NULL,
    role rustygpt.conversation_role NOT NULL,
    invited_by UUID NOT NULL,
    token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_by UUID,
    accepted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_conversation_invites_conversation
    ON rustygpt.conversation_invites (conversation_id);

-- Presence tracking ---------------------------------------------------------

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_type
        WHERE typname = 'presence_status'
          AND typnamespace = 'rustygpt'::regnamespace
    ) THEN
        CREATE TYPE rustygpt.presence_status AS ENUM ('online', 'away', 'offline');
    END IF;
END;
$$;

CREATE TABLE IF NOT EXISTS rustygpt.presence (
    user_id UUID PRIMARY KEY,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    status rustygpt.presence_status NOT NULL DEFAULT 'offline'
);

CREATE INDEX IF NOT EXISTS idx_presence_last_seen ON rustygpt.presence (last_seen_at);

-- Message read watermarks ---------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.message_read_watermarks (
    conversation_id UUID NOT NULL REFERENCES rustygpt.conversations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    root_message_id UUID NOT NULL REFERENCES rustygpt.messages(id) ON DELETE CASCADE,
    last_read_path LTREE NOT NULL,
    last_read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (conversation_id, user_id, root_message_id)
);

CREATE INDEX IF NOT EXISTS idx_read_watermarks_user
    ON rustygpt.message_read_watermarks (user_id, conversation_id);

-- RLS policies --------------------------------------------------------------

ALTER TABLE rustygpt.conversation_invites ENABLE ROW LEVEL SECURITY;
ALTER TABLE rustygpt.message_read_watermarks ENABLE ROW LEVEL SECURITY;

DO $policy$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'rustygpt'
          AND tablename = 'conversation_invites'
          AND policyname = 'conversation_invites_participant_access'
    ) THEN
        CREATE POLICY conversation_invites_participant_access ON rustygpt.conversation_invites
            USING (
                EXISTS (
                    SELECT 1
                    FROM rustygpt.conversation_participants cp
                    WHERE cp.conversation_id = conversation_invites.conversation_id
                      AND cp.user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
                      AND cp.left_at IS NULL
                )
            );
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'rustygpt'
          AND tablename = 'message_read_watermarks'
          AND policyname = 'message_read_watermarks_owner'
    ) THEN
        CREATE POLICY message_read_watermarks_owner ON rustygpt.message_read_watermarks
            USING (
                user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
            )
            WITH CHECK (
                user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
            );
    END IF;
END;
$policy$;
