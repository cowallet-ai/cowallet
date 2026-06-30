//! Transaction history tool.

use super::{parse_param, ToolContext, ToolExecutionResult};
use serde_json::Value;
use sqlx::Row;

impl ToolContext {
    // --- get_transaction_history ---
    pub(super) async fn execute_get_transaction_history(&self, tool_id: &str, params: Value) -> ToolExecutionResult {
        let limit: i64 = parse_param(&params, "limit").unwrap_or(20).min(100);
        let offset: i64 = parse_param(&params, "offset").unwrap_or(0);
        let chain_id_filter: Option<u64> = parse_param(&params, "chain_id");

        // Try Covalent API first if no chain_id filter and wallet address available
        if chain_id_filter.is_none() {
            if let (Some(api_key), Some(addr)) = (&self.app_state.covalent_api_key, &self.wallet_address) {
                let supported_chains = vec![1u64, 8453, 42161, 10, 56, 137];
                match crate::services::covalent::get_all_chain_transactions(
                    &self.app_state.http,
                    api_key,
                    addr,
                    &supported_chains,
                )
                .await
                {
                    Ok(txs) => {
                        let transactions: Vec<serde_json::Value> = txs
                            .into_iter()
                            .take(limit as usize)
                            .map(|tx| {
                                let token = &tx.token_symbol;
                                let decimals: u32 = if token == "USDC" || token == "USDT" { 6 } else { 18 };
                                let formatted_value = crate::services::covalent::format_value(&tx.value, decimals);
                                serde_json::json!({
                                    "chain_id": tx.chain_id,
                                    "chain_name": tx.chain_name,
                                    "tx_hash": tx.tx_hash,
                                    "from_addr": tx.from,
                                    "to_addr": tx.to,
                                    "value": formatted_value,
                                    "value_raw": tx.value,
                                    "token": tx.token_symbol,
                                    "timestamp": tx.timestamp,
                                    "status": tx.status,
                                })
                            })
                            .collect();

                        let result = serde_json::json!({
                            "transactions": transactions,
                            "multi_chain": true,
                            "limit": limit,
                            "total": transactions.len()
                        });

                        return ToolExecutionResult {
                            tool_id: tool_id.to_string(),
                            tool_name: "get_transaction_history".into(),
                            success: true,
                            result,
                            error: None,
                        };
                    }
                    Err(e) => {
                        tracing::warn!("Covalent multi-chain tx query failed, falling back to DB: {}", e);
                    }
                }
            }
        }

        // Fallback to database query
        let db = match self.app_state.require_db() {
            Ok(db) => db,
            Err(_) => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "get_transaction_history".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("数据库不可用".into()),
                };
            }
        };

        let user_id = match &self.user_id {
            Some(uid) => match uuid::Uuid::parse_str(uid) {
                Ok(id) => id,
                Err(_) => {
                    return ToolExecutionResult {
                        tool_id: tool_id.to_string(),
                        tool_name: "get_transaction_history".into(),
                        success: false,
                        result: Value::Null,
                        error: Some("Invalid user ID format".into()),
                    };
                }
            },
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "get_transaction_history".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("User not authenticated".into()),
                };
            }
        };

        // Query database with optional chain_id filter
        let rows = if let Some(chain_id) = chain_id_filter {
            sqlx::query(
                "SELECT id, chain_id, to_addr, value, token, tx_hash, status, created_at
                 FROM transactions WHERE user_id = $1 AND chain_id = $2
                 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
            )
            .bind(user_id)
            .bind(chain_id as i64)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await
        } else {
            sqlx::query(
                "SELECT id, chain_id, to_addr, value, token, tx_hash, status, created_at
                 FROM transactions WHERE user_id = $1
                 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await
        };

        let rows = match rows {
            Ok(r) => r,
            Err(e) => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "get_transaction_history".into(),
                    success: false,
                    result: Value::Null,
                    error: Some(format!("Database query failed: {}", e)),
                };
            }
        };

        let transactions: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                let chain_id = row.get::<i64, _>("chain_id") as u64;
                let chain_name = crate::services::covalent::chain_display_name(chain_id);
                serde_json::json!({
                    "id": row.get::<uuid::Uuid, _>("id").to_string(),
                    "chain_id": chain_id,
                    "chain_name": chain_name,
                    "to_addr": format!("0x{}", hex::encode(row.get::<Vec<u8>, _>("to_addr"))),
                    "value": row.get::<String, _>("value"),
                    "token": row.get::<Option<String>, _>("token"),
                    "tx_hash": row.get::<Option<Vec<u8>>, _>("tx_hash").map(|h| format!("0x{}", hex::encode(&h))),
                    "status": row.get::<String, _>("status"),
                    "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339()
                })
            })
            .collect();

        let total = transactions.len();
        let result = serde_json::json!({
            "transactions": transactions,
            "limit": limit,
            "offset": offset,
            "total": total
        });

        ToolExecutionResult {
            tool_id: tool_id.to_string(),
            tool_name: "get_transaction_history".into(),
            success: true,
            result,
            error: None,
        }
    }
}
