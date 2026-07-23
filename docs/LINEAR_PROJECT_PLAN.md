# CoWallet — Linear Project Plan (COW)

> Generated from `PROJECT_STATUS.md` (2026-05-30, Alpha ~80-85%) + `CLAUDE.md`.
> Team: **COW** · Workspace: clawmint
>
> **How to use this doc:** Each `## Milestone` → a Linear Project or Milestone.
> Each `### [PREFIX] Epic` → a parent issue. Each `- [ ]` line → a sub-issue.
> Metadata tags per issue: `P0-P3` (priority), `~Xh/Xd` (estimate), `#label`.
>
> Priority map: P0 = Urgent (launch blocker) · P1 = High · P2 = Medium · P3 = Low.
> Status map: ✅ Done · 🔧 In Progress · ❌ Todo · 🧪 Done (verified on device/prod).

---

## Labels (create these first)

`mpc` · `blockchain` · `ai` · `mobile` · `backend` · `security` · `reliability` · `testing` · `devops` · `defi` · `swap` · `tech-debt` · `recovery`

---

## Milestone 0 — Launch Blockers (P0)

> Target: **now → 2 weeks**. Must all close before any public exposure.

### [SEC] Critical security fixes
- [ ] Add JWT blacklist check to WebSocket MPC session — logout'd tokens can still sign · `P0` `~1h` `#security` `#mpc` — `routes/mpc_ws.rs:47`
- [ ] Replace JWT-in-URL with one-time WS ticket — token leaks to logs/CDN, replayable · `P0` `~4h` `#security` `#mpc` — `routes/mpc_ws.rs:22`
- [ ] Change ERC-20 approval from MaxUint256 to exact amount — aggregator compromise = full drain · `P0` `~30m` `#security` `#blockchain` — `intent_executor.dart:392`
- [ ] Remove hardcoded Covalent API key from source · `P0` `~5m` `#security` `#backend` — `state.rs:143-144`

### [REL] Critical reliability fixes
- [ ] Add 60s idle timeout + 4MB buffer cap to Bedrock stream — prevents hung connections & OOM · `P0` `~2h` `#reliability` `#ai`
- [ ] Bound Bedrock buffer growth (fault frames → unbounded memory) · `P0` `~2h` `#reliability` `#ai`

---

## Milestone 1 — Pre-launch Hardening (P1)

> Target: **1-2 weeks**. Security + reliability + recovery correctness.

### [SEC] Security hardening
- [ ] Structured sanitization of contacts/portfolio before injecting into AI context — indirect prompt injection · `P1` `~2h` `#security` `#ai` — `ai.rs:793-801`
- [ ] Strengthen prompt-injection defense beyond keyword matching (unicode/variant bypass) · `P1` `~2h` `#security` `#ai` — `ai.rs:438-445`
- [ ] Enforce Device-ID header (currently bypassable by omitting it) · `P1` `~30m` `#security` `#backend` — `middleware/auth.rs:199-207`
- [ ] Upgrade Argon2id params to m=64MB, t=3, p=4 · `P1` `~30m` `#security` `#mpc` — `mpc-core/shard/encrypt.rs:99`
- [ ] Use HKDF salt for key derivation · `P1` `~1h` `#security` — `services/crypto.rs:63`
- [ ] Replace bare SHA-256 AES-key derivation in shard transport with proper KDF · `P1` `~2h` `#security` `#mpc` — `routes/shards.rs:281`

### [REL] Reliability fixes
- [ ] Fix float precision loss in amount parsing (`double * 1e18`) — use string/decimal parsing · `P1` `~2h` `#reliability` `#blockchain`
- [ ] Add 60s no-data timeout to client-side AI stream — UI stuck on "loading" · `P1` `~1h` `#reliability` `#mobile`

### [REC] End-to-end recovery verification
- [ ] Validate full lost-device recovery path (UI exists, flow unverified) — funds recoverability · `P1` `~3d` `#recovery` `#mpc`
- [ ] Verify key-rotation backup (reshare) under failure conditions — untested, risk of fund loss · `P1` `~2d` `#recovery` `#mpc`

### [TEST] Integration test coverage
- [ ] Integration tests for MPC protocol (DKG/presign/sign/reshare) — schema-drift regression guard · `P1` `~3d` `#testing` `#mpc`
- [ ] Integration tests for transaction submission + status state machine · `P1` `~2d` `#testing` `#blockchain`
- [ ] Wire test-only CI workflow into gate (in progress on branch) · `P1` `~4h` `#testing` `#devops`

---

## Milestone 2 — MPC Security Fix Release (Blocking constraints)

> ⚠️ From RELEASE-NOTES: `sign.rs` private-key leak (critical) + reshare pubkey binding fixed.
> Two hard constraints on ship.

### [MPC] Coordinated security release
- [ ] Ship App + Server **same version** — wire-format changed, cross-version deserialization fails · `P0` `~1d` `#mpc` `#devops`
- [ ] Assess existing wallets: keys signed by old code may be leaked; reshare can't fix (pubkey unchanged) — plan fresh DKG + asset migration · `P0` `~1w` `#mpc` `#security` `#recovery`
- [ ] Author + validate force-upgrade runbook (`docs/FORCE_UPGRADE_RUNBOOK.md`) · `P1` `~4h` `#devops` `#mobile`

---

## Milestone 3 — Core Wallet (mostly done — verification & closeout)

### [MPC] MPC wallet core 🧪
- [x] DKLS23 DKG (2-of-3) 🧪 · `#mpc`
- [x] Online sign (<100ms) 🧪 · `#mpc`
- [x] Presign (background generation) ✅ · `#mpc`
- [x] Reshare (2-of-2 device+server, pubkey preserved) ✅ · `#mpc`
- [x] AES-GCM shard storage (Secure Enclave/Keystore) 🧪 · `#mpc` `#security`
- [x] Server shard management (HSM-grade, ENCRYPTION_KEY) 🧪 · `#mpc`
- [x] Backup shard export 🧪 · `#mpc`
- [x] Noise_XX transport encryption 🧪 · `#mpc` `#security`
- [x] WebSocket MPC session (NATS + DB-poll fallback) 🧪 · `#mpc` `#backend`
- [ ] Verify presign-pool auto-refill edge cases (untested boundaries) · `P2` `~1d` `#mpc` `#testing`

### [CHAIN] Blockchain 🧪
- [x] Multi-chain EVM (ETH/Base/Arbitrum/Optimism/BSC/Polygon) 🧪 · `#blockchain`
- [x] Native + ERC-20 transfers 🧪 · `#blockchain`
- [x] Gas estimation, tx simulation, status tracking 🧪 · `#blockchain`
- [x] Multi-chain balance + tx history (OKX Wallet API) 🧪 · `#blockchain`
- [x] Multi-RPC failover 🧪 · `#blockchain`
- [x] Token price query (cached, USD) 🧪 · `#blockchain`
- [ ] Validate ERC-4337 Account Abstraction end-to-end (built, effectively unused) · `P2` `~3d` `#blockchain`
- [ ] Exercise EIP-712 structured signing in a real DApp scenario · `P3` `~2d` `#blockchain`

### [SWAP] DEX swap (Bridgers)
- [x] Quote + build swap tx (Bridgers aggregator) ✅ · `#swap`
- [x] Same-chain swap 🧪 · `#swap`
- [x] Cross-chain swap (USDT approve-reset fix) ✅ · `#swap`
- [x] AI-initiated swap + slippage control ✅ · `#swap` `#ai`
- [ ] Device-verify cross-chain swap on real bridges · `P2` `~1d` `#swap` `#testing`

### [DEFI] DeFi yield
- [x] Yield protocol search (DeFiLlama) ✅ · `#defi`
- [x] Protocol list + DeFi Hub page ✅ · `#defi` `#mobile`

---

## Milestone 4 — AI Assistant (done — closeout)

### [AI] AI chat system 🧪
- [x] NL chat, dual provider (Bedrock Claude default + DeepSeek), SSE streaming 🧪 · `#ai`
- [x] Session management, context memory (Postgres), function calling, 2nd-round tool calls 🧪 · `#ai`
- [x] Widget card persistence (11 card types) 🧪 · `#ai` `#mobile`
- [x] 8 AI tools (get_balance, send_transaction, get_token_info, get_supported_chains, get_transaction_history, get_wallet_address, swap_token, list_yield_protocols) 🧪 · `#ai`

---

## Milestone 5 — Mobile App (done — theme polish remaining)

### [MOBILE] Screens & interactions 🧪
- [x] 12 primary screens (Home/Chat/Wallet/Yield/Settings/Search/TxHistory/Keys/Recovery/Scan/Contacts/BackupShard) 🧪 · `#mobile`
- [x] Biometric signing, real-time tx status, friendly errors 🧪 · `#mobile`
- [x] Local + FCM remote push (tx/security/MPC channels) ✅ · `#mobile`
- [x] Voice input, search→AI, token logos, EN/ZH i18n, Google Fonts 🧪 · `#mobile`
- [ ] Complete dark-mode adaptation (palette defined, details pending) · `P2` `~2d` `#mobile`

---

## Milestone 6 — Backend Services (done — closeout)

### [BE] API server + middleware + background jobs
- [x] Full route surface: auth/OTP/recovery/audit, MPC session+WS+presign, tx (submit/status/simulate/gas/history/summary/userop), balance, wallets, policy, AI, price, chains, swap, yield, shards, push ✅ · `#backend`
- [x] Middleware stack: JWT auth, rate limiting, audit log, metrics, CORS, security headers, 30s timeout, circuit breakers ✅ · `#backend`
- [x] Background: tx confirmation tracker, presign pool manager, session cleanup, price cache, RPC failover, email/OTP ✅ · `#backend`
- [ ] Verify Indexer/Worker on-chain event completeness (skeleton exists) · `P2` `~2d` `#backend`
- [ ] Wire MPC-sign approval interaction into FCM push (base done) · `P2` `~2d` `#backend` `#mobile`

---

## Milestone 7 — Tech Debt / Architecture (P2-P3)

### [DEBT] Refactors
- [ ] Split `AppState` God Object (24 fields) into domain sub-states · `P3` `~1d` `#tech-debt` `#backend`
- [ ] Split `ai.rs` (1330 lines) into prompt/tools/stream/session modules · `P2` `~4h` `#tech-debt` `#backend`
- [ ] Split `chat_view.dart` (700+ lines) by responsibility · `P2` `~4h` `#tech-debt` `#mobile`
- [ ] Parse ENCRYPTION_KEY once (dup in main.rs + state.rs) · `P3` `~30m` `#tech-debt` `#backend`
- [ ] Replace `buffer.remove(0)` O(n) with a deque/ring buffer · `P3` `~30m` `#tech-debt` `#reliability`
- [ ] Migrate Bedrock buffer from `Vec<u8>` to `bytes::BytesMut` · `P2` `~2h` `#tech-debt` `#ai`

### [LONG] Long-term security
- [ ] Vector-model-based prompt injection detection · `P3` `~1w` `#security` `#ai`
- [ ] Move root key from process memory to KMS (currently demo-only) · `P3` `~1w` `#security`
- [ ] Bedrock SigV4 auth instead of Bearer token · `P3` `~1d` `#security` `#ai`
- [ ] Expand mnemonic detection beyond 5 fixed phrases · `P3` `~2h` `#security`
- [ ] Third-party security audit + pentest (external) · `P1` `external` `#security`

---

## Milestone 8 — DevOps / Observability (P3)

### [OPS] Tooling
- [x] Docker + GitHub Actions → ECS, health checks, graceful shutdown, auto-migrations ✅ · `#devops`
- [ ] Add code-coverage tooling to CI · `P3` `~4h` `#devops` `#testing`
- [ ] Performance benchmarks (signing latency, presign concurrency) · `P3` `~2d` `#testing`

---

## Launch Timeline (from roadmap)

```
now ──── +2wk ──── +1mo ──── +2mo
 │         │         │          │
 ├ P0 fix  ├ P1 fix  ├ audit    ├ public beta
 ├ int.test├ recovery├ pentest  │
 └ internal└ perf    └ fix-round└ GA release
```
