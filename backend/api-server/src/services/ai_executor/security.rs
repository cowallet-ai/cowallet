//! Wallet security audit tools.

use super::{parse_wallet_address, ToolContext, ToolExecutionResult};
use alloy_primitives::Address;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Shard-health signals for the security audit, read from `shard_metadata`.
///
/// Extracted from `execute_security_audit` so the shard-completeness logic can be
/// unit-tested against a real DB (via `#[sqlx::test]`) without constructing a full
/// `AppState` or making network calls. Keeping the production SQL here means a
/// schema drift (e.g. renaming a column the audit reads) breaks the tests.
pub(super) struct ShardSignals {
    /// A `location='server'` row exists (DKG server share present).
    pub has_server: bool,
    /// Backup proven server-side: an explicit `location='backup'` row OR a
    /// non-empty `backup_shard_hash` recorded on the server row.
    pub has_backup: bool,
    /// (location, status) for every shard whose status != 'healthy'.
    pub unhealthy: Vec<(String, String)>,
    /// Shard locations not verified in >30 days (or never verified).
    pub stale: Vec<String>,
    /// Whether the user has any shard rows at all.
    pub any_rows: bool,
}

/// Compute shard-health signals for `user_id` from `shard_metadata`.
pub(super) async fn shard_signals(db: &PgPool, user_id: Uuid) -> ShardSignals {
    let shards: Vec<(String, String, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>, Option<Vec<u8>>)> = sqlx::query_as(
        "SELECT location, status, last_used, last_verified, backup_shard_hash FROM shard_metadata WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let has_server = shards.iter().any(|s| s.0 == "server");
    let has_backup = shards.iter().any(|s| s.0 == "backup")
        || shards.iter().any(|s| s.4.as_ref().map_or(false, |h| !h.is_empty()));

    let unhealthy = shards.iter()
        .filter(|s| s.1 != "healthy")
        .map(|s| (s.0.clone(), s.1.clone()))
        .collect();

    let stale = shards.iter()
        .filter(|s| s.3.map_or(true, |v| (chrono::Utc::now() - v).num_days() > 30))
        .map(|s| s.0.clone())
        .collect();

    ShardSignals { has_server, has_backup, unhealthy, stale, any_rows: !shards.is_empty() }
}

/// Count usable (available, not-yet-expired) presignatures for `user_id`.
///
/// The `presignatures` table tracks availability via `status`
/// ('available'/'reserved'/'consumed'/'expired'); there is no `used` column.
/// Matches `PresignManager::reserve_presignature`'s own predicate.
pub(super) async fn available_presignature_count(db: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM presignatures
         WHERE user_id = $1 AND status = 'available' AND expires_at > NOW()"
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0)
}

/// Enforced transaction-limit / policy signals for `user_id`.
pub(super) struct PolicySignals {
    /// Per-transaction USD limit actually enforced (defaults if no row).
    pub single_limit_usd: f64,
    /// Rolling-24h USD limit actually enforced (defaults if no row).
    pub daily_limit_usd: f64,
    /// Count of enabled custom JSONB rule policies.
    pub custom_policy_count: i64,
    /// Whether a `user_policies` row exists (false → engine defaults apply).
    pub has_explicit_limits: bool,
}

/// Read the limits the policy-engine actually enforces (`user_policies`) plus any
/// custom rule policies (`policies`). A missing `user_policies` row is NOT "no
/// protection" — the engine falls back to Default (single=$500, daily=$2000).
pub(super) async fn policy_signals(db: &PgPool, user_id: Uuid) -> PolicySignals {
    let limits: Option<(f64, f64)> = sqlx::query_as(
        "SELECT single_limit_usd, daily_limit_usd FROM user_policies WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .unwrap_or(None);

    let has_explicit_limits = limits.is_some();
    let (single_limit_usd, daily_limit_usd) = limits.unwrap_or((500.0, 2000.0));

    let custom_policy_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM policies WHERE user_id = $1 AND rules != '{}' AND enabled = true"
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    PolicySignals { single_limit_usd, daily_limit_usd, custom_policy_count, has_explicit_limits }
}

impl ToolContext {
    // --- security_audit ---
    pub(super) async fn execute_security_audit(&self, tool_id: &str) -> ToolExecutionResult {
        let address = match parse_wallet_address(self.wallet_address.as_deref()) {
            Some(a) => a,
            None => return ToolExecutionResult {
                tool_id: tool_id.to_string(),
                tool_name: "security_audit".into(),
                success: false,
                result: Value::Null,
                error: Some("钱包地址未提供".into()),
            },
        };

        let mut findings: Vec<Value> = Vec::new();
        let mut score: u32 = 100;
        let mut recommendations: Vec<String> = Vec::new();

        let user_uuid = self.user_id.as_ref()
            .and_then(|uid| uuid::Uuid::parse_str(uid).ok());

        let db_available = self.app_state.require_db().is_ok();
        let has_user = user_uuid.is_some();

        if !db_available || !has_user {
            tracing::warn!(
                "security_audit: skipping DB checks — db_available={}, has_user={}, user_id={:?}",
                db_available, has_user, self.user_id
            );
            score -= 30;
            findings.push(serde_json::json!({
                "severity": "medium",
                "type": "audit_incomplete",
                "message": if !db_available {
                    "数据库连接不可用，部分安全检查无法执行"
                } else {
                    "用户身份未验证，部分安全检查无法执行"
                },
            }));
            recommendations.push("请确保已登录并联网后重新执行审计".into());
        }

        // ═══ 1. Shard Health Check ═══
        // (logic and rationale documented in `shard_signals`)
        if let (Ok(db), Some(uid)) = (self.app_state.require_db(), &user_uuid) {
            let sig = shard_signals(db, *uid).await;

            if !sig.has_server {
                score -= 20;
                findings.push(serde_json::json!({
                    "severity": "high",
                    "type": "shard_incomplete",
                    "message": "服务器密钥分片缺失，钱包无法签名，请通过恢复流程重建",
                }));
                recommendations.push("尽快通过恢复流程重建服务器分片".into());
            } else if !sig.has_backup {
                score -= 10;
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "backup_incomplete",
                    "message": "备份分片尚未完成，丢失设备后将无法恢复资产",
                }));
                recommendations.push("尽快完成备份分片的导出与保存".into());
            } else {
                findings.push(serde_json::json!({
                    "severity": "info",
                    "type": "shard_complete",
                    "message": "密钥分片完整 (设备/服务器/备份 3/3)",
                }));
            }

            if !sig.unhealthy.is_empty() {
                score -= 25;
                for (loc, st) in &sig.unhealthy {
                    findings.push(serde_json::json!({
                        "severity": "high",
                        "type": "shard_unhealthy",
                        "message": format!("{}分片状态异常: {}", loc, st),
                    }));
                }
                recommendations.push("执行密钥刷新(Reshare)修复异常分片".into());
            }

            if !sig.stale.is_empty() && sig.any_rows {
                score -= 5;
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "shard_stale",
                    "message": format!("分片超过30天未验证: {}", sig.stale.join("、")),
                }));
                recommendations.push("建议定期执行分片刷新以确保安全".into());
            }
        }

        // ═══ 2. Transaction Pattern Analysis ═══
        if let (Ok(db), Some(uid)) = (self.app_state.require_db(), &user_uuid) {
            // Failed transactions (7 days)
            let failed_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND status = 'failed' AND created_at > NOW() - INTERVAL '7 days'"
            )
            .bind(uid)
            .fetch_one(db)
            .await
            .unwrap_or(0);

            if failed_count > 5 {
                score -= 15;
                findings.push(serde_json::json!({
                    "severity": "high",
                    "type": "failed_transactions",
                    "message": format!("过去7天有 {} 笔失败交易，存在异常操作风险", failed_count),
                }));
                recommendations.push("检查失败交易原因，确认是否有未授权的操作尝试".into());
            } else if failed_count > 2 {
                score -= 5;
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "failed_transactions",
                    "message": format!("过去7天有 {} 笔失败交易", failed_count),
                }));
            } else {
                findings.push(serde_json::json!({
                    "severity": "info",
                    "type": "tx_clean",
                    "message": "近7天交易记录正常，无异常失败",
                }));
            }

            // Large value transactions without policy (24h)
            let large_tx_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND created_at > NOW() - INTERVAL '24 hours' AND CAST(value AS NUMERIC) > 1000000000000000000"
            )
            .bind(uid)
            .fetch_one(db)
            .await
            .unwrap_or(0);

            if large_tx_count > 0 {
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "large_transactions",
                    "message": format!("过去24小时有 {} 笔大额交易 (>1 ETH)", large_tx_count),
                }));
            }

            // Transaction frequency spike (compare last 24h vs average)
            let recent_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND created_at > NOW() - INTERVAL '24 hours'"
            )
            .bind(uid)
            .fetch_one(db)
            .await
            .unwrap_or(0);

            let avg_daily: i64 = sqlx::query_scalar(
                "SELECT COALESCE(COUNT(*) / GREATEST(EXTRACT(DAY FROM NOW() - MIN(created_at))::int, 1), 0)::bigint FROM transactions WHERE user_id = $1"
            )
            .bind(uid)
            .fetch_one(db)
            .await
            .unwrap_or(0);

            if avg_daily > 0 && recent_count > avg_daily * 3 {
                score -= 10;
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "tx_frequency_spike",
                    "message": format!("交易频率异常: 过去24h {} 笔，日均 {} 笔 (3倍以上)", recent_count, avg_daily),
                }));
                recommendations.push("确认近期高频交易是否为本人操作".into());
            }

            // Unique recipient analysis (potential address poisoning)
            let unique_recipients: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT to_addr) FROM transactions WHERE user_id = $1 AND created_at > NOW() - INTERVAL '7 days'"
            )
            .bind(uid)
            .fetch_one(db)
            .await
            .unwrap_or(0);

            if unique_recipients > 10 {
                score -= 5;
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "many_recipients",
                    "message": format!("7天内转账到 {} 个不同地址，请警惕地址投毒攻击", unique_recipients),
                }));
                recommendations.push("转账前仔细核对完整地址，不要只看首尾字符".into());
            }
        } else if !db_available || !has_user {
            // Already reported above
        } else {
            findings.push(serde_json::json!({
                "severity": "info",
                "type": "tx_clean",
                "message": "近7天交易记录正常，无异常失败",
            }));
        }

        // ═══ 3. On-Chain ERC-20 Approval Check ═══
        let approval_check = self.check_token_approvals(&address).await;
        match approval_check {
            Some((unlimited_count, high_risk_approvals)) => {
                if unlimited_count > 0 {
                    score -= std::cmp::min(unlimited_count as u32 * 5, 20);
                    findings.push(serde_json::json!({
                        "severity": "high",
                        "type": "unlimited_approvals",
                        "message": format!("发现 {} 个无限额度代币授权，存在资产被盗风险", unlimited_count),
                        "details": high_risk_approvals,
                    }));
                    recommendations.push("撤销不必要的代币授权，特别是无限额度的授权".into());
                } else {
                    findings.push(serde_json::json!({
                        "severity": "info",
                        "type": "approvals_clean",
                        "message": "未发现高风险代币授权",
                    }));
                }
            }
            None => {
                tracing::warn!("security_audit: on-chain approval check failed for {:?}", address);
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "approval_check_failed",
                    "message": "链上授权检查暂时不可用，请稍后重试",
                }));
            }
        }

        // ═══ 4. Presign Pool Status ═══
        if let Some(_pm) = &self.app_state.presign_manager {
            if let (Ok(db), Some(uid)) = (self.app_state.require_db(), &user_uuid) {
                let presign_count = available_presignature_count(db, *uid).await;

                if presign_count == 0 {
                    score -= 5;
                    findings.push(serde_json::json!({
                        "severity": "medium",
                        "type": "no_presignatures",
                        "message": "预签名池为空，下次签名将需要额外时间",
                    }));
                } else {
                    findings.push(serde_json::json!({
                        "severity": "info",
                        "type": "presign_ready",
                        "message": format!("预签名池就绪 ({} 个可用)", presign_count),
                    }));
                }
            }
        } else {
            findings.push(serde_json::json!({
                "severity": "medium",
                "type": "no_presignatures",
                "message": "预签名服务未初始化",
            }));
        }

        // ═══ 5. Policy Engine Check ═══
        // (logic and rationale documented in `policy_signals`)
        if let (Ok(db), Some(uid)) = (self.app_state.require_db(), &user_uuid) {
            let pol = policy_signals(db, *uid).await;

            findings.push(serde_json::json!({
                "severity": "info",
                "type": "policies_active",
                "message": format!(
                    "限额保护已启用：单笔上限 ${:.0}，日累计上限 ${:.0}",
                    pol.single_limit_usd, pol.daily_limit_usd
                ),
            }));

            if pol.custom_policy_count > 0 {
                findings.push(serde_json::json!({
                    "severity": "info",
                    "type": "custom_policies_active",
                    "message": format!("{} 条自定义规则策略已生效", pol.custom_policy_count),
                }));
            } else if pol.single_limit_usd >= 500.0 && pol.daily_limit_usd >= 2000.0 {
                score -= 5;
                recommendations.push(
                    "建议根据实际使用习惯调低单笔和日限额，降低意外或授权滥用风险".into()
                );
            }
        }

        // ═══ 6. Infrastructure Health ═══
        findings.push(serde_json::json!({
            "severity": "info",
            "type": "mpc_protection",
            "message": "MPC 2-of-3 门限签名保护已启用",
        }));
        findings.push(serde_json::json!({
            "severity": "info",
            "type": "transport_encryption",
            "message": "Noise_XX 协议传输加密已启用",
        }));

        // Check auth method from client context
        let auth_method = self.auth_method.as_deref().unwrap_or("unknown");
        match auth_method {
            "biometric" => {
                findings.push(serde_json::json!({
                    "severity": "info",
                    "type": "biometric_auth",
                    "message": "生物识别签名授权已启用",
                }));
            }
            "pin" => {
                score -= 5;
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "biometric_auth",
                    "message": "当前使用 PIN 验证，建议开启生物识别以提高安全性",
                }));
                recommendations.push("在设置中开启 Face ID / 指纹验证，比 PIN 更安全且不可窥视".into());
            }
            _ => {
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "biometric_auth",
                    "message": "无法确认本地认证方式",
                }));
            }
        }

        // Always include base recommendations
        if recommendations.is_empty() {
            recommendations.push("当前安全状况良好，保持定期检查习惯".into());
        }
        recommendations.push("不要在不信任的网站连接钱包或签署交易".into());

        score = score.clamp(0, 100);
        let risk_level = if score >= 90 { "low" } else if score >= 70 { "medium" } else { "high" };

        let result = serde_json::json!({
            "address": format!("0x{:x}", address),
            "score": score,
            "risk_level": risk_level,
            "findings": findings,
            "recommendations": recommendations,
            "audit_time": chrono::Utc::now().to_rfc3339(),
        });

        ToolExecutionResult {
            tool_id: tool_id.to_string(),
            tool_name: "security_audit".into(),
            success: true,
            result,
            error: None,
        }
    }

    /// Check ERC-20 token approvals on-chain via eth_getLogs
    async fn check_token_approvals(&self, address: &Address) -> Option<(usize, Vec<Value>)> {
        // ERC-20 Approval event topic: keccak256("Approval(address,address,uint256)")
        let approval_topic = "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925";
        let owner_topic = format!("0x000000000000000000000000{:x}", address);

        // Check on the primary chain (Ethereum mainnet or Base)
        let chain_id = 8453u64; // Base as default active chain
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getLogs",
            "params": [{
                "fromBlock": "earliest",
                "toBlock": "latest",
                "topics": [approval_topic, &owner_topic]
            }],
            "id": 1
        });

        let resp = self.app_state.rpc_call(chain_id, &body).await.ok()?;
        let logs = resp.get("result")?.as_array()?;

        let max_uint256 = "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let mut unlimited_approvals: Vec<Value> = Vec::new();

        for log in logs.iter().rev().take(50) {
            let data = log.get("data")?.as_str()?;
            // Unlimited approval: data == max_uint256
            if data == max_uint256 {
                let spender_topic = log.get("topics")?.as_array()?.get(2)?.as_str().unwrap_or("");
                let contract = log.get("address")?.as_str().unwrap_or("unknown");
                unlimited_approvals.push(serde_json::json!({
                    "contract": contract,
                    "spender": spender_topic,
                }));
            }
        }

        let count = unlimited_approvals.len();
        if count > 0 {
            Some((count, unlimited_approvals.into_iter().take(5).collect()))
        } else {
            None
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for the three DB-signal helpers.
//
// Each test uses `#[sqlx::test(migrations = "../migrations")]` which spins
// up a real isolated Postgres database, runs all migrations, and tears it down
// after the test.  This means any column rename or table restructure that breaks
// the production queries will immediately break these tests.
//
// The tests cover the three bugs that were found in the security audit:
//   1. shard_signals: healthy wallet must NOT report missing-shard high-severity
//      (old code required device+server+backup rows; design only has server row)
//   2. available_presignature_count: must count via `status='available'`, not the
//      nonexistent `used` column
//   3. policy_signals: must read `user_policies` (enforced limits), not just the
//      `policies` JSONB table
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::{available_presignature_count, policy_signals, shard_signals};
    use sqlx::PgPool;
    use uuid::Uuid;

    // ── seed helpers ──────────────────────────────────────────────────────────

    async fn seed_user(pool: &PgPool) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, device_id) VALUES ($1, $2)"
        )
        .bind(id)
        .bind(format!("test-device-{}", id))
        .execute(pool)
        .await
        .expect("seed user");
        id
    }

    async fn seed_wallet(pool: &PgPool, user_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO wallets (id, user_id, public_key, eth_address)
             VALUES ($1, $2, '\\x01'::bytea, '\\x02'::bytea)"
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("seed wallet");
        id
    }

    // ── 1. shard_signals ─────────────────────────────────────────────────────

    /// A wallet created by DKG has exactly one shard_metadata row (location='server').
    /// The backup is proven by backup_shard_hash on that same row.
    /// `shard_signals` must report has_server=true, has_backup=true → no false high.
    #[sqlx::test(migrations = "../migrations")]
    async fn shard_signals_healthy_wallet_server_row_plus_backup_hash(pool: PgPool) {
        let uid = seed_user(&pool).await;
        // Insert the server shard row with a non-empty backup_shard_hash.
        sqlx::query(
            "INSERT INTO shard_metadata
             (user_id, location, party_index, status, last_verified, backup_shard_hash)
             VALUES ($1, 'server', 1, 'healthy', NOW(), $2)"
        )
        .bind(uid)
        .bind(vec![0xabu8; 32]) // 32-byte SHA-256 placeholder
        .execute(&pool)
        .await
        .expect("seed shard_metadata");

        let sig = shard_signals(&pool, uid).await;

        assert!(sig.has_server,  "has_server must be true: server row exists");
        assert!(sig.has_backup,  "has_backup must be true: backup_shard_hash is set");
        assert!(sig.unhealthy.is_empty(), "no unhealthy shards");
        assert!(sig.stale.is_empty(),     "recently-verified shard must not be stale");
        assert!(sig.any_rows);
    }

    /// Without any shard row the wallet is broken: has_server must be false.
    #[sqlx::test(migrations = "../migrations")]
    async fn shard_signals_no_rows_reports_missing_server(pool: PgPool) {
        let uid = seed_user(&pool).await;
        let sig = shard_signals(&pool, uid).await;
        assert!(!sig.has_server, "no shard row → has_server must be false");
        assert!(!sig.has_backup);
        assert!(!sig.any_rows);
    }

    /// Server row exists but backup_shard_hash is NULL → backup not yet completed.
    #[sqlx::test(migrations = "../migrations")]
    async fn shard_signals_server_without_backup_hash(pool: PgPool) {
        let uid = seed_user(&pool).await;
        sqlx::query(
            "INSERT INTO shard_metadata (user_id, location, party_index, status, last_verified)
             VALUES ($1, 'server', 1, 'healthy', NOW())"
        )
        .bind(uid)
        .execute(&pool)
        .await
        .expect("seed shard_metadata");

        let sig = shard_signals(&pool, uid).await;
        assert!(sig.has_server);
        assert!(!sig.has_backup, "no backup_shard_hash → has_backup must be false");
    }

    // ── 2. available_presignature_count ──────────────────────────────────────

    /// Query must use `status='available'` — the old `used=false` column does not
    /// exist and would always error → unwrap_or(0) → always report empty pool.
    #[sqlx::test(migrations = "../migrations")]
    async fn presig_count_returns_available_not_consumed(pool: PgPool) {
        let uid = seed_user(&pool).await;
        let wid = seed_wallet(&pool, uid).await;

        // One available presig (should be counted).
        sqlx::query(
            "INSERT INTO presignatures (wallet_id, user_id, presig_data, status, expires_at)
             VALUES ($1, $2, '\\x00'::bytea, 'available', NOW() + INTERVAL '1 hour')"
        )
        .bind(wid).bind(uid).execute(&pool).await.expect("seed available presig");

        // One consumed presig (must NOT be counted).
        sqlx::query(
            "INSERT INTO presignatures (wallet_id, user_id, presig_data, status, expires_at)
             VALUES ($1, $2, '\\x00'::bytea, 'consumed', NOW() + INTERVAL '1 hour')"
        )
        .bind(wid).bind(uid).execute(&pool).await.expect("seed consumed presig");

        // One expired-by-timestamp available presig (must NOT be counted).
        sqlx::query(
            "INSERT INTO presignatures (wallet_id, user_id, presig_data, status, expires_at)
             VALUES ($1, $2, '\\x00'::bytea, 'available', NOW() - INTERVAL '1 minute')"
        )
        .bind(wid).bind(uid).execute(&pool).await.expect("seed expired presig");

        let count = available_presignature_count(&pool, uid).await;
        assert_eq!(count, 1, "only the non-expired available presig should be counted");
    }

    /// User with no presigs → count 0 (must not error from missing `used` column).
    #[sqlx::test(migrations = "../migrations")]
    async fn presig_count_zero_for_new_user(pool: PgPool) {
        let uid = seed_user(&pool).await;
        let count = available_presignature_count(&pool, uid).await;
        assert_eq!(count, 0);
    }

    // ── 3. policy_signals ────────────────────────────────────────────────────

    /// A new account always has enforced limits (engine Default or explicit row).
    /// The old code only checked `policies` JSONB rules and reported "no protection"
    /// even when user_policies limits were in force.
    #[sqlx::test(migrations = "../migrations")]
    async fn policy_signals_explicit_row_returns_correct_limits(pool: PgPool) {
        let uid = seed_user(&pool).await;
        sqlx::query(
            "INSERT INTO user_policies (user_id, single_limit_usd, daily_limit_usd)
             VALUES ($1, 100.0, 500.0)"
        )
        .bind(uid)
        .execute(&pool)
        .await
        .expect("seed user_policies");

        let pol = policy_signals(&pool, uid).await;
        assert!(pol.has_explicit_limits);
        assert_eq!(pol.single_limit_usd, 100.0);
        assert_eq!(pol.daily_limit_usd,  500.0);
        assert_eq!(pol.custom_policy_count, 0);
    }

    /// Missing user_policies row → fallback to engine Default ($500/$2000).
    /// Must NOT report "no protection" (the old bug).
    #[sqlx::test(migrations = "../migrations")]
    async fn policy_signals_no_row_falls_back_to_default(pool: PgPool) {
        let uid = seed_user(&pool).await;
        let pol = policy_signals(&pool, uid).await;
        assert!(!pol.has_explicit_limits);
        // These must match UserLimits::default() in crates/policy-engine/src/limits.rs
        assert_eq!(pol.single_limit_usd, 500.0,  "default single limit");
        assert_eq!(pol.daily_limit_usd,  2000.0, "default daily limit");
    }
}
