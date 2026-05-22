-- Add 'frozen' to wallets status constraint for emergency freeze feature
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS wallets_status_check;
ALTER TABLE wallets ADD CONSTRAINT wallets_status_check
    CHECK (status IN ('active', 'archived', 'compromised', 'frozen'));
