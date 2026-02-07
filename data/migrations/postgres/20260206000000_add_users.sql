-- Add app_user table for Discord OAuth authentication
CREATE TABLE app_user (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    discord_id  TEXT UNIQUE NOT NULL,
    username    TEXT NOT NULL,
    avatar_url  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Add optional user_id to match table (nullable for local-only mode and existing data)
ALTER TABLE match ADD COLUMN user_id UUID REFERENCES app_user(id);

CREATE INDEX idx_match_user_id ON match(user_id);
