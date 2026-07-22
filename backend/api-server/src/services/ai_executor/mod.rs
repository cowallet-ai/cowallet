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
    /// The user's original typed message (free of injected untrusted context).
    /// Used to cross-validate the LLM-chosen recipient address against any
    /// address the user literally typed, as a defense against indirect prompt
    /// injection (F-013).
    pub user_message: Option<String>,
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
        1 => Some(Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap()),
        8453 => Some(Address::from_str("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913").unwrap()),
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
        format!(
            "{}.{:06}",
            integer,
            fraction.to_string().chars().take(6).collect::<String>()
        )
    }
}

/// Convert a human-readable decimal amount string (e.g. "1.5") to the token's
/// smallest unit using exact integer math (F-015). Avoids f64, which silently
/// loses precision for values the user explicitly approved. Rejects malformed
/// input and amounts with more fractional digits than the token supports rather
/// than truncating them.
fn parse_decimal_to_smallest(value: &str, decimals: u32) -> Result<alloy_primitives::U256, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("empty amount".into());
    }
    let (int_part, frac_part) = match value.split_once('.') {
        Some((i, f)) => (i, f),
        None => (value, ""),
    };
    // Both sides must be pure decimal digits (no sign, no exponent, no hex).
    let valid = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if (!int_part.is_empty() && !valid(int_part)) || !valid(frac_part) {
        return Err("invalid decimal amount".into());
    }
    if frac_part.len() > decimals as usize {
        return Err(format!(
            "amount has more fractional digits ({}) than token decimals ({})",
            frac_part.len(),
            decimals
        ));
    }
    // Right-pad the fraction to `decimals` digits, then parse the concatenation
    // as a single integer in the smallest unit.
    let mut digits = String::new();
    digits.push_str(if int_part.is_empty() { "0" } else { int_part });
    digits.push_str(frac_part);
    for _ in 0..(decimals as usize - frac_part.len()) {
        digits.push('0');
    }
    alloy_primitives::U256::from_str_radix(&digits, 10)
        .map_err(|_| "amount out of range".to_string())
}

/// Validate an EVM address: 0x-prefixed, 20 bytes, valid hex, and — when the
/// input is mixed-case — a correct EIP-55 checksum (F-016). Returns the
/// canonical checksummed form.
fn validate_evm_address(addr: &str) -> Result<String, String> {
    if !addr.starts_with("0x") || addr.len() != 42 {
        return Err("expected 0x-prefixed 40-char hex address".into());
    }
    let body = &addr[2..];
    if !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("address contains non-hex characters".into());
    }
    // Address::from_str enforces the EIP-55 checksum when the string is mixed
    // case; all-lower / all-upper inputs are accepted and normalized.
    let parsed = if body.chars().any(|c| c.is_ascii_uppercase())
        && body.chars().any(|c| c.is_ascii_lowercase())
    {
        Address::from_str(addr).map_err(|_| "invalid EIP-55 checksum".to_string())?
    } else {
        Address::from_str(&addr.to_lowercase()).map_err(|_| "invalid address".to_string())?
    };
    Ok(parsed.to_checksum(None))
}

/// Extract all 0x-prefixed 40-hex-char EVM addresses literally present in a
/// string. Used to cross-validate LLM-chosen recipients against addresses the
/// user actually typed (F-013).
fn extract_0x_addresses(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 42 <= bytes.len() {
        if bytes[i] == b'0' && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
            // Guard against slicing across a multi-byte UTF-8 boundary: `i` and
            // `i+2` are ASCII ('0','x') so always valid, but `i+42` can land in
            // the middle of a non-ASCII char (e.g. "0x"+39 hex+"é"), which would
            // panic. `text` is the raw user message on the send_transaction path.
            if text.is_char_boundary(i + 42) {
                let candidate = &text[i..i + 42];
                if candidate[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                    // Ensure the char after the 42-char run isn't another hex digit
                    // (which would make this a longer, non-address hex string).
                    let next_is_hex = bytes
                        .get(i + 42)
                        .map(|b| (*b as char).is_ascii_hexdigit())
                        .unwrap_or(false);
                    if !next_is_hex {
                        out.push(candidate.to_string());
                        i += 42;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

fn token_balance_to_json(b: &crate::services::okx::TokenBalance) -> serde_json::Value {
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
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        tool_id: &str,
        params: Value,
    ) -> ToolExecutionResult {
        tracing::debug!("Executing tool: {} with params: {:?}", tool_name, params);

        let result = match tool_name {
            "get_balance" => self.execute_get_balance(tool_id, params).await,
            "get_supported_chains" => self.execute_get_supported_chains(tool_id).await,
            "get_token_info" => self.execute_get_token_info(tool_id, params).await,
            "send_transaction" => self.execute_send_transaction(tool_id, params).await,
            "get_transaction_history" => {
                self.execute_get_transaction_history(tool_id, params).await
            }
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

#[cfg(test)]
mod tests {
    use super::extract_0x_addresses;

    #[test]
    fn extracts_plain_address() {
        let addr = "0x000102030405060708090a0b0c0d0e0f10111213";
        let got = extract_0x_addresses(&format!("send to {addr} please"));
        assert_eq!(got, vec![addr.to_string()]);
    }

    #[test]
    fn multibyte_after_prefix_does_not_panic() {
        // Regression: `0x` + 39 hex + a multi-byte char makes byte offset i+42
        // land mid-UTF-8-char. Must not panic on the funds-custody path.
        let text = format!("0x{}é rest", "0".repeat(39));
        let got = extract_0x_addresses(&text);
        assert!(
            got.is_empty(),
            "malformed candidate should be skipped, not panic"
        );
    }

    #[test]
    fn multibyte_before_valid_address_is_handled() {
        let addr = "0xabcdefABCDEF0123456789abcdef0123456789ab";
        let text = format!("café balance then {addr}");
        assert_eq!(extract_0x_addresses(&text), vec![addr.to_string()]);
    }

    #[test]
    fn rejects_longer_hex_run() {
        // 43 hex chars after 0x — not an address.
        let text = format!("0x{}", "a".repeat(43));
        assert!(extract_0x_addresses(&text).is_empty());
    }
}
