CREATE TABLE parse_error (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID REFERENCES app_user(id),
    raw_json    TEXT NOT NULL,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_parse_error_user_id ON parse_error(user_id);
CREATE INDEX idx_parse_error_created_at ON parse_error(created_at);
