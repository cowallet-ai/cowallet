//! Token swap quoting tool.

use super::{infer_chain_id_from_token, parse_param, ToolContext, ToolExecutionResult};
use serde_json::Value;

impl ToolContext {
    // --- swap_token ---
    pub(super) async fn execute_swap_token(&self, tool_id: &str, params: Value) -> ToolExecutionResult {
        let from_token: String = match parse_param(&params, "from_token") {
            Some(t) => t,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "swap_token".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("Missing required parameter: from_token".into()),
                };
            }
        };

        let to_token: String = match parse_param(&params, "to_token") {
            Some(t) => t,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "swap_token".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("Missing required parameter: to_token".into()),
                };
            }
        };

        let amount: String = match parse_param(&params, "amount") {
            Some(a) => a,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "swap_token".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("Missing required parameter: amount".into()),
                };
            }
        };

        let slippage: f64 = parse_param(&params, "slippage").unwrap_or(0.5);
        let chain_id: u64 = match parse_param(&params, "chain_id")
            .or_else(|| infer_chain_id_from_token(&from_token))
        {
            Some(id) => id,
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "swap_token".into(),
                    success: false,
                    result: Value::Null,
                    error: Some("Cannot determine target chain. Please ask the user which chain to use for this swap. Multi-chain tokens (USDC, USDT, DAI, WETH, LINK) require an explicit chain_id.".into()),
                };
            }
        };
        let to_chain_id: u64 = parse_param(&params, "to_chain_id").unwrap_or(chain_id);

        let from_upper = from_token.to_uppercase();
        let to_upper = to_token.to_uppercase();

        // Resolve token addresses for Bridgers API
        let sell_addr = match crate::services::bridgers::token_address(&from_upper, chain_id) {
            Some(addr) => addr.to_string(),
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "swap_token".into(),
                    success: false,
                    result: Value::Null,
                    error: Some(format!("不支持的代币: {} (chain {})", from_upper, chain_id)),
                };
            }
        };
        let buy_addr = match crate::services::bridgers::token_address(&to_upper, to_chain_id) {
            Some(addr) => addr.to_string(),
            None => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "swap_token".into(),
                    success: false,
                    result: Value::Null,
                    error: Some(format!("不支持的代币: {} (chain {})", to_upper, to_chain_id)),
                };
            }
        };

        // Convert amount to raw units
        let sell_decimals = crate::services::bridgers::token_decimals(&from_upper);
        let raw_amount = match crate::services::bridgers::amount_to_raw(&amount, sell_decimals) {
            Ok(raw) => raw,
            Err(e) => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "swap_token".into(),
                    success: false,
                    result: Value::Null,
                    error: Some(format!("无效金额: {}", e)),
                };
            }
        };

        // Try to get a real quote from Bridgers API
        let buy_decimals = crate::services::bridgers::token_decimals(&to_upper);
        let (estimated_output, exchange_rate, price_impact, gas_estimate, sources, estimated_time) =
            match crate::services::bridgers::get_quote(
                &self.app_state.http,
                &self.app_state.bridgers_source_flag,
                chain_id,
                to_chain_id,
                &sell_addr,
                &buy_addr,
                &raw_amount,
                &from_upper,
                &to_upper,
                None,
            )
            .await
            {
                Ok(quote) => {
                    // Bridgers' toTokenAmount is already human-readable; only convert if it's a raw integer
                    let output_formatted = if quote.buy_amount.contains('.') {
                        quote.buy_amount.clone()
                    } else {
                        crate::services::bridgers::raw_to_amount(&quote.buy_amount, buy_decimals)
                    };
                    (
                        output_formatted,
                        quote.price.clone(),
                        None::<String>,
                        quote.estimated_gas.clone(),
                        vec!["bridgers".to_string()],
                        quote.estimated_time,
                    )
                }
                Err(e) => {
                    tracing::warn!("[Bridgers] quote failed, falling back to price estimate: {}", e);
                    // Fallback to price-based estimation
                    let from_price = self.app_state.price_cache
                        .get_usd_price(&self.app_state.http, &from_upper)
                        .await;
                    let to_price = self.app_state.price_cache
                        .get_usd_price(&self.app_state.http, &to_upper)
                        .await;

                    match (from_price, to_price) {
                        (Some(fp), Some(tp)) if tp > 0.0 => {
                            let amt: f64 = amount.parse().unwrap_or(0.0);
                            let output = amt * fp / tp;
                            let output_str = if tp >= 1.0 { format!("{:.2}", output) } else { format!("{:.6}", output) };
                            let rate = format!("{:.6}", fp / tp);
                            (output_str, rate, None, "200000".to_string(), vec!["price_estimate".to_string()], None)
                        }
                        _ => {
                            return ToolExecutionResult {
                                tool_id: tool_id.to_string(),
                                tool_name: "swap_token".into(),
                                success: false,
                                result: Value::Null,
                                error: Some(format!("无法获取 {}/{} 报价", from_upper, to_upper)),
                            };
                        }
                    }
                }
            };

        let result = serde_json::json!({
            "status": "pending_confirmation",
            "from_token": from_upper,
            "to_token": to_upper,
            "amount": amount,
            "estimated_output": estimated_output,
            "exchange_rate": exchange_rate,
            "price_impact": price_impact,
            "gas_estimate": gas_estimate,
            "slippage": slippage,
            "chain_id": chain_id,
            "to_chain_id": to_chain_id,
            "estimated_time": estimated_time,
            "sources": sources,
            "route": format!("{} → {}", from_upper, to_upper),
            "warning": "兑换需要您确认后执行。实际到账金额可能因市场波动略有差异。",
        });

        ToolExecutionResult {
            tool_id: tool_id.to_string(),
            tool_name: "swap_token".into(),
            success: true,
            result,
            error: None,
        }
    }
}
