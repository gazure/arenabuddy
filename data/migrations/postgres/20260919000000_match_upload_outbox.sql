-- Used by desktop clients. Server writes and downloads do not enqueue uploads.
CREATE TABLE match_upload_outbox (
    match_id UUID PRIMARY KEY REFERENCES match(id) ON DELETE CASCADE,
    -- Remote identity: local databases do not contain the server's app_user rows.
    user_id UUID,
    payload TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 1,
    state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'sent', 'blocked')),
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    lease_token UUID,
    lease_until TIMESTAMPTZ,
    last_error TEXT
);

CREATE INDEX match_upload_outbox_due ON match_upload_outbox(user_id, next_attempt_at)
WHERE state = 'pending';
