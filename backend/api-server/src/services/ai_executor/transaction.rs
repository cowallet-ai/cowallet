//! Transaction preparation and gas estimation tools.

use super::{
    extract_0x_addresses, format_units, infer_chain_id_from_token, parse_decimal_to_smallest,
    parse_param, parse_wallet_address, validate_evm_address, ToolContext, ToolExecutionResult,
};
use serde_json::Value;

/// Gas estimation result
struct GasEstimate {
    gas_units: u64,
    gas_price_gwei: Option<String>,
    cost_eth: Option<String>,
    cost_usd: Option<String>,
}

impl ToolContext {
    // --- send_transaction ---
    pub(super) async fn execute_send_transaction(
        &self,
        tool_id: &str,
        params: Value,
    ) -> ToolExecutionResult {
        // Important: We only PREPARE the transaction, do NOT actually send it
        // User biometric confirmation is required before signing

        let to_address: String = match parse_param(&params, "to_address") {
            Some(addr) => addr,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "send_transaction".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("Missing required parameter: to_address".into()),
                };
            }
        };

        let value: String = match parse_param(&params, "value") {
            Some(v) => v,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "send_transaction".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("Missing required parameter: value (in wei)".into()),
                };
            }
        };

        let token_str: String = parse_param(&params, "token").unwrap_or_else(|| "ETH".into());
        let contract_address: Option<String> = parse_param(&params, "contract_address");
        let decimals: u8 =
            parse_param::<u8>(&params, "decimals").unwrap_or_else(|| {
                match token_str.to_uppercase().as_str() {
                    "USDC" | "USDT" => 6,
                    _ => 18,
                }
            });
        let chain_id: u64 = match parse_param(&params, "chain_id")
            .or_else(|| infer_chain_id_from_token(&token_str))
        {
            Some(id) => id,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "send_transaction".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("Cannot determine target chain. Please ask the user which chain to use for this operation. Multi-chain tokens (USDC, USDT, DAI, WETH, LINK) require an explicit chain_id.".into()),
                };
            }
        };
        let send_all: bool = parse_param(&params, "send_all").unwrap_or(false);

        // Validate contract_address (hex + EIP-55 checksum, F-016) and normalize.
        let contract_address = match contract_address {
            Some(ca) => match validate_evm_address(&ca) {
                Ok(canonical) => Some(canonical),
                Err(e) => {
                    return ToolExecutionResult {
                        tool_id: tool_id.to_string(),
                        tool_name: "send_transaction".into(),
                        success: false,
                        result: Value::Null,
                        error: Some(format!("Invalid contract_address: {}", e)),
                    };
                }
            },
            None => None,
        };
        let from_address = match parse_wallet_address(self.wallet_address.as_deref()) {
            Some(a) => a,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "send_transaction".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("钱包地址未提供".into()),
                }
            }
        };

        // Validate to_address (hex + EIP-55 checksum, F-016) and normalize.
        let to_address = match validate_evm_address(&to_address) {
            Ok(canonical) => canonical,
            Err(e) => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "send_transaction".into(),
                    success: false,
                    result: Value::Null,
                    error: Some(format!("Invalid to_address: {}", e)),
                };
            }
        };

        // F-013: Cross-validate the LLM-chosen recipient. If the user's ORIGINAL
        // message literally contains one or more 0x addresses, the tool-chosen
        // to_address MUST be one of them. This prevents indirect prompt injection
        // (e.g. malicious portfolio/contact data injected into the AI context)
        // from redirecting a transfer the user explicitly addressed to a typed
        // address. When the user did not type any address (referring to a contact
        // by name), we fall through to the mandatory user-confirmation card.
        if let Some(msg) = &self.user_message {
            let typed_addresses = extract_0x_addresses(msg);
            if !typed_addresses.is_empty() {
                let to_lower = to_address.to_lowercase();
                let matches = typed_addresses.iter().any(|a| a.to_lowercase() == to_lower);
                if !matches {
                    tracing::warn!(
                        "send_transaction to_address {} does not match any address typed by the user; rejecting (possible prompt injection)",
                        to_address
                    );
                    return ToolExecutionResult {
                        tool_id: tool_id.to_string(),
                        tool_name: "send_transaction".into(),
                        success: false,
                        result: Value::Null,
                        error: Some("收款地址与您消息中提供的地址不一致，已拒绝。请重新确认收款地址。(recipient address does not match the address you provided)".into()),
                    };
                }
            }
        }

        // Parse value - support both smallest-unit (integer) and human-readable (decimal) formats
        let value_wei_str: String;
        let value_u256 = if value.contains('.') {
            // Human-readable amount - convert to smallest unit with exact integer
            // math (F-015). f64 would silently alter the amount the user approved.
            match parse_decimal_to_smallest(&value, decimals as u32) {
                Ok(v) => {
                    value_wei_str = v.to_string();
                    v
                }
                Err(e) => {
                    return ToolExecutionResult {
                        tool_id: tool_id.to_string(),
                        tool_name: "send_transaction".into(),
                        success: false,
                        result: Value::Null,
                        error: Some(format!("Invalid value format: {}", e)),
                    };
                }
            }
        } else {
            match alloy_primitives::U256::from_str_radix(&value, 10) {
                Ok(v) => {
                    value_wei_str = value.clone();
                    v
                }
                Err(_) => {
                    return ToolExecutionResult {
                        tool_id: tool_id.to_string(),
                        tool_name: "send_transaction".into(),
                        success: false,
                        result: Value::Null,
                        error: Some("Invalid value format. Expected numeric string".into()),
                    };
                }
            }
        };

        let value_formatted = format_units(value_u256, decimals as u32);

        // Estimate gas via RPC
        let gas_estimate = if let Some(ref ca) = contract_address {
            // ERC-20: estimate gas for transfer(to, amount) call on the contract
            self.estimate_gas_for_erc20_transfer(
                &format!("0x{:x}", from_address),
                ca,
                &to_address,
                &value_wei_str,
                chain_id,
            )
            .await
        } else {
            self.estimate_gas_for_transfer(
                &format!("0x{:x}", from_address),
                &to_address,
                &value_wei_str,
                chain_id,
            )
            .await
        };

        let is_native = contract_address.is_none();

        // Pre-check balance for native token transfers (amount + gas vs balance)
        let mut needs_deduction = false;
        let mut max_sendable_str: Option<String> = None;
        let mut balance_str: Option<String> = None;
        let mut gas_cost_wei: Option<u128> = None;

        if is_native && !send_all {
            if let Some(native_balance) = self
                .get_native_balance(&format!("0x{:x}", from_address), chain_id)
                .await
            {
                let gas_wei = gas_estimate.gas_units as u128
                    * self.get_gas_price_wei(chain_id).await.unwrap_or(0);
                let total_needed = value_wei_str.parse::<u128>().unwrap_or(0) + gas_wei;
                if total_needed > native_balance && native_balance > gas_wei {
                    needs_deduction = true;
                    let max_send = native_balance - gas_wei;
                    max_sendable_str = Some(format_units(
                        alloy_primitives::U256::from(max_send),
                        decimals as u32,
                    ));
                    balance_str = Some(format_units(
                        alloy_primitives::U256::from(native_balance),
                        decimals as u32,
                    ));
                    gas_cost_wei = Some(gas_wei);
                }
            }
        }

        // --- Policy Engine Evaluation ---
        let policy_result = self
            .evaluate_transfer_policy(
                &format!("0x{:x}", from_address),
                &to_address,
                &token_str,
                chain_id,
                value_u256,
                decimals,
            )
            .await;

        // If policy rejects, return early with violation info
        if !policy_result.allowed {
            let violation =
                policy_result
                    .violation
                    .unwrap_or(policy_engine::limits::PolicyViolation {
                        reason: "Policy check failed".into(),
                        limit: "unknown".into(),
                    });
            return ToolExecutionResult {
                tool_id: tool_id.to_string(),
                tool_name: "send_transaction".into(),
                success: true,
                result: serde_json::json!({
                    "status": "policy_rejected",
                    "from": format!("0x{:x}", from_address),
                    "to": to_address,
                    "value_formatted": format!("{} {}", value_formatted, token_str),
                    "chain_id": chain_id,
                    "policy_violation": {
                        "reason": violation.reason,
                        "limit": violation.limit,
                    },
                }),
                error: None,
            };
        }

        let mut result = serde_json::json!({
            "status": "prepared",
            "from": format!("0x{:x}", from_address),
            "to": to_address,
            "value": value_wei_str,
            "value_formatted": format!("{} {}", value_formatted, token_str),
            "chain_id": chain_id,
            "token": token_str,
            "is_native": is_native,
            "decimals": decimals,
            "send_all": send_all,
            "estimated_gas": gas_estimate.gas_units,
            "warning": "This transaction requires your biometric confirmation before being signed and broadcast. Please verify all parameters carefully.",
            "next_step": "Review the details above and confirm with your biometric authentication to proceed"
        });

        // Add policy warnings if any
        if !policy_result.warnings.is_empty() {
            result["policy_warnings"] = serde_json::json!(policy_result.warnings);
        }
        if policy_result.requires_extra_confirmation {
            result["requires_extra_confirmation"] = serde_json::json!(true);
        }

        if let Some(ref ca) = contract_address {
            result["contract_address"] = serde_json::json!(ca);
        }

        // Add gas cost estimate if available
        if let Some(ref cost_eth) = gas_estimate.cost_eth {
            result["gas_estimate"] = serde_json::json!({
                "gas_units": gas_estimate.gas_units,
                "gas_price_gwei": gas_estimate.gas_price_gwei,
                "cost_eth": cost_eth,
                "cost_usd": gas_estimate.cost_usd,
            });
        }

        // If amount + gas > balance, include deduction info so frontend shows it directly
        if needs_deduction {
            if let (Some(ref max_send), Some(ref balance), Some(gas_cost)) =
                (&max_sendable_str, &balance_str, gas_cost_wei)
            {
                let gas_formatted =
                    format_units(alloy_primitives::U256::from(gas_cost), decimals as u32);
                result["needs_deduction"] = serde_json::json!({
                    "original_amount": value_formatted,
                    "max_sendable": max_send,
                    "gas_cost": gas_formatted,
                    "balance": balance,
                });
            }
        }

        ToolExecutionResult {
            tool_id: tool_id.to_string(),
            tool_name: "send_transaction".into(),
            success: true,
            result,
            error: None,
        }
    }

    /// Estimate gas for an ERC-20 transfer(to, amount) call
    async fn estimate_gas_for_erc20_transfer(
        &self,
        from: &str,
        contract: &str,
        to: &str,
        amount_raw: &str,
        chain_id: u64,
    ) -> GasEstimate {
        let rpc_url = self.app_state.rpc_for_chain(chain_id);
        let http = &self.app_state.http;

        // Encode ERC-20 transfer(address,uint256) calldata
        // selector: 0xa9059cbb
        let to_padded = format!("{:0>64}", to.trim_start_matches("0x"));
        let amount_u256 = amount_raw.parse::<u128>().unwrap_or(0);
        let amount_padded = format!("{:064x}", amount_u256);
        let data = format!("0xa9059cbb{}{}", to_padded, amount_padded);

        let tx_obj = serde_json::json!({
            "from": from,
            "to": contract,
            "data": data,
        });

        let estimate_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_estimateGas",
            "params": [tx_obj, "latest"],
            "id": 1
        });

        let gas_price_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 2
        });

        let (estimate_resp, price_resp) = tokio::join!(
            http.post(rpc_url).json(&estimate_body).send(),
            http.post(rpc_url).json(&gas_price_body).send(),
        );

        let gas_units = if let Ok(resp) = estimate_resp {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let hex = json
                        .get("result")
                        .and_then(|r| r.as_str())
                        .unwrap_or("0x10000");
                    u64::from_str_radix(hex.strip_prefix("0x").unwrap_or(hex), 16).unwrap_or(65000)
                }
                Err(_) => 65000,
            }
        } else {
            65000 // Default for ERC-20 transfer
        };

        let gas_price_wei = if let Ok(resp) = price_resp {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let hex = json.get("result").and_then(|r| r.as_str()).unwrap_or("0x0");
                    u128::from_str_radix(hex.strip_prefix("0x").unwrap_or(hex), 16).unwrap_or(0)
                }
                Err(_) => 0,
            }
        } else {
            0
        };

        if gas_price_wei == 0 {
            return GasEstimate {
                gas_units,
                gas_price_gwei: None,
                cost_eth: None,
                cost_usd: None,
            };
        }

        let gas_price_gwei = gas_price_wei as f64 / 1e9;
        let cost_wei = gas_units as u128 * gas_price_wei;
        let cost_eth = cost_wei as f64 / 1e18;

        let native_sym = crate::services::okx::native_symbol(chain_id);
        let cost_usd = self
            .app_state
            .price_cache
            .get_usd_price(&self.app_state.http, native_sym)
            .await
            .map(|native_price| format!("${:.2}", cost_eth * native_price));

        GasEstimate {
            gas_units,
            gas_price_gwei: Some(format!("{:.2}", gas_price_gwei)),
            cost_eth: Some(format!("{:.6}", cost_eth)),
            cost_usd,
        }
    }

    /// Estimate gas for a simple ETH transfer via RPC
    async fn estimate_gas_for_transfer(
        &self,
        from: &str,
        to: &str,
        value_wei: &str,
        chain_id: u64,
    ) -> GasEstimate {
        let rpc_url = self.app_state.rpc_for_chain(chain_id);
        let http = &self.app_state.http;

        // Convert value to hex for RPC
        let value_hex = match value_wei.parse::<u128>() {
            Ok(v) => format!("0x{:x}", v),
            Err(_) => "0x0".to_string(),
        };

        let tx_obj = serde_json::json!({
            "from": from,
            "to": to,
            "value": value_hex,
        });

        let estimate_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_estimateGas",
            "params": [tx_obj, "latest"],
            "id": 1
        });

        let gas_price_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 2
        });

        // Execute both RPC calls concurrently
        let (estimate_resp, price_resp) = tokio::join!(
            http.post(rpc_url).json(&estimate_body).send(),
            http.post(rpc_url).json(&gas_price_body).send(),
        );

        // Parse gas units
        let gas_units = if let Ok(resp) = estimate_resp {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let hex = json
                        .get("result")
                        .and_then(|r| r.as_str())
                        .unwrap_or("0x5208");
                    u64::from_str_radix(hex.strip_prefix("0x").unwrap_or(hex), 16).unwrap_or(21000)
                }
                Err(_) => 21000,
            }
        } else {
            21000 // Default for simple ETH transfer
        };

        // Parse gas price
        let gas_price_wei = if let Ok(resp) = price_resp {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let hex = json.get("result").and_then(|r| r.as_str()).unwrap_or("0x0");
                    u128::from_str_radix(hex.strip_prefix("0x").unwrap_or(hex), 16).unwrap_or(0)
                }
                Err(_) => 0,
            }
        } else {
            0
        };

        if gas_price_wei == 0 {
            return GasEstimate {
                gas_units,
                gas_price_gwei: None,
                cost_eth: None,
                cost_usd: None,
            };
        }

        let gas_price_gwei = gas_price_wei as f64 / 1e9;
        let cost_wei = gas_units as u128 * gas_price_wei;
        let cost_eth = cost_wei as f64 / 1e18;

        // Try to get native token price for USD conversion
        let native_sym = crate::services::okx::native_symbol(chain_id);
        let cost_usd = self
            .app_state
            .price_cache
            .get_usd_price(&self.app_state.http, native_sym)
            .await
            .map(|native_price| format!("${:.2}", cost_eth * native_price));

        GasEstimate {
            gas_units,
            gas_price_gwei: Some(format!("{:.2}", gas_price_gwei)),
            cost_eth: Some(format!("{:.6}", cost_eth)),
            cost_usd,
        }
    }
}
