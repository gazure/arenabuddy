CREATE TABLE refresh_token (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES app_user(id) ON DELETE CASCADE,
    token_hash    BYTEA NOT NULL UNIQUE,
    expires_at    TIMESTAMPTZ NOT NULL,
    revoked       BOOLEAN NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_refresh_token_user_id ON refresh_token(user_id);
CREATE INDEX idx_refresh_token_hash ON refresh_token(token_hash);
