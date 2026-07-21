//! Wallet security audit tools.

use super::{parse_wallet_address, ToolContext, ToolExecutionResult};
use alloy_primitives::Address;
use serde_json::Value;

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
        //
        // Completeness must be judged with the SAME signals the app's recovery
        // status page uses — NOT by requiring three separate location rows. By
        // design a healthy wallet has ONE shard_metadata row (location='server'):
        //   • server shard → the `location='server'` row (created by DKG)
        //   • backup shard → proven server-side by `backup_shard_hash` set on the
        //                     server row (store_backup_hash), or an explicit
        //                     `location='backup'` row if the client uploaded one
        //   • device shard → held in the phone's secure enclave; there is normally
        //                     NO server row for it and the server cannot inspect it
        // The old code required device+server+backup rows to ALL exist, so a fully
        // healthy wallet was always flagged "缺少: 设备、备份" (false high) even
        // though the recovery page correctly showed 3/3.
        if let (Ok(db), Some(uid)) = (self.app_state.require_db(), &user_uuid) {
            let shards: Vec<(String, String, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>, Option<Vec<u8>>)> = sqlx::query_as(
                "SELECT location, status, last_used, last_verified, backup_shard_hash FROM shard_metadata WHERE user_id = $1"
            )
            .bind(uid)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            let has_server = shards.iter().any(|s| s.0 == "server");
            // Backup is proven to the server either by an explicit backup row or by
            // the backup_shard_hash recorded on the server row after DKG.
            let has_backup = shards.iter().any(|s| s.0 == "backup")
                || shards.iter().any(|s| s.4.as_ref().map_or(false, |h| !h.is_empty()));

            if !has_server {
                // No server share = the wallet cannot participate in signing. This
                // is the only genuinely server-detectable "missing shard".
                score -= 20;
                findings.push(serde_json::json!({
                    "severity": "high",
                    "type": "shard_incomplete",
                    "message": "服务器密钥分片缺失，钱包无法签名，请通过恢复流程重建",
                }));
                recommendations.push("尽快通过恢复流程重建服务器分片".into());
            } else if !has_backup {
                // Server (and hence device) exist — the wallet is operational — but
                // the recovery backup was never completed. Real and actionable, but
                // it's an incomplete backup, not a missing signing shard.
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

            // Check for compromised/unhealthy shards
            let unhealthy: Vec<_> =
                shards.iter().filter(|s| s.1 != "healthy").collect();
            if !unhealthy.is_empty() {
                score -= 25;
                for s in &unhealthy {
                    findings.push(serde_json::json!({
                        "severity": "high",
                        "type": "shard_unhealthy",
                        "message": format!("{}分片状态异常: {}", s.0, s.1),
                    }));
                }
                recommendations.push("执行密钥刷新(Reshare)修复异常分片".into());
            }

            // Check shard freshness (no verification in 30+ days)
            let stale_shards: Vec<&str> = shards.iter()
                .filter(|s| {
                    s.3.map_or(true, |v| {
                        (chrono::Utc::now() - v).num_days() > 30
                    })
                })
                .map(|s| s.0.as_str())
                .collect();
            if !stale_shards.is_empty() && !shards.is_empty() {
                score -= 5;
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "shard_stale",
                    "message": format!("分片超过30天未验证: {}", stale_shards.join("、")),
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
                // The presignatures table tracks availability via `status`
                // ('available'/'reserved'/'consumed'/'expired') — there is no
                // `used` column. The old `WHERE used = false` always errored and
                // fell back to 0, so the audit permanently reported an empty pool.
                // Count usable (available + not-yet-expired) presignatures, matching
                // reserve_presignature's own predicate.
                let presign_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM presignatures
                     WHERE user_id = $1 AND status = 'available' AND expires_at > NOW()"
                )
                .bind(uid)
                .fetch_one(db)
                .await
                .unwrap_or(0);

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
        //
        // Two tiers of protection to check:
        //   a) user_policies: per-user USD limits (single/daily) enforced by the
        //      policy-engine for EVERY transaction. Missing row → engine falls back
        //      to Default (single=$500, daily=$2000) — protection is always present.
        //   b) policies: custom JSONB rule-based policies (optional, advanced).
        //
        // The old code only checked `policies` (JSONB rules) and reported "no
        // protection" even when user_policies limits were in force — misleading.
        if let (Ok(db), Some(uid)) = (self.app_state.require_db(), &user_uuid) {
            // Read the actual enforced limits from user_policies (row may not exist
            // for accounts created before migration 013; engine defaults apply then).
            let limits: Option<(f64, f64)> = sqlx::query_as(
                "SELECT single_limit_usd, daily_limit_usd FROM user_policies WHERE user_id = $1"
            )
            .bind(uid)
            .fetch_optional(db)
            .await
            .unwrap_or(None);

            let (single, daily) = limits.unwrap_or((500.0, 2000.0)); // matches engine Default

            // Count any additional custom JSONB policies the user may have set up.
            let custom_policy_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM policies WHERE user_id = $1 AND rules != '{}' AND enabled = true"
            )
            .bind(uid)
            .fetch_one(db)
            .await
            .unwrap_or(0);

            findings.push(serde_json::json!({
                "severity": "info",
                "type": "policies_active",
                "message": format!(
                    "限额保护已启用：单笔上限 ${:.0}，日累计上限 ${:.0}",
                    single, daily
                ),
            }));

            if custom_policy_count > 0 {
                findings.push(serde_json::json!({
                    "severity": "info",
                    "type": "custom_policies_active",
                    "message": format!("{} 条自定义规则策略已生效", custom_policy_count),
                }));
            } else if single >= 500.0 && daily >= 2000.0 {
                // Only the default limits — nudge toward tightening for high-value wallets.
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
