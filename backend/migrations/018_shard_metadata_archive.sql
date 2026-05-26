-- Archive table for shard_metadata: preserves old shards when force re-registration occurs
-- instead of permanently deleting them, enabling potential recovery via support intervention.
CREATE TABLE IF NOT EXISTS shard_metadata_archive (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    location VARCHAR(20) NOT NULL,
    party_index SMALLINT NOT NULL,
    encrypted_shard BYTEA NOT NULL,
    nonce BYTEA NOT NULL,
    public_key BYTEA,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    archive_reason VARCHAR(50) NOT NULL DEFAULT 'force_reregister',
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_shard_archive_user ON shard_metadata_archive(user_id);
CREATE INDEX idx_shard_archive_reason ON shard_metadata_archive(archive_reason);

-- Add backup_shard_hash column: stores SHA-256(backup_shard) for force re-register verification.
-- The client sends this hash after DKG completes so the server can later verify backup shard possession.
ALTER TABLE shard_metadata ADD COLUMN IF NOT EXISTS backup_shard_hash BYTEA;

-- Add attempts column to email_verifications for brute-force protection on registration OTP.
ALTER TABLE email_verifications ADD COLUMN IF NOT EXISTS attempts INT NOT NULL DEFAULT 0;
