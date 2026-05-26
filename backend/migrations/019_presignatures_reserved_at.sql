-- Add reserved_at column to presignatures for accurate stale-reservation detection.
-- Previously, cleanup_stale_reservations used created_at which is set at presig generation
-- time, not reservation time — causing valid presignatures to be incorrectly expired.
ALTER TABLE presignatures ADD COLUMN IF NOT EXISTS reserved_at TIMESTAMPTZ;

-- Backfill: for currently reserved rows, use created_at as a conservative estimate.
UPDATE presignatures SET reserved_at = created_at WHERE status = 'reserved' AND reserved_at IS NULL;
