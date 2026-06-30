# Autopilot Implementation Plan — Wallet Security & Logic Fixes

## Phase A: CRITICAL Fixes (4 issues)

### Task 1: Fix plaintext shard exposure (C1)
- File: `backend/api-server/src/routes/shards.rs`
- Action: Remove or encrypt the raw shard return at line 246-250
- Approach: Return encrypted shard using client's public key from request header

### Task 2: Fix fake HMAC in MPC session (C2)
- File: `backend/api-server/src/routes/mpc.rs`
- Action: Replace SHA-256 hash at line 277-279 with proper HMAC using server secret
- Approach: Use `hmac` crate with JWT_SECRET as HMAC key

### Task 3: Fix float-to-integer gas truncation (C3)
- File: `backend/api-server/src/routes/tx.rs`
- Action: Replace `(gas_price as f64 * 1.2) as u128` at line 458
- Approach: `gas_price.saturating_mul(120) / 100`

### Task 4: Fix compressed public key rejection (C4)
- File: `backend/api-server/src/routes/wallets.rs`
- Action: Add 33-byte compressed key decompression in `eth_address_from_pubkey`
- Approach: Use `k256::PublicKey::from_sec1_bytes()` then convert to uncompressed

## Phase B: HIGH Fixes (8 issues)

### Task 5: Gate full key reconstruction behind debug (H1)
- File: `crates/ffi-mobile/src/api.rs`
- Action: Add `#[cfg(debug_assertions)]` to `sign_hash` at line 1212-1234

### Task 6: Add zeroization after key rotation (H2)
- File: `backend/api-server/src/services/crypto.rs`
- Action: Replace `drop()` with `zeroize()` at line 118
- Dep: Add `zeroize` to Cargo.toml if not present

### Task 7: Fix dual wallet creation race (H3)
- File: `backend/api-server/src/services/mpc_participant/mod.rs`
- Action: Remove auto wallet creation at lines 337-349 in DKG finalize
- Client discovers wallet via `listWallets` after DKG

### Task 8: Make DKG completion atomic (H4)
- File: `backend/api-server/src/services/mpc_participant/mod.rs`
- Action: Wrap wallet INSERT + shard store in single transaction

### Task 9: Fix u64 overflow in spending summary (H5)
- File: `backend/api-server/src/routes/tx.rs`
- Action: Change sum type from `u64` to `u128` at line 320

### Task 10: Fix round number mismatch (H6)
- File: `backend/api-server/src/services/mpc_participant/mod.rs`
- Action: Use consistent round number for DB write and NATS publish

### Task 11: Fix presign reservation leak (H7)
- File: `backend/api-server/src/services/presign_manager.rs`
- Action: On session failure, release presignature back to `available`; use `reserved_at` for stale check

### Task 12: Add freeze check to tx broadcast (H8)
- File: `backend/api-server/src/routes/tx.rs`
- Action: Check wallet freeze status before broadcasting in `submit` and `submit_userop`

## Phase C: MEDIUM Fixes (8 issues)

### Task 13: Rate limit shard retrieval (M1)
- File: `backend/api-server/src/routes/shards.rs`
- Action: Add per-user rate limit (3/hour) on shard access

### Task 14: MPC session expiry (M2)
- File: `backend/api-server/src/services/mpc_participant/mod.rs`
- Action: Add 5-minute TTL enforcement on sessions

### Task 15: Wallet creation atomicity (M5)
- File: `backend/api-server/src/routes/wallets.rs`
- Action: Replace check-then-insert with INSERT ON CONFLICT

### Task 16: WebSocket timeout increase (M6)
- File: `mobile/lib/services/mpc_wallet_service.dart`
- Action: Increase DKG timeout from 5s to 30s

### Task 17: PIN zeroization (M7)
- File: `mobile/lib/onboarding/onboarding_flow.dart`
- Action: Clear PIN string from state after use

### Task 18: Validate tx_hash format (M8)
- File: `backend/api-server/src/routes/tx.rs`
- Action: Validate 66-char hex format before returning 200

## Execution Strategy
- Tasks 1-4 (CRITICAL): Execute sequentially, verify with `cargo check` after each
- Tasks 5-12 (HIGH): Execute in parallel where independent
- Tasks 13-18 (MEDIUM): Execute in parallel
- Final: Run `cargo check --workspace` and `cargo test` to verify all changes
