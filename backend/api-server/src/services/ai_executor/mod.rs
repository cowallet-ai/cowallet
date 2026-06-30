//! AI Tool Execution Engine
//! Handles execution of wallet tools requested by Claude AI

mod balance;
mod chains;
mod history;
mod policy;
mod security;
mod swap;
mod transaction;
mod yield_tools;

use crate::state::AppState;
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

/// Execution context for tool calls
#[derive(Clone)]
pub struct ToolContext {
    pub app_state: AppState,
    pub user_id: Option<String>,
    pub wallet_address: Option<String>,
    pub auth_method: Option<String>,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize)]
pub struct ToolExecutionResult {
    pub tool_id: String,
    pub tool_name: String,
    pub success: bool,
    pub result: Value,
    pub error: Option<String>,
}

/// Helper: Parse a parameter from JSON Value
fn parse_param<T: for<'a> Deserialize<'a>>(params: &Value, key: &str) -> Option<T> {
    params
        .get(key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// Parse wallet address from context. Returns None if not provided or invalid.
fn parse_wallet_address(wallet_address: Option<&str>) -> Option<Address> {
    wallet_address.and_then(|addr| Address::from_str(addr).ok())
}

/// Helper: Get USDC address for common chains
fn usdc_address_for_chain(chain_id: u64) -> Option<Address> {
    match chain_id {
        1 => Some(
            Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap(),
        ),
        8453 => Some(
            Address::from_str("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913").unwrap(),
        ),
        _ => None,
    }
}

/// Infer chain ID from token symbol when not explicitly provided.
fn infer_chain_id_from_token(token: &str) -> Option<u64> {
    match token.to_uppercase().as_str() {
        "POL" | "MATIC" => Some(137),
        "BNB" => Some(56),
        "ETH" => Some(1),
        _ => None,
    }
}

/// Format U256 value with given decimals (simplified version)
fn format_units(value: alloy_primitives::U256, decimals: u32) -> String {
    let divisor = alloy_primitives::U256::from(10).pow(alloy_primitives::U256::from(decimals));
    let integer = value / divisor;
    let fraction = value % divisor;
    if fraction.is_zero() {
        format!("{}", integer)
    } else {
        format!("{}.{:06}", integer, fraction.to_string().chars().take(6).collect::<String>())
    }
}

fn token_balance_to_json(b: &crate::services::covalent::TokenBalance) -> serde_json::Value {
    serde_json::json!({
        "symbol": b.symbol,
        "name": b.name,
        "balance": b.balance_formatted,
        "balance_raw": b.balance,
        "usd": b.usd,
        "usd_24h": b.usd_24h,
        "quote_rate": b.quote_rate,
        "quote_rate_24h": b.quote_rate_24h,
        "native": b.native_token,
        "contract_address": b.contract_address,
        "decimals": b.decimals,
        "logo_url": b.logo_url,
        "chain_id": b.chain_id,
        "chain_name": b.chain_name,
        "last_transferred_at": b.last_transferred_at,
    })
}

impl ToolContext {
    /// Execute a tool by name with parameters
    pub async fn execute_tool(&self, tool_name: &str, tool_id: &str, params: Value) -> ToolExecutionResult {
        tracing::debug!("Executing tool: {} with params: {:?}", tool_name, params);

        let result = match tool_name {
            "get_balance" => self.execute_get_balance(tool_id, params).await,
            "get_supported_chains" => self.execute_get_supported_chains(tool_id).await,
            "get_token_info" => self.execute_get_token_info(tool_id, params).await,
            "send_transaction" => self.execute_send_transaction(tool_id, params).await,
            "get_transaction_history" => self.execute_get_transaction_history(tool_id, params).await,
            "get_wallet_address" => self.execute_get_wallet_address(tool_id).await,
            "security_audit" => self.execute_security_audit(tool_id).await,
            "swap_token" => self.execute_swap_token(tool_id, params).await,
            "search_yield_opportunities" => self.execute_search_yield(tool_id, params).await,
            "list_yield_protocols" => self.execute_list_protocols(tool_id, params).await,
            _ => ToolExecutionResult {
                tool_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                success: false,
                result: Value::Null,
                error: Some(format!("Unknown tool: {}", tool_name)),
            },
        };

        tracing::debug!(
            "Tool {} result: success={}, error={:?}",
            tool_name,
            result.success,
            result.error
        );
        result
    }
}
