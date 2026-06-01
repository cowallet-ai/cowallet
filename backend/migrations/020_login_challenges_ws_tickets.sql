-- F-001: public-key challenge-response login.
-- Stores a short-lived random nonce per device that the client must sign with
-- the wallet's secp256k1 private key to prove possession before tokens are issued.
CREATE TABLE login_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id TEXT NOT NULL,
    nonce BYTEA NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_login_challenges_device_id ON login_challenges(device_id);
CREATE INDEX idx_login_challenges_expires_at ON login_challenges(expires_at);

-- F-010: single-use, short-lived WebSocket tickets.
-- Exchanged from a valid JWT so the JWT never appears in WS query strings / logs.
CREATE TABLE ws_tickets (
    ticket TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ws_tickets_expires_at ON ws_tickets(expires_at);
