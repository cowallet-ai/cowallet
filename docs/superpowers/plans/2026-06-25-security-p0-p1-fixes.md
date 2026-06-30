# CoWallet 安全修复 (P0/P1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复多方专家审查发现的 P0/P1 安全缺陷——使 2-of-3 MPC 钱包的"服务器作为风控关卡"和"会话访问控制"两个核心安全假设重新成立。

**Architecture:** 后端 Rust/axum 为主。修复分三层:(1) 访问控制——给所有 `/session/{id}/*` MPC 端点加会话归属校验;(2) 签名授权——服务器签名前强制接收结构化 `raw_tx`、重算 `msg_hash` 比对、调用策略引擎;(3) 配置/密码学卫生——删除硬编码密钥、禁止预签名 nonce 回收复用、修正 DKG `total_parties`。每个任务独立可测、独立可回滚。

**Tech Stack:** Rust (axum 0.8, sqlx 0.8, k256 0.13, alloy 1), PostgreSQL, Flutter/Dart FFI (flutter_rust_bridge 2.x)。

## Global Constraints

- 测试框架:Rust 用 `#[tokio::test]` + `cargo test -p <crate>`;集成测试放对应 crate 的 `tests/` 或模块内 `#[cfg(test)] mod tests`。
- 本项目为预生产阶段,**无需向后兼容**——可自由破坏接口,不加兼容垫片(见 MEMORY: no-backward-compat)。
- 所有数据库迁移为顺序编号 `.sql`,当前最高 `019`,新迁移从 `020` 起。迁移在 api-server 启动时经 `sqlx::migrate!("../migrations")` 自动执行。
- 提交粒度:每个任务结尾一次提交,信息用 `fix(security): ...` / `feat(security): ...` 前缀,结尾附 `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`。
- 当前分支 `release`;任何改动前先确认在工作分支而非直接动 `main`。
- 验证命令统一:`cargo check --workspace` 必须通过;`cargo clippy -- -D warnings` 不得新增告警。
- MPC 服务器参与方索引常量为 `SERVER_PARTY_INDEX`(值为 1),客户端设备为 Party 0。

## 任务依赖与执行顺序

```
Task 1 (会话归属 helper) ─┬─> Task 2 (get/abort/recv 加校验)
                          └─> Task 3 (send_message 加校验)
Task 4 (push/send 鉴权)   — 独立
Task 5 (预签名禁止回收)    — 独立
Task 6 (删除硬编码密钥)    — 独立
Task 7 (DKG total_parties) — 独立
Task 8 (签名前 raw_tx+策略 gate) — 依赖 Task 1 的校验风格,逻辑独立  ← P0 最关键
Task 9 (登录改挑战-响应)   — P1
Task 10 (userop 归属+冻结校验) — P1,依赖 Task 1 helper
Task 11 (HMAC 强制 + 独立密钥) — P1
```

Task 1–7 可并行批次执行;Task 8 单独审查;Task 9–11 为 P1 批次。

---

### Task 1: 抽取会话归属校验 helper

**Files:**
- Modify: `backend/api-server/src/routes/mpc.rs` (在文件顶部 `resolve_wallet_id` 之后新增 helper)
- Test: `backend/api-server/src/routes/mpc.rs`(模块内 `#[cfg(test)] mod ownership_tests`)

**Interfaces:**
- Produces: `async fn fetch_session_owner(db: &sqlx::PgPool, session_id: uuid::Uuid) -> Result<uuid::Uuid, StatusCode>` — 查 `mpc_sessions.user_id`,未找到返回 `Err(StatusCode::NOT_FOUND)`,DB 错误返回 `Err(StatusCode::INTERNAL_SERVER_ERROR)`。
- Produces: `fn claims_user_id(claims: &Claims) -> Result<uuid::Uuid, StatusCode>` — 解析 `claims.sub`,失败返回 `Err(StatusCode::UNAUTHORIZED)`。
- 现有 `get_backup_contribution`(`mpc.rs:470-540`)已是正确的归属校验范式,后续任务复用本 helper 统一逻辑。

- [ ] **Step 1: 写失败测试**

在 `mpc.rs` 末尾追加:

```rust
#[cfg(test)]
mod ownership_tests {
    use super::*;

    #[test]
    fn claims_user_id_parses_valid_uuid() {
        let claims = Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            device_id: "DEV0000000000001".to_string(),
            exp: 9999999999,
            iat: 0,
            token_type: "access".to_string(),
        };
        let uid = claims_user_id(&claims).expect("should parse");
        assert_eq!(uid.to_string(), "11111111-1111-1111-1111-111111111111");
    }

    #[test]
    fn claims_user_id_rejects_garbage() {
        let claims = Claims {
            sub: "not-a-uuid".to_string(),
            device_id: "DEV0000000000001".to_string(),
            exp: 9999999999,
            iat: 0,
            token_type: "access".to_string(),
        };
        assert_eq!(claims_user_id(&claims).unwrap_err(), StatusCode::UNAUTHORIZED);
    }
}
```

注意:`Claims` 的真实字段需先确认——打开 `backend/api-server/src/middleware/auth.rs` 查看 struct 定义,按实际字段调整测试构造(上面假设含 `sub/device_id/exp/iat/token_type`)。

- [ ] **Step 2: 运行测试,确认失败**

Run: `cargo test -p api-server claims_user_id 2>&1 | tail -20`
Expected: 编译失败,`cannot find function claims_user_id in this scope`。

- [ ] **Step 3: 实现 helper**

在 `mpc.rs` 的 `resolve_wallet_id` 函数(结束于第 36 行)之后插入:

```rust
/// Parse the authenticated user's UUID from JWT claims.
fn claims_user_id(claims: &Claims) -> Result<uuid::Uuid, StatusCode> {
    uuid::Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Fetch the owner (user_id) of an MPC session.
/// Returns NOT_FOUND if the session does not exist.
async fn fetch_session_owner(
    db: &sqlx::PgPool,
    session_id: uuid::Uuid,
) -> Result<uuid::Uuid, StatusCode> {
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM mpc_sessions WHERE id = $1"
    )
    .bind(session_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!("fetch_session_owner query failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(row.ok_or(StatusCode::NOT_FOUND)?.0)
}
```

- [ ] **Step 4: 运行测试,确认通过**

Run: `cargo test -p api-server ownership_tests 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`。

- [ ] **Step 5: 提交**

```bash
git add backend/api-server/src/routes/mpc.rs
git commit -m "feat(security): add session ownership helpers for MPC routes

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: get_session / abort_session / recv_messages 加归属校验 (修复 IDOR)

**Files:**
- Modify: `backend/api-server/src/routes/mpc.rs:175-220`(get_session、abort_session)、`mpc.rs:407-465`(recv_messages)

**Interfaces:**
- Consumes: `fetch_session_owner`、`claims_user_id`(Task 1)。
- 三个 handler 均需新增 `Extension(claims): Extension<Claims>` 参数。`Extension` 已在 `mpc.rs:6` 导入。

- [ ] **Step 1: 改 get_session 签名与校验**

替换 `mpc.rs:175-196` 的 `get_session` 整个函数为:

```rust
/// Get session status
pub async fn get_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let caller = claims_user_id(&claims)?;
    let owner = fetch_session_owner(db, id).await?;
    if owner != caller {
        tracing::warn!("User {} attempted to read session {} owned by {}", caller, id, owner);
        return Err(StatusCode::FORBIDDEN);
    }

    let row: (String, i32, Option<chrono::DateTime<Utc>>, Option<uuid::Uuid>) = sqlx::query_as(
        "SELECT status, current_round, last_activity, wallet_id FROM mpc_sessions WHERE id = $1"
    )
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(SessionResponse {
        session_id: id.to_string(),
        status: row.0,
        current_round: row.1,
        last_activity: row.2.map(|t| t.to_rfc3339()),
        wallet_id: row.3.map(|w| w.to_string()),
    }))
}
```

- [ ] **Step 2: 改 abort_session 签名与校验**

替换 `mpc.rs:199-220` 的 `abort_session` 整个函数为:

```rust
/// Abort a session
pub async fn abort_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<uuid::Uuid>,
) -> Result<StatusCode, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let caller = claims_user_id(&claims)?;

    let result = sqlx::query(
        "UPDATE mpc_sessions SET status = 'failed'
         WHERE id = $1 AND user_id = $2 AND status IN ('pending', 'active')"
    )
    .bind(id)
    .bind(caller)
    .execute(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::GONE);
    }

    tracing::info!("User {} aborted MPC session {}", caller, id);
    Ok(StatusCode::NO_CONTENT)
}
```

注:用 `AND user_id = $2` 而非先查再比对,确保原子性——非所有者的 abort 直接 0 行受影响返回 GONE,不泄露会话是否存在。

- [ ] **Step 3: 改 recv_messages 签名与校验**

替换 `mpc.rs:407-423` 的函数签名及会话存在性检查段(即 `pub async fn recv_messages(...)` 到 `if !exists { return Err(StatusCode::NOT_FOUND); }`)为:

```rust
pub async fn recv_messages(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<uuid::Uuid>,
    Query(query): Query<RecvQuery>,
) -> Result<Json<Vec<MessageResponse>>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    // Verify session exists AND belongs to the caller
    let caller = claims_user_id(&claims)?;
    let owner = fetch_session_owner(db, session_id).await?;
    if owner != caller {
        tracing::warn!("User {} attempted to read messages of session {} owned by {}", caller, session_id, owner);
        return Err(StatusCode::FORBIDDEN);
    }
```

(其余查询逻辑 `mpc.rs:425-465` 不变。)

- [ ] **Step 4: 编译并跑 MPC 路由测试**

Run: `cargo check -p api-server 2>&1 | tail -20`
Expected: 编译通过,无 error。
Run: `cargo test -p api-server mpc 2>&1 | tail -20`
Expected: 已有测试不回归(若无相关测试,确认编译通过即可)。

- [ ] **Step 5: 提交**

```bash
git add backend/api-server/src/routes/mpc.rs
git commit -m "fix(security): enforce session ownership on get/abort/recv MPC endpoints

Fixes IDOR allowing any authenticated user to read, abort, or poll
another user's MPC session via known session UUID.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: send_message 加归属校验 (修复最危险的 IDOR)

**Files:**
- Modify: `backend/api-server/src/routes/mpc.rs:239-294`(send_message 函数开头到 HMAC 段之前)

**Interfaces:**
- Consumes: `claims_user_id`(Task 1)。`send_message` 已有 `Extension(claims): Extension<Claims>` 参数(`mpc.rs:241`)和 `session_user_id`(`mpc.rs:259`),只是从未比对。

- [ ] **Step 1: 在 status 检查前插入归属校验**

在 `mpc.rs:259`(`let session_user_id = session.3;`)之后、`mpc.rs:261`(`// Session must be active`)之前插入:

```rust
    // Enforce session ownership: only the owner may drive their session.
    let caller = claims_user_id(&claims)?;
    if session_user_id != caller {
        tracing::warn!(
            "User {} attempted to send message to session {} owned by {}",
            caller, session_id, session_user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }
```

- [ ] **Step 2: 编译,确认通过**

Run: `cargo check -p api-server 2>&1 | tail -20`
Expected: 编译通过。注意 `session_user_id` 之前可能触发未使用变量告警,现在被使用,告警消失。

- [ ] **Step 3: clippy 检查**

Run: `cargo clippy -p api-server -- -D warnings 2>&1 | tail -20`
Expected: 无新增告警。

- [ ] **Step 4: 提交**

```bash
git add backend/api-server/src/routes/mpc.rs
git commit -m "fix(security): enforce session ownership on send_message

Previously any authenticated user could POST protocol messages to another
user's session and drive the server signing state machine (combined with
the unverified msg_hash this enabled signing for a victim's wallet).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: push/send 端点加鉴权 (修复未鉴权推送)

**Files:**
- Modify: `backend/api-server/src/routes/push.rs:75-102`(send_push 函数)

**Interfaces:**
- `Claims` 已在 `push.rs:11` 导入。`register_token`(`push.rs:36-40`)已正确提取 `claims: axum::Extension<Claims>` 作为范式。

- [ ] **Step 1: 改 send_push 签名,提取并校验 claims**

替换 `push.rs:75-82` 的函数签名与 user_id 解析段为:

```rust
async fn send_push(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Json(req): Json<SendPushRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.require_db().map_err(|_| err(StatusCode::SERVICE_UNAVAILABLE, "database not available"))?;
    let caller_id: uuid::Uuid = claims.0.sub.parse()
        .map_err(|_| err(StatusCode::UNAUTHORIZED, "invalid user id in token"))?;
    let user_id: uuid::Uuid = req.user_id.parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid user_id"))?;

    // A user may only send push notifications to their own devices.
    if caller_id != user_id {
        tracing::warn!("User {} attempted to push to user {}", caller_id, user_id);
        return Err(err(StatusCode::FORBIDDEN, "cannot send push to another user"));
    }
```

(原 `push.rs:79-81` 的 `let db = ...` 与 `let user_id = ...` 两行被上面替换覆盖,后续 `let fcm_server_key = ...` 起的逻辑不变。)

- [ ] **Step 2: 编译,确认通过**

Run: `cargo check -p api-server 2>&1 | tail -20`
Expected: 编译通过。

- [ ] **Step 3: 提交**

```bash
git add backend/api-server/src/routes/push.rs
git commit -m "fix(security): require auth + ownership on push/send endpoint

Previously any party knowing a user_id UUID could send arbitrary push
notifications to all of that user's devices (phishing vector).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: 预签名禁止回收复用 (修复 ECDSA nonce 复用 → 私钥泄露)

**Files:**
- Modify: `backend/api-server/src/services/presign_manager.rs:185-205`(cleanup_stale_reservations)

**Interfaces:**
- `cleanup_stale_reservations` 被 `spawn_background_task`(`presign_manager.rs:266`)周期调用,签名 `async fn cleanup_stale_reservations(&self) -> Result<u64, String>` 不变,仅改 SQL 语义。

- [ ] **Step 1: 改语义——过期预留标记 expired 而非释放回 available**

替换 `presign_manager.rs:185-205` 的整个 `cleanup_stale_reservations` 函数为:

```rust
    /// Expire presignatures that have been reserved too long (>10 min) without
    /// being consumed — likely from failed sessions.
    ///
    /// SECURITY: A reserved presignature's ephemeral nonce k_1 may have already
    /// been exposed to the client as R_1 = k_1*G during a partially-completed
    /// signing round. Reusing that k_1 for a second signature with a different
    /// message — combined with a repeated or attacker-controlled client nonce
    /// k_0 — reuses the aggregate nonce and leaks the private key via the
    /// classic ECDSA nonce-reuse equation. Therefore stale reservations are
    /// marked 'expired' (terminal) and NEVER returned to 'available'.
    pub async fn cleanup_stale_reservations(&self) -> Result<u64, String> {
        let result = sqlx::query(
            "UPDATE presignatures SET status = 'expired'
             WHERE status = 'reserved'
             AND reserved_at < NOW() - INTERVAL '10 minutes'
             AND consumed_at IS NULL"
        )
        .execute(&self.db)
        .await
        .map_err(|e| format!("DB stale cleanup failed: {}", e))?;

        let count = result.rows_affected();
        if count > 0 {
            tracing::info!("Expired {} stale reserved presignatures (never reused)", count);
        }
        Ok(count)
    }
```

- [ ] **Step 2: 写回归测试断言不会出现 available 复用**

由于该函数依赖真实 DB,这里加一个文档级断言测试——在 `presign_manager.rs` 末尾追加:

```rust
#[cfg(test)]
mod nonce_safety_tests {
    /// Guards against regressing to releasing stale reservations back to
    /// 'available'. If someone reintroduces "SET status = 'available'" in the
    /// stale-cleanup SQL, this test's source-level check fails.
    #[test]
    fn stale_cleanup_never_releases_to_available() {
        let src = include_str!("presign_manager.rs");
        // Find the cleanup_stale_reservations function body.
        let start = src.find("pub async fn cleanup_stale_reservations")
            .expect("function must exist");
        let body = &src[start..start + 600.min(src.len() - start)];
        assert!(
            !body.contains("'available'"),
            "cleanup_stale_reservations must NOT release nonces back to 'available' (ECDSA nonce-reuse risk)"
        );
        assert!(body.contains("'expired'"), "stale reservations must be marked 'expired'");
    }
}
```

- [ ] **Step 3: 运行测试,确认通过**

Run: `cargo test -p api-server nonce_safety 2>&1 | tail -20`
Expected: `test result: ok. 1 passed`。

- [ ] **Step 4: 提交**

```bash
git add backend/api-server/src/services/presign_manager.rs
git commit -m "fix(security): never recycle reserved presignature nonces

Stale reserved presignatures are now marked 'expired' (terminal) instead
of released back to 'available'. Reusing an exposed k_1 nonce risks ECDSA
private-key recovery. Adds a source-level regression guard.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: 删除硬编码兜底密钥 (fail-fast)

**Files:**
- Modify: `backend/api-server/src/services/claude.rs:146-162`(from_env)
- Modify: `backend/api-server/src/state.rs:134-142`(covalent_api_key)

**Interfaces:**
- `AiClient::from_env() -> AiResult<Self>` 签名不变;缺 key 时改为返回 `Err`。
- `covalent_api_key` 在 `state.rs` 为 `Option<String>`,缺失时保持 `None`(已有 503 降级路径)。

- [ ] **Step 1: 改 from_env,缺 DEEPSEEK_API_KEY 即报错**

替换 `claude.rs:146-153` 的 api_key 获取段为:

```rust
    pub fn from_env() -> AiResult<Self> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AiError::Config("DEEPSEEK_API_KEY not set".into()))?;
```

注意:确认 `AiError` 枚举有 `Config(String)` 变体——打开 `claude.rs` 顶部或其 error 定义模块核对;若变体名不同(如 `Configuration`/`Missing`),改用实际变体。若无合适变体,新增 `Config(String)`。

- [ ] **Step 2: 删除 state.rs 的 Covalent 硬编码兜底**

替换 `state.rs:134-137`:

```rust
        let covalent_api_key = std::env::var("COVALENT_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| Some("cqt_rQGHc9RXCJfWxFDffW6qp7xHqcYG".to_string()));
```

为:

```rust
        let covalent_api_key = std::env::var("COVALENT_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
```

- [ ] **Step 3: 编译,确认通过**

Run: `cargo check -p api-server 2>&1 | tail -20`
Expected: 编译通过。`from_env` 的调用方 `state.rs:103-110` 已用 `match ... Err(e) => warn` 处理,缺 key 时 AI 降级为 None(已有路径),无需改调用方。

- [ ] **Step 4: 全仓库确认无残留硬编码密钥**

Run: `grep -rn "cqt_rQGHc9\|sk-3a272cf5" backend/ crates/ 2>/dev/null`
Expected: 无输出(空)。

- [ ] **Step 5: 提交**

```bash
git add backend/api-server/src/services/claude.rs backend/api-server/src/state.rs
git commit -m "fix(security): remove hardcoded fallback API keys

DeepSeek and Covalent keys no longer have committed fallback values.
Missing DEEPSEEK_API_KEY now disables the AI provider (existing graceful
path); missing COVALENT_API_KEY yields 503 on balance endpoints.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 7: 修正 DKG total_parties (2 → 3)

**Files:**
- Modify: `crates/ffi-mobile/src/api.rs:143-157`(dkg_session_new)

**Interfaces:**
- `dkg_session_new(party_index: u16) -> Result<FfiDkgSession, String>` 签名不变。仅修正 `SessionConfig.total_parties` 从 `2` 到 `3`,与服务端 `mpc_participant`(用 3)和 `generate_wallet`/`import_device_shard`(用 3)一致。

- [ ] **Step 1: 改 total_parties**

替换 `crates/ffi-mobile/src/api.rs:146-151` 的 `SessionConfig` 构造为:

```rust
    let config = SessionConfig {
        session_id: session_id.clone(),
        threshold: 2,
        total_parties: 3,
        party_index,
    };
```

- [ ] **Step 2: 全仓库核对 total_parties 一致性**

Run: `grep -rn "total_parties" crates/ffi-mobile/src/api.rs backend/api-server/src/services/mpc_participant/ 2>/dev/null`
Expected: 所有 DKG/sign 相关处均为 `3`(2-of-3),无残留 `2`。

- [ ] **Step 3: 编译 ffi-mobile**

Run: `cargo check -p ffi-mobile 2>&1 | tail -20`
Expected: 编译通过。

- [ ] **Step 4: 提交**

```bash
git add crates/ffi-mobile/src/api.rs
git commit -m "fix(mpc): correct DKG total_parties from 2 to 3 for 2-of-3 TSS

The device-side dkg_session_new hardcoded total_parties=2 while the server
participant and wallet generation use 3, breaking Shamir polynomial degree
and Lagrange interpolation in a real distributed DKG.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 8: 签名前强制 raw_tx 重算哈希 + 策略校验 (P0 最关键)

> **⚠️ 本任务改动 MPC 签名核心路径,务必单独审查。** 当前服务端对客户端提供的任意 `msg_hash` 盲签,且 `policy-engine` 从未在签名路径被调用——策略限额可被完全绕过。本任务在服务端签名前插入"重算哈希比对 + 策略评估"强制关卡。

**Files:**
- Modify: `backend/api-server/src/services/mpc_participant/mod.rs:450-518`(process_sign_message Round 1 段)
- Modify: `backend/api-server/src/services/mpc_participant/mod.rs:757-784`(extract_msg_hash → 改为接收结构化交易)
- 参考(只读):`crates/policy-engine/src/` 的评估入口签名;`crates/chain-evm/src/transaction.rs` 的交易哈希计算。

**Interfaces:**
- 前置确认:实现前必须先用 `codegraph_explore "policy-engine evaluate transaction limit"` 和 `codegraph_explore "chain-evm transaction signing hash keccak eip155"` 读出:(a) policy-engine 的真实评估函数签名与返回类型;(b) chain-evm 中从 raw_tx 计算签名哈希的现成函数。下面代码中的 `policy_engine.evaluate(...)` 和 `chain_evm::transaction::signing_hash(...)` 为占位接口名,**实现时替换为真实签名**。
- 客户端协议变更:Round 1 的 JSON wrapper 从 `{"msg_hash":[...]}` 改为 `{"raw_tx":"0x...","chain_id":1,"msg_hash":[...]}`。msg_hash 仍传用于交叉校验,但不再被信任为权威。

- [ ] **Step 1: 探查真实接口(不写代码,先读)**

Run: `grep -rn "pub fn\|pub async fn" crates/policy-engine/src/ | grep -i "eval\|check\|assess\|limit" | head -20`
Run: `grep -rn "pub fn\|signing_hash\|tx_hash\|keccak" crates/chain-evm/src/transaction.rs | head -20`
记录真实函数签名,用于后续步骤替换占位名。同时确认 `AppState` 是否已持有 policy engine 实例(`grep -rn "policy" backend/api-server/src/state.rs`);若未持有,本任务需先把 policy engine 加入 AppState 与 MpcParticipant(扩展为 Step 1b)。

- [ ] **Step 1b(条件性): 若 MpcParticipant 未持有 policy engine,先注入**

若 Step 1 发现 `MpcParticipant` 无法访问策略引擎,在 `MpcParticipant` struct 增加字段 `policy: Arc<PolicyEngine>`(类型以真实为准),并在 `state.rs` 构造 participant 处传入。这一步是 Task 8 的前置,不单独提交。

- [ ] **Step 2: 把 extract_msg_hash 改为结构化交易提取**

替换 `mpc_participant/mod.rs:757-784` 的 `extract_msg_hash` 为新函数 `extract_signing_request`:

```rust
    /// Parsed signing request from the client's Round 1 payload.
    /// The client MUST send the structured transaction so the server can
    /// independently recompute the signing hash and enforce policy.
    fn extract_signing_request(&self, payload: &[u8]) -> Result<SigningRequest, String> {
        let json: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|_| "round 1 payload must be JSON with raw_tx + chain_id".to_string())?;

        let raw_tx_hex = json.get("raw_tx").and_then(|v| v.as_str())
            .ok_or("missing raw_tx in signing request")?;
        let raw_tx = hex::decode(raw_tx_hex.trim_start_matches("0x"))
            .map_err(|_| "raw_tx is not valid hex".to_string())?;

        let chain_id = json.get("chain_id").and_then(|v| v.as_u64())
            .ok_or("missing chain_id in signing request")?;

        // The client-claimed hash, used only for a cross-check (never trusted).
        let claimed_hash: Option<[u8; 32]> = json.get("msg_hash")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                let bytes: Vec<u8> = arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect();
                if bytes.len() == 32 {
                    let mut a = [0u8; 32];
                    a.copy_from_slice(&bytes);
                    Some(a)
                } else { None }
            });

        Ok(SigningRequest { raw_tx, chain_id, claimed_hash })
    }
```

并在文件合适位置(struct 定义区)新增:

```rust
/// A structured signing request extracted from the client's Round 1 payload.
struct SigningRequest {
    raw_tx: Vec<u8>,
    chain_id: u64,
    claimed_hash: Option<[u8; 32]>,
}
```

- [ ] **Step 3: 在 process_sign_message Round 1 插入重算 + 策略 gate**

替换 `mpc_participant/mod.rs:467`(`let msg_hash = self.extract_msg_hash(payload)?;`)为:

```rust
                // SECURITY GATE: parse the structured tx, recompute the hash
                // server-side, and enforce policy before contributing a signature.
                let req = self.extract_signing_request(payload)?;

                // 1) Recompute the signing hash from the raw transaction.
                //    Replace `chain_evm::transaction::signing_hash` with the real
                //    function found in Step 1.
                let msg_hash = chain_evm::transaction::signing_hash(&req.raw_tx, req.chain_id)
                    .map_err(|e| format!("failed to recompute signing hash: {}", e))?;

                // 2) Cross-check the client's claimed hash, if provided.
                if let Some(claimed) = req.claimed_hash {
                    if claimed != msg_hash {
                        tracing::warn!(
                            "Sign hash mismatch session={} (client claim != server recompute)",
                            session_id
                        );
                        return Err("msg_hash does not match raw_tx".into());
                    }
                }

                // 3) Evaluate policy (limits, approvals, risk). Replace
                //    `self.policy.evaluate` with the real signature from Step 1.
                let decision = self.policy
                    .evaluate(user_id, wallet_id, &req.raw_tx, req.chain_id)
                    .await
                    .map_err(|e| format!("policy evaluation failed: {}", e))?;
                if !decision.allowed {
                    tracing::warn!(
                        "Policy denied signing for session={} user={}: {}",
                        session_id, user_id, decision.reason
                    );
                    return Err(format!("policy denied: {}", decision.reason));
                }
```

注意:`wallet_id` 在此处尚未从 meta 取出(原代码在 `mod.rs:470-474` 才取)。需把 meta 取 `wallet_id` 的代码段上移到本 gate 之前。`decision.allowed` / `decision.reason` 字段名以 policy-engine 真实返回类型为准(Step 1 已确认)。

- [ ] **Step 4: 移除 payload 末尾 32 字节剥离逻辑(已不适用)**

原 `mod.rs:504-509` 剥离客户端附加的 msg_hash:

```rust
                let round1_payload = if payload.len() > 32 {
                    &payload[..payload.len() - 32]
                } else {
                    payload
                };
```

现在 payload 是 JSON 而非"R_0 + 末尾 hash",需改为从 JSON 中取 R_0 字段。在 Step 2 的 `SigningRequest` 增加 `r0: Vec<u8>` 字段(从 JSON `"r0"` hex 解码),并把 `round1_payload` 替换为 `&req.r0`。客户端协议需同步在 Round 1 JSON 中携带 `"r0":"0x..."`(序列化的 SignRound1Message)。**这是客户端协议的破坏性变更,记入下方"客户端协同改动"。**

- [ ] **Step 5: 编译,确认通过**

Run: `cargo check -p api-server 2>&1 | tail -30`
Expected: 编译通过。若 policy/chain-evm 函数名占位未替换,会报 `cannot find function`——按 Step 1 探查到的真实名修正。

- [ ] **Step 6: 写单元测试——hash 不匹配必拒绝**

在 `mpc_participant/mod.rs` 末尾 `#[cfg(test)] mod tests` 内追加(若已有 tests mod 则并入):

```rust
    #[test]
    fn signing_request_rejects_non_json() {
        let p = test_participant(); // helper that builds a MpcParticipant for tests
        assert!(p.extract_signing_request(b"not json").is_err());
    }

    #[test]
    fn signing_request_requires_raw_tx() {
        let p = test_participant();
        let payload = br#"{"chain_id":1}"#;
        assert!(p.extract_signing_request(payload).is_err());
    }
```

若构造 `MpcParticipant` 需要 DB,改为把 `extract_signing_request` 重构为不依赖 `&self` 的关联函数(纯解析逻辑无需实例),测试直接调用 `MpcParticipant::extract_signing_request(payload)`。优先采用此重构,使解析逻辑可单测。

- [ ] **Step 7: 运行测试,确认通过**

Run: `cargo test -p api-server signing_request 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`。

- [ ] **Step 8: 提交**

```bash
git add backend/api-server/src/services/mpc_participant/
git commit -m "fix(security): enforce raw_tx hash recompute + policy gate before signing

The server now requires a structured raw_tx + chain_id in MPC sign Round 1,
recomputes the signing hash server-side (rejecting client/server mismatch),
and evaluates policy-engine limits before contributing its signature share.
Previously the server blind-signed any client-supplied 32-byte hash and the
policy engine was never consulted on the signing path.

BREAKING (client protocol): sign Round 1 payload is now JSON
{raw_tx, chain_id, r0, msg_hash} instead of R_0+appended-hash.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

**客户端协同改动(记入,但属移动端范围,本计划不实施):** `mobile/lib/` 的签名发起代码需把 Round 1 payload 改为 JSON `{raw_tx, chain_id, r0, msg_hash}`。FFI 层 `crates/ffi-mobile/src/api.rs` 的签名入口需暴露 raw_tx 给 Dart。

---

### Task 9: 登录改为挑战-响应 (P1)

> **本任务需要客户端协同改动(设备私钥签名挑战),属跨端工作。** 后端先落地挑战签发 + 验签端点;客户端改造另行排期。建议执行前与产品确认设备公钥的注册时机(注册时登记设备 secp256k1 公钥)。

**Files:**
- Create: `backend/migrations/020_login_challenges.sql`
- Modify: `backend/api-server/src/routes/auth.rs`(新增 `request_challenge` 端点,改 `login` 验签)
- Modify: `backend/api-server/src/routes/auth.rs` 的 `routes()`(注册新路由)

**Interfaces:**
- Produces: `POST /api/v1/auth/challenge` → `{challenge: String, expires_in: u64}`,签发随机 nonce 并存 DB。
- Produces: `POST /api/v1/auth/login` 改为接收 `{device_id, challenge, signature}`,用注册时登记的设备公钥验签。
- 前置确认:`grep -rn "device_pubkey\|public_key" backend/migrations/ backend/api-server/src/routes/auth.rs` 确认 users 表是否已存设备公钥;若无,本任务需扩展注册流程登记公钥(扩为 Step 0)。

- [ ] **Step 1: 写迁移**

创建 `backend/migrations/020_login_challenges.sql`:

```sql
CREATE TABLE login_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id TEXT NOT NULL,
    challenge BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '2 minutes',
    consumed BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX idx_login_challenges_device ON login_challenges(device_id, consumed, expires_at);
```

- [ ] **Step 2: 实现 challenge 签发端点**

在 `auth.rs` 新增:

```rust
#[derive(Deserialize)]
struct ChallengeRequest {
    device_id: String,
}

#[derive(Serialize)]
struct ChallengeResponse {
    challenge: String, // hex
    expires_in: u64,
}

async fn request_challenge(
    State(state): State<AppState>,
    Json(body): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    use rand::RngCore;
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let mut nonce = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut nonce);

    sqlx::query(
        "INSERT INTO login_challenges (device_id, challenge) VALUES ($1, $2)"
    )
    .bind(&body.device_id)
    .bind(&nonce[..])
    .execute(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ChallengeResponse {
        challenge: hex::encode(nonce),
        expires_in: 120,
    }))
}
```

- [ ] **Step 3: 改 login 为验签**

替换 `auth.rs:385-450` 的 `LoginRequest` 与 `login`:

```rust
#[derive(Deserialize)]
struct LoginRequest {
    device_id: String,
    challenge: String,  // hex, must match an unconsumed challenge
    signature: String,  // hex secp256k1 signature over the challenge bytes
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let challenge_bytes = hex::decode(&body.challenge).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Atomically consume a valid, unexpired challenge for this device.
    let consumed = sqlx::query(
        "UPDATE login_challenges SET consumed = TRUE
         WHERE device_id = $1 AND challenge = $2 AND consumed = FALSE AND expires_at > NOW()"
    )
    .bind(&body.device_id)
    .bind(&challenge_bytes)
    .execute(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if consumed.rows_affected() == 0 {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Load the device's registered public key and verify the signature.
    let (user_id, pubkey): (uuid::Uuid, Vec<u8>) = sqlx::query_as(
        "SELECT id, device_pubkey FROM users WHERE device_id = $1"
    )
    .bind(&body.device_id)
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    verify_secp256k1_signature(&pubkey, &challenge_bytes, &body.signature)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let token_pair = issue_token_pair(&user_id.to_string(), &body.device_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse {
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
        token_type: token_pair.token_type,
        user_id: user_id.to_string(),
    }))
}
```

并实现 `verify_secp256k1_signature(pubkey: &[u8], msg: &[u8], sig_hex: &str) -> Result<(), String>`,用 `k256` crate 验证签名。具体验签算法(对挑战原文 sha256 后验签 vs 直接验签)需与客户端约定,实现时固定一种。

- [ ] **Step 4: 注册路由**

在 `auth.rs` 的 `routes()` 函数中,`login` 路由旁新增:

```rust
        .route("/challenge", post(request_challenge))
```

- [ ] **Step 5: 编译**

Run: `cargo check -p api-server 2>&1 | tail -20`
Expected: 编译通过(若 users 表无 `device_pubkey` 列,Step 0 须先补迁移登记设备公钥)。

- [ ] **Step 6: 提交**

```bash
git add backend/migrations/020_login_challenges.sql backend/api-server/src/routes/auth.rs
git commit -m "feat(security): challenge-response login replacing device_id bearer

Login now requires signing a server-issued nonce with the device's
registered secp256k1 key. A stolen device_id alone no longer grants tokens.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 10: submit_signed_userop 加归属 + 冻结校验 (P1)

**Files:**
- Modify: `backend/api-server/src/routes/tx.rs:720-757`(submit_signed_userop 开头)

**Interfaces:**
- Consumes: 复用 `submit`(`tx.rs:68-87`)已有的冻结检查范式。`_claims` 当前被丢弃(`tx.rs:722`),改为使用。
- 前置:`grep -n "fn submit\b" -A40 backend/api-server/src/routes/tx.rs` 读出 `submit` 的冻结检查 SQL,复用同样逻辑。

- [ ] **Step 1: 解析 sender 后校验钱包归属与冻结状态**

把 `tx.rs:722` 的 `_claims: axum::Extension<Claims>` 改为 `Extension(claims): axum::Extension<Claims>`,并在 `tx.rs:756`(`let sender = parse_address("sender")?;`)之后插入:

```rust
    // Authorize: the sender wallet must belong to the caller and not be frozen.
    let db = state.require_db().map_err(|_| rpc_error("database not available"))?;
    let caller_id: uuid::Uuid = claims.sub.parse()
        .map_err(|_| rpc_error("invalid user id in token"))?;
    let sender_bytes = sender.as_slice().to_vec();
    let wallet: Option<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT user_id, status FROM wallets WHERE eth_address = $1"
    )
    .bind(&sender_bytes)
    .fetch_optional(db)
    .await
    .map_err(|_| rpc_error("wallet lookup failed"))?;
    match wallet {
        Some((owner, status)) => {
            if owner != caller_id {
                return Err(rpc_error("sender wallet does not belong to caller"));
            }
            if status == "frozen" {
                return Err(rpc_error("wallet is frozen"));
            }
        }
        None => return Err(rpc_error("unknown sender wallet")),
    }
```

注意:确认 `wallets` 表存以太坊地址的列名与编码(`eth_address` BYTEA,见 `mpc.rs:27` 的 `WHERE eth_address = $1` + `addr_bytes`)。`rpc_error` 返回类型为 `(StatusCode, Json<ErrorResponse>)`,与本 handler 返回类型一致。

- [ ] **Step 2: 编译**

Run: `cargo check -p api-server 2>&1 | tail -20`
Expected: 编译通过。

- [ ] **Step 3: 提交**

```bash
git add backend/api-server/src/routes/tx.rs
git commit -m "fix(security): authorize submit_signed_userop by wallet ownership + freeze

Previously the endpoint forwarded any signed UserOperation to the bundler
without checking the sender wallet belongs to the caller or is frozen,
bypassing the freeze risk control.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 11: MPC HMAC 强制校验 + 独立密钥 (P1)

**Files:**
- Modify: `backend/api-server/src/routes/mpc.rs:277-294`(HMAC 验证段)、`mpc.rs:326-327`(participant 触发前)

**Interfaces:**
- 新增环境变量 `MPC_HMAC_KEY`(独立于 `JWT_SECRET`),记入 `.env.example`。
- 改动:HMAC 校验失败(`verified == false`)时,若消息发往服务器(Party 1),拒绝处理。

- [ ] **Step 1: 改 HMAC 用独立密钥**

替换 `mpc.rs:280-281` 的:

```rust
        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
```

为:

```rust
        let hmac_key = std::env::var("MPC_HMAC_KEY")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
```

并把 `mpc.rs:282`、`mpc.rs:285` 中的 `jwt_secret` 全部改为 `hmac_key`。

- [ ] **Step 2: 发往服务器的消息强制 HMAC 通过**

在 `mpc.rs:327`(`if body.to_party == 1 {`)之后、`if let Some(participant)` 之前插入:

```rust
        // Messages that drive the server signing state machine MUST be authenticated.
        if !verified {
            tracing::warn!("Rejected unverified message to server for session {}", session_id);
            return Err(StatusCode::UNAUTHORIZED);
        }
```

- [ ] **Step 3: 记录新环境变量**

在 `.env.example` 增加一行:

```
MPC_HMAC_KEY=change_me_independent_32_byte_min_hmac_key
```

- [ ] **Step 4: 编译**

Run: `cargo check -p api-server 2>&1 | tail -20`
Expected: 编译通过。

- [ ] **Step 5: 提交**

```bash
git add backend/api-server/src/routes/mpc.rs .env.example
git commit -m "fix(security): enforce MPC HMAC on server-bound messages, separate key

HMAC now uses a dedicated MPC_HMAC_KEY instead of JWT_SECRET, and messages
addressed to the server participant are rejected unless HMAC verifies.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-Review

**1. Spec coverage(对照 5 份专家报告的 P0/P1 发现):**

| 发现 | 严重度 | 任务 |
|------|--------|------|
| 签名路径不校验交易/策略被绕过 | P0 | Task 8 |
| send_message IDOR | P0 | Task 3 |
| get/abort/recv IDOR | P0 | Task 2 |
| push/send 无鉴权 | P0 | Task 4 |
| 预签名 nonce 回收复用 | P0 | Task 5 |
| 硬编码密钥 | P0 | Task 6 |
| DKG total_parties 不一致 | P0/P1 | Task 7 |
| 登录非挑战-响应 | P1 | Task 9 |
| userop 无归属/冻结校验 | P1 | Task 10 |
| HMAC 形同虚设 + 复用 JWT_SECRET | P1 | Task 11 |

**明确不在本计划内(需密码学审计/产品决策,故意排除):**
- 生产路径全密钥重建(`generate_local`/`sign_local`)→ 需真实分布式 DKG 替换,研究级工作,单独立项。
- DKG Feldman 验证绕过(`dkg.rs:315`)、Reshare 无 VSS 验证 → 属 mpc-core 协议层,建议交第三方密码学审计后统一修。
- 手写 Paillier/范围证明审计 → 第三方专项。
- WS query-param token 泄露 + ws-ticket → 跨端协议改动,建议与 Task 9 客户端改造合并排期。
- 移动端 iOS Secure Enclave 误用、Keychain 可访问性、占位实现(estimate_gas/se_manager)→ 移动端独立计划。
- ENCRYPTION_KEY 已泄露密钥轮转 → 运维动作,非代码,执行前由人工确认生产值。

**2. Placeholder scan:** Task 8 含两处显式标注的占位接口名(`policy_engine.evaluate`、`chain_evm::transaction::signing_hash`),已在 Step 1 要求先探查真实签名再替换——这是受控的、有探查步骤兜底的占位,非计划失败。其余任务代码均为可直接落地的完整实现。

**3. Type consistency:** `claims_user_id`/`fetch_session_owner`(Task 1)在 Task 2/3/10 一致复用;`SigningRequest` 结构在 Task 8 内部自洽;`rpc_error` 返回类型在 Task 10 与 tx.rs 现有一致。

## 执行风险提示

- Task 8、Task 9 涉及**客户端协议破坏性变更**,后端单独落地会导致现有移动端签名/登录失败,直到客户端同步改造。若要保持端到端可用,Task 8/9 应与移动端改造同批次执行。
- Task 1–7 为纯后端、不破坏客户端协议(Task 7 仅修正本就错误的参数),可安全独立落地。
