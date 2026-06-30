# Wallet Creation & Management — Comprehensive Security & Logic Spec

## Scope
Full analysis of wallet creation, MPC protocol, shard management, transaction signing, and lifecycle logic.

## Findings Summary

| # | Severity | Category | Issue | Location |
|---|----------|----------|-------|----------|
| 1 | CRITICAL | Security | Plaintext shard returned over HTTP | shards.rs:246-250 |
| 2 | CRITICAL | Security | Fake HMAC (SHA-256 hash, not keyed) | mpc.rs:277-279 |
| 3 | CRITICAL | Code | Float-to-integer truncation in gas estimation | tx.rs:458 |
| 4 | CRITICAL | Code | Compressed public key rejection breaks wallet creation | wallets.rs:eth_address_from_pubkey |
| 5 | HIGH | Security | Full private key reconstruction in release FFI | ffi-mobile/api.rs:1212-1234 |
| 6 | HIGH | Security | Missing zeroization after key rotation | crypto.rs:118 |
| 7 | HIGH | Architecture | Dual wallet creation (server DKG + client API) race | mpc_participant/mod.rs:337-349 |
| 8 | HIGH | Code | Non-atomic DKG completion (wallet + shard separate) | mpc_participant/mod.rs |
| 9 | HIGH | Code | u64 overflow in spending summary (wei) | tx.rs:320 |
| 10 | HIGH | Code | Round number mismatch DB vs NATS | mpc_participant/mod.rs |
| 11 | HIGH | Architecture | Presign reservation never released on sign failure | presign_manager.rs:187-203 |
| 12 | HIGH | Architecture | Freeze check missing from tx submit/broadcast | tx.rs:43-195 |
| 13 | MEDIUM | Security | No rate limiting on shard retrieval | shards.rs |
| 14 | MEDIUM | Security | MPC session never expires | mpc_participant/ |
| 15 | MEDIUM | Security | ProtocolMessage lacks integrity protection | mpc-core dkls23 |
| 16 | MEDIUM | Architecture | Backup contribution race (DashMap 5min TTL) | mpc_participant/mod.rs:44 |
| 17 | MEDIUM | Code | Race condition in wallet creation (check-then-insert) | wallets.rs:create_wallet |
| 18 | MEDIUM | Code | 5-second WebSocket timeout for DKG | mpc_wallet_service.dart |
| 19 | MEDIUM | Code | PIN stored in plaintext in widget state | onboarding_flow.dart:1375 |
| 20 | MEDIUM | Code | Empty tx_hash returned without validation | tx.rs |
| 21 | MEDIUM | Code | Fire-and-forget last_used update | shards.rs |
| 22 | LOW | Architecture | No wallet status check for reshare | mpc.rs |
| 23 | LOW | Code | derive_backup_share returns empty vec on error | mpc_participant/mod.rs |
| 24 | LOW | Code | Encryption key from env without KDF | shard_store.rs |

## Detailed Requirements

### CRITICAL Fixes (Must fix before any production use)

#### C1: Plaintext Shard Over HTTP
- **Problem**: `shards.rs:246-250` returns raw shard material in HTTP response body. Network interception yields full key compromise.
- **Fix**: Envelope encryption — encrypt shard with client's ephemeral X25519 public key before returning.

#### C2: Fake HMAC in MPC Session
- **Problem**: `mpc.rs:277-279` uses `Sha256::digest(session_id + round)` as "MAC" — not keyed, trivially forgeable.
- **Fix**: Use `hmac::Hmac<Sha256>` with server-side secret, or replace with a random nonce.

#### C3: Float-to-Integer Truncation in Gas Estimation
- **Problem**: `tx.rs:458` casts `f64` to `u128` — silently truncates at large values, causing incorrect fees.
- **Fix**: Integer arithmetic: `gas_price * 120 / 100` instead of float multiplication.

#### C4: Compressed Public Key Rejection
- **Problem**: `wallets.rs:eth_address_from_pubkey` only accepts 64/65-byte keys. 33-byte compressed keys error out, breaking wallet creation if any code path produces them.
- **Fix**: Add `k256::PublicKey::from_sec1_bytes()` decompression for 33-byte inputs.

### HIGH Fixes (Must fix before handling real funds)

#### H1: Full Key Reconstruction in Release FFI
- **Problem**: `ffi-mobile/api.rs:1212-1234` reconstructs complete private key from 2 shards. Compiled into release builds.
- **Fix**: Gate behind `#[cfg(debug_assertions)]` or remove entirely.

#### H2: Missing Zeroization After Key Rotation
- **Problem**: `crypto.rs:118` uses `drop()` instead of `zeroize()` — old key persists in memory.
- **Fix**: Use `zeroize::Zeroize` trait on all sensitive byte buffers.

#### H3: Dual Wallet Creation Race
- **Problem**: Server DKG finalize auto-creates wallet (`mpc_participant/mod.rs:337-349`), then client's `createWallet` hits CONFLICT. Client never receives wallet_id from server path.
- **Fix**: Remove server-side auto-creation; let client call `createWallet` after DKG. Or: remove client endpoint, have client discover via `listWallets`.

#### H4: Non-Atomic DKG Completion
- **Problem**: Wallet row INSERT and shard storage are separate queries. Crash between them = wallet without shard (unrecoverable).
- **Fix**: Wrap in a single `sqlx` transaction.

#### H5: u64 Overflow in Spending Summary
- **Problem**: `tx.rs:320` sums wei values as `u64` — overflows at ~18.4 ETH.
- **Fix**: Use `U256` or `u128` for wei-denominated arithmetic.

#### H6: Round Number Mismatch (DB vs NATS)
- **Problem**: Message stored in DB with `round_number` but published to NATS with `round_number + 1`. Polling fallback reads wrong round.
- **Fix**: Use a single `next_round` variable for both paths.

#### H7: Presign Reservation Leak
- **Problem**: `presign_manager.rs:187-203` — reserved presignatures never released on sign failure. Stale cleanup uses `created_at` not `reserved_at`.
- **Fix**: Add `reserved_at` column; on session failure, explicitly release back to `available`.

#### H8: Freeze Check Missing from TX Broadcast
- **Problem**: `tx.rs:43-195` and `submit_userop` at line 609 don't check wallet freeze status. Pre-freeze signatures can still broadcast.
- **Fix**: Add freeze check in `submit` when `from_addr` matches a wallet.

### MEDIUM Fixes

#### M1: No Rate Limiting on Shard Retrieval
- Add per-user rate limit (3 req/hour) on shard access endpoints.

#### M2: MPC Session Never Expires
- Enforce 5-minute TTL on MPC sessions with automatic cleanup.

#### M3: ProtocolMessage Lacks Integrity Protection
- Add HMAC (keyed from Noise session) over serialized payloads.

#### M4: Backup Contribution Race (5min DashMap TTL)
- Persist backup contributions to DB with 1-hour TTL instead of in-memory DashMap.

#### M5: Wallet Creation Check-Then-Insert Race
- Replace SELECT+INSERT with `INSERT ... ON CONFLICT DO NOTHING` or use DB unique constraint.

#### M6: WebSocket Timeout Too Short for DKG
- Increase from 5s to 30s with exponential backoff retry.

#### M7: PIN in Plaintext Widget State
- Zero PIN string after use or store in FlutterSecureStorage immediately.

#### M8: Empty tx_hash Not Validated
- Validate tx_hash is 66 chars (0x + 64 hex) before returning 200.

### LOW Fixes

#### L1: No Wallet Status Check for Reshare
- Block reshare on archived wallets.

#### L2: derive_backup_share Returns Empty Vec on Error
- Propagate error instead of returning `vec![]`.

#### L3: Encryption Key Without KDF
- Apply HKDF with domain separator before using env var as AES key.

## Implementation Priority

1. **Phase A** (CRITICAL, blocking): C1, C2, C3, C4
2. **Phase B** (HIGH, pre-launch): H1-H8
3. **Phase C** (MEDIUM, hardening): M1-M8
4. **Phase D** (LOW, polish): L1-L3

## Out of Scope
- UI/UX changes unrelated to security
- Performance optimization (unless security-adjacent)
- New feature development
