-- Challenge-response login verifies signatures against the device's registered
-- hardware key. iOS Secure Enclave uses P-256 ECDSA; Android StrongBox uses
-- RSA-2048. Record which algorithm a device's public_key is so login can pick
-- the right verifier. NULL = legacy/unregistered (challenge-response disabled).
ALTER TABLE users ADD COLUMN device_pubkey_alg TEXT;
