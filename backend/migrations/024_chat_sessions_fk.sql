-- Add the missing foreign key from chat_sessions.user_id to users(id).
--
-- 009_chat_history.sql declared user_id as a bare `UUID NOT NULL` with no
-- reference, unlike transactions/policies/shard_metadata. Combined with the
-- (now-fixed) AI-route IDOR that trusted a body-supplied user_id, this allowed
-- orphaned/nil-owner sessions to be inserted. Enforce referential integrity and
-- cascade deletes so a user's sessions are removed with the user.

-- Remove any orphaned sessions (user_id not present in users, including the
-- nil UUID) before adding the constraint, otherwise the ALTER would fail.
-- chat_messages rows cascade-delete via their existing FK to chat_sessions.
DELETE FROM chat_sessions s
WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = s.user_id);

ALTER TABLE chat_sessions
    ADD CONSTRAINT chat_sessions_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
