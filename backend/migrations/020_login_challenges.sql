-- Challenge-response login: server issues a random nonce that the device must
-- sign with its registered secp256k1 key. A stolen device_id alone no longer
-- grants tokens.
CREATE TABLE login_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id TEXT NOT NULL,
    challenge BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '2 minutes',
    consumed BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_login_challenges_device ON login_challenges(device_id, consumed, expires_at);
