//! Balance and token info tools.

use super::{
    format_units, infer_chain_id_from_token, parse_param, parse_wallet_address,
    token_balance_to_json, ToolContext, ToolExecutionResult,
};
use serde_json::Value;

impl ToolContext {
    // --- get_balance ---
    pub(super) async fn execute_get_balance(&self, tool_id: &str, params: Value) -> ToolExecutionResult {
        let chain_id_filter: Option<u64> = parse_param(&params, "chain_id");
        let token_filter: Option<String> = parse_param(&params, "token");
        let owner = match parse_wallet_address(self.wallet_address.as_deref()) {
            Some(a) => a,
            None => return ToolExecutionResult {
                tool_id: tool_id.to_string(),
                tool_name: "get_balance".into(),
                success: false,
                result: Value::Null,
                error: Some("钱包地址未提供".into()),
            },
        };
        let address = format!("0x{:x}", owner);

        // Use Covalent API if configured
        if let Some(api_key) = &self.app_state.covalent_api_key {
            // Multi-chain query if no chain_id specified
            if chain_id_filter.is_none() {
                let supported_chains = vec![1u64, 8453, 42161, 10, 56, 137];
                match crate::services::covalent::get_all_chain_balances(
                    &self.app_state.http,
                    api_key,
                    &address,
                    &supported_chains,
                )
                .await
                {
                    Ok(all_balances) => {
                        let mut chains_data: Vec<serde_json::Value> = Vec::new();

                        for chain in &all_balances.chains {
                            let filtered_tokens: Vec<&crate::services::covalent::TokenBalance> =
                                if let Some(ref symbol) = token_filter {
                                    let s = symbol.to_uppercase();
                                    chain.tokens.iter().filter(|b| b.symbol.to_uppercase() == s).collect()
                                } else {
                                    chain.tokens.iter().collect()
                                };

                            if !filtered_tokens.is_empty() {
                                let tokens: Vec<serde_json::Value> = filtered_tokens
                                    .iter()
                                    .map(|b| token_balance_to_json(b))
                                    .collect();

                                chains_data.push(serde_json::json!({
                                    "chain_id": chain.chain_id,
                                    "chain_name": chain.chain_name,
                                    "tokens": tokens,
                                    "total_usd": chain.total_usd,
                                }));
                            }
                        }

                        let result = serde_json::json!({
                            "address": address,
                            "multi_chain": true,
                            "chains": chains_data,
                            "total_usd": all_balances.total_usd,
                        });

                        return ToolExecutionResult {
                            tool_id: tool_id.to_string(),
                            tool_name: "get_balance".into(),
                            success: true,
                            result,
                            error: None,
                        };
                    }
                    Err(e) => {
                        tracing::warn!("Covalent multi-chain balance query failed: {}", e);
                    }
                }
            } else {
                // Single chain query
                let chain_id = chain_id_filter.unwrap();
                match crate::services::covalent::get_balances(
                    &self.app_state.http,
                    api_key,
                    &address,
                    chain_id,
                )
                .await
                {
                    Ok(balances) => {
                        let filtered: Vec<&crate::services::covalent::TokenBalance> =
                            if let Some(ref symbol) = token_filter {
                                let s = symbol.to_uppercase();
                                balances.iter().filter(|b| b.symbol.to_uppercase() == s).collect()
                            } else {
                                balances.iter().collect()
                            };

                        let total_usd: f64 = filtered
                            .iter()
                            .filter_map(|b| b.usd.parse::<f64>().ok())
                            .sum();

                        let tokens: Vec<serde_json::Value> = filtered
                            .iter()
                            .map(|b| token_balance_to_json(b))
                            .collect();

                        let result = serde_json::json!({
                            "address": address,
                            "chain_id": chain_id,
                            "tokens": tokens,
                            "total_usd": format!("{:.2}", total_usd),
                        });

                        return ToolExecutionResult {
                            tool_id: tool_id.to_string(),
                            tool_name: "get_balance".into(),
                            success: true,
                            result,
                            error: None,
                        };
                    }
                    Err(e) => {
                        tracing::warn!("Covalent balance query failed, falling back to RPC: {}", e);
                    }
                }
            }
        }

        // Fallback: direct RPC query — default to Ethereum mainnet for native balance
        let chain_id = chain_id_filter.unwrap_or(1);
        let rpc_url = self.app_state.rpc_for_chain(chain_id);
        let result = match chain_evm::tokens::query_native_balance(owner, rpc_url).await {
            Ok(balance) => {
                let formatted = format_units(balance, 18);
                serde_json::json!({
                    "address": address,
                    "chain_id": chain_id,
                    "tokens": [{
                        "symbol": "ETH",
                        "balance": formatted,
                        "usd": "—",
                        "native": true,
                    }],
                    "total_usd": "—",
                })
            }
            Err(e) => {
                return ToolExecutionResult {
                    tool_id: tool_id.to_string(),
                    tool_name: "get_balance".into(),
                    success: false,
                    result: Value::Null,
                    error: Some(format!("Failed to query balance: {}", e)),
                };
            }
        };

        ToolExecutionResult {
            tool_id: tool_id.to_string(),
            tool_name: "get_balance".into(),
            success: true,
            result,
            error: None,
        }
    }

    // --- get_token_info ---
    pub(super) async fn execute_get_token_info(&self, tool_id: &str, params: Value) -> ToolExecutionResult {
        let token_symbol: String = parse_param(&params, "token").unwrap_or_else(|| "ETH".into());
        let chain_id_param: Option<u64> = parse_param(&params, "chain_id");
        let symbol_upper = token_symbol.to_uppercase();

        let owner = parse_wallet_address(self.wallet_address.as_deref());
        let address_str = owner.map(|a| format!("0x{:x}", a));

        // Determine which chain to query: explicit param > infer from token > search all chains
        let chain_id = chain_id_param
            .or_else(|| infer_chain_id_from_token(&token_symbol))
            .unwrap_or(0); // 0 = search all chains

        // Get balance from Covalent if available
        let mut balance_info = serde_json::json!(null);
        let mut resolved_chain_id = chain_id;
        if let (Some(api_key), Some(ref addr)) = (&self.app_state.covalent_api_key, &address_str) {
            if chain_id == 0 {
                // Multi-chain search: find which chain has this token
                let all_chains: &[u64] = &[1, 137, 8453, 42161, 10, 56];
                for &cid in all_chains {
                    if let Ok(balances) = crate::services::covalent::get_balances(
                        &self.app_state.http, api_key, addr, cid,
                    ).await {
                        if let Some(token) = balances.iter().find(|b| b.symbol.to_uppercase() == symbol_upper) {
                            resolved_chain_id = cid;
                            balance_info = serde_json::json!({
                                "balance": token.balance_formatted,
                                "balance_raw": token.balance,
                                "usd_value": token.usd,
                                "usd_24h": token.usd_24h,
                                "quote_rate": token.quote_rate,
                                "quote_rate_24h": token.quote_rate_24h,
                                "contract_address": token.contract_address,
                                "is_native": token.native_token,
                            });
                            break;
                        }
                    }
                }
            } else {
                if let Ok(balances) = crate::services::covalent::get_balances(
                    &self.app_state.http, api_key, addr, chain_id,
                ).await {
                    if let Some(token) = balances.iter().find(|b| b.symbol.to_uppercase() == symbol_upper) {
                        resolved_chain_id = chain_id;
                        balance_info = serde_json::json!({
                            "balance": token.balance_formatted,
                            "balance_raw": token.balance,
                            "usd_value": token.usd,
                            "usd_24h": token.usd_24h,
                            "quote_rate": token.quote_rate,
                            "quote_rate_24h": token.quote_rate_24h,
                            "contract_address": token.contract_address,
                            "decimals": token.decimals,
                            "is_native": token.native_token,
                            "logo_url": token.logo_url,
                            "last_transferred_at": token.last_transferred_at,
                        });
                    }
                }
            }
        }

        // Get price from PriceCache (DeFiLlama primary, CoinGecko fallback)
        let mut price_usd = self.app_state.price_cache
            .get_usd_price(&self.app_state.http, &symbol_upper)
            .await;

        // Fallback: if symbol lookup failed, try by contract address via DeFiLlama
        if price_usd.is_none() {
            let contract_addr = balance_info.get("contract_address")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty());
            if let Some(addr) = contract_addr {
                price_usd = self.app_state.price_cache
                    .get_token_price_by_address(&self.app_state.http, resolved_chain_id, addr)
                    .await;
            }
        }

        // Build known token metadata
        let token_meta = match symbol_upper.as_str() {
            "ETH" => serde_json::json!({
                "name": "Ethereum",
                "symbol": "ETH",
                "decimals": 18,
                "type": "native",
                "description": "Native gas token of Ethereum and L2 networks",
            }),
            "USDC" => serde_json::json!({
                "name": "USD Coin",
                "symbol": "USDC",
                "decimals": 6,
                "type": "ERC-20",
                "issuer": "Circle",
                "contract_address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                "description": "Fully reserved stablecoin pegged to USD, issued by Circle",
            }),
            "USDT" => serde_json::json!({
                "name": "Tether USD",
                "symbol": "USDT",
                "decimals": 6,
                "type": "ERC-20",
                "issuer": "Tether",
                "description": "Most widely used stablecoin pegged to USD",
            }),
            "WETH" => serde_json::json!({
                "name": "Wrapped Ether",
                "symbol": "WETH",
                "decimals": 18,
                "type": "ERC-20",
                "description": "ERC-20 wrapped version of ETH for DeFi compatibility",
            }),
            "DAI" => serde_json::json!({
                "name": "Dai",
                "symbol": "DAI",
                "decimals": 18,
                "type": "ERC-20",
                "issuer": "MakerDAO",
                "description": "Decentralized stablecoin backed by crypto collateral",
            }),
            _ => serde_json::json!({
                "name": symbol_upper.clone(),
                "symbol": symbol_upper.clone(),
                "type": "ERC-20",
            }),
        };

        let result = serde_json::json!({
            "token": token_meta,
            "balance": balance_info,
            "price_usd": price_usd,
            "chain_id": resolved_chain_id,
            "wallet_address": address_str,
        });

        ToolExecutionResult {
            tool_id: tool_id.to_string(),
            tool_name: "get_token_info".into(),
            success: true,
            result,
            error: None,
        }
    }

    /// Query native token balance via RPC eth_getBalance
    pub(super) async fn get_native_balance(&self, address: &str, chain_id: u64) -> Option<u128> {
        let rpc_url = self.app_state.rpc_for_chain(chain_id);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [address, "latest"],
            "id": 1
        });
        let resp = self.app_state.http.post(rpc_url).json(&body).send().await.ok()?;
        let json = resp.json::<serde_json::Value>().await.ok()?;
        let hex = json.get("result")?.as_str()?;
        u128::from_str_radix(hex.strip_prefix("0x").unwrap_or(hex), 16).ok()
    }

    /// Get current gas price in wei
    pub(super) async fn get_gas_price_wei(&self, chain_id: u64) -> Option<u128> {
        let rpc_url = self.app_state.rpc_for_chain(chain_id);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 1
        });
        let resp = self.app_state.http.post(rpc_url).json(&body).send().await.ok()?;
        let json = resp.json::<serde_json::Value>().await.ok()?;
        let hex = json.get("result")?.as_str()?;
        u128::from_str_radix(hex.strip_prefix("0x").unwrap_or(hex), 16).ok()
    }
}
