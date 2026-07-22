//! Chain enumeration and wallet address tools.

use super::{parse_wallet_address, ToolContext, ToolExecutionResult};
use serde_json::Value;

impl ToolContext {
    // --- get_supported_chains ---
    pub(super) async fn execute_get_supported_chains(&self, tool_id: &str) -> ToolExecutionResult {
        let chains = vec![
            serde_json::json!({
                "chain_id": 1,
                "name": "Ethereum",
                "symbol": "ETH",
                "type": "mainnet"
            }),
            serde_json::json!({
                "chain_id": 8453,
                "name": "Base",
                "symbol": "ETH",
                "type": "mainnet"
            }),
            serde_json::json!({
                "chain_id": 42161,
                "name": "Arbitrum One",
                "symbol": "ETH",
                "type": "mainnet"
            }),
            serde_json::json!({
                "chain_id": 10,
                "name": "Optimism",
                "symbol": "ETH",
                "type": "mainnet"
            }),
            serde_json::json!({
                "chain_id": 56,
                "name": "BNB Chain",
                "symbol": "BNB",
                "type": "mainnet"
            }),
            serde_json::json!({
                "chain_id": 137,
                "name": "Polygon",
                "symbol": "POL",
                "type": "mainnet"
            }),
        ];

        let result = serde_json::json!({
            "chains": chains,
            "total_count": chains.len(),
        });

        ToolExecutionResult {
            tool_id: tool_id.to_string(),
            tool_name: "get_supported_chains".into(),
            success: true,
            result,
            error: None,
        }
    }

    // --- get_wallet_address ---
    pub(super) async fn execute_get_wallet_address(&self, tool_id: &str) -> ToolExecutionResult {
        let address = match parse_wallet_address(self.wallet_address.as_deref()) {
            Some(a) => a,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "get_wallet_address".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("钱包地址未提供".into()),
                }
            }
        };
        let result = serde_json::json!({
            "address": format!("0x{:x}", address),
            "supported_chains": [
                {"chain_id": 1, "name": "Ethereum"},
                {"chain_id": 8453, "name": "Base"},
                {"chain_id": 42161, "name": "Arbitrum One"},
                {"chain_id": 10, "name": "Optimism"},
                {"chain_id": 56, "name": "BNB Chain"},
                {"chain_id": 137, "name": "Polygon"},
            ],
            "note": "同一地址适用于所有 EVM 链",
        });

        ToolExecutionResult {
            tool_id: tool_id.to_string(),
            tool_name: "get_wallet_address".into(),
            success: true,
            result,
            error: None,
        }
    }
}
