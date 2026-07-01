-- Per-session MPC message HMAC key (F-004 completion).
-- The server-bound MPC message HMAC was keyed off a single global
-- MPC_HMAC_KEY, but that key was never distributable to clients without
-- baking a shared secret into the app. Instead, generate a random HMAC key
-- per session, hand it to the authenticated owner in the create/get session
-- response, and verify server-bound messages against this column.
ALTER TABLE mpc_sessions ADD COLUMN hmac_key BYTEA;
