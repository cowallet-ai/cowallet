-- F-010: single-use, short-lived WebSocket tickets.
-- Exchanged from a valid JWT so the JWT never appears in WS query strings / logs.
-- (login_challenges lives in 020_login_challenges.sql; this migration only adds
-- the ws_tickets table.)
CREATE TABLE ws_tickets (
    ticket TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ws_tickets_expires_at ON ws_tickets(expires_at);
