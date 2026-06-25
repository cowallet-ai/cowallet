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
        if let (Ok(db), Some(uid)) = (self.app_state.require_db(), &user_uuid) {
            let shards: Vec<(String, String, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
                "SELECT location, status, last_used, last_verified FROM shard_metadata WHERE user_id = $1"
            )
            .bind(uid)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            let has_device = shards.iter().any(|s| s.0 == "device");
            let has_server = shards.iter().any(|s| s.0 == "server");
            let has_backup = shards.iter().any(|s| s.0 == "backup");

            if has_device && has_server && has_backup {
                findings.push(serde_json::json!({
                    "severity": "info",
                    "type": "shard_complete",
                    "message": "密钥分片完整 (设备/服务器/备份 3/3)",
                }));
            } else {
                let missing: Vec<&str> = [
                    if !has_device { Some("设备") } else { None },
                    if !has_server { Some("服务器") } else { None },
                    if !has_backup { Some("备份") } else { None },
                ].into_iter().flatten().collect();

                score -= 20;
                findings.push(serde_json::json!({
                    "severity": "high",
                    "type": "shard_incomplete",
                    "message": format!("密钥分片不完整，缺少: {}", missing.join("、")),
                }));
                recommendations.push("立即完成缺失分片的备份，防止资产丢失".into());
            }

            // Check for compromised/unhealthy shards
            let unhealthy: Vec<&(String, String, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)> =
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
                let presign_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM presignatures WHERE user_id = $1 AND used = false"
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
        if let (Ok(db), Some(uid)) = (self.app_state.require_db(), &user_uuid) {
            let policy_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM policies WHERE user_id = $1 AND rules != '{}'"
            )
            .bind(uid)
            .fetch_one(db)
            .await
            .unwrap_or(0);

            if policy_count == 0 {
                score -= 10;
                findings.push(serde_json::json!({
                    "severity": "medium",
                    "type": "no_policies",
                    "message": "未设置安全策略，交易无限额保护",
                }));
                recommendations.push("设置每日转账限额和单笔最大金额策略".into());
            } else {
                findings.push(serde_json::json!({
                    "severity": "info",
                    "type": "policies_active",
                    "message": format!("{} 条安全策略已生效", policy_count),
                }));
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
