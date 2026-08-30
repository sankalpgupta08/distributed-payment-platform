CREATE TABLE idempotency_keys (
    id UUID PRIMARY KEY,
    key VARCHAR(255) NOT NULL UNIQUE CHECK (char_length(key) > 0),
    request_hash VARCHAR(64) NOT NULL,
    response_body JSONB,
    status_code SMALLINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    CHECK (
        (response_body IS NULL AND status_code IS NULL)
        OR (response_body IS NOT NULL AND status_code IS NOT NULL)
    )
);

CREATE INDEX idx_idempotency_keys_expires_at ON idempotency_keys (expires_at);
