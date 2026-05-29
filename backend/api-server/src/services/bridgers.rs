//! Bridgers cross-chain swap API client.
//!
//! Provides token swap quotes and transaction building for same-chain and cross-chain swaps.
//! API docs: https://docs-bridgers.bridgers.xyz
//! Env: `BRIDGERS_SOURCE_FLAG` (required, platform identifier)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::retry::{retry_with_backoff, RetryConfig};

const BRIDGERS_API_BASE: &str = "https://api.bridgers.xyz";

pub const NATIVE_TOKEN_ADDRESS: &str = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn is_retryable_error(err: &String) -> bool {
    err.contains("request failed")
        || err.contains("timed out")
        || err.contains("connection")
        || err.contains("502")
        || err.contains("503")
        || err.contains("777")
}

/// Map chain ID to Bridgers chain name (for fromTokenChain/toTokenChain)
pub fn chain_name(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 => Some("ETH"),
        8453 => Some("BASE"),
        42161 => Some("ARBITRUM"),
        10 => Some("OPTIMISM"),
        56 => Some("BSC"),
        137 => Some("POLYGON"),
        43114 => Some("AVALANCHE"),
        250 => Some("FTM"),
        324 => Some("ZKSYNC"),
        59144 => Some("LINEA"),
        534352 => Some("SCROLL"),
        _ => None,
    }
}

/// Map chain ID to coin code chain suffix (for fromCoinCode/toCoinCode SYMBOL(CHAIN) format)
pub fn coin_code_chain(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 => Some("ERC20"),
        8453 => Some("BASE"),
        42161 => Some("ARB"),
        10 => Some("Optimism"),
        56 => Some("BSC"),
        137 => Some("POL"),
        43114 => Some("C-Chain"),
        250 => Some("FTM"),
        324 => Some("ZKSYNC"),
        59144 => Some("LINEA"),
        534352 => Some("SCROLL"),
        _ => None,
    }
}

/// Build coin code in SYMBOL(CHAIN) format required by Bridgers
pub fn build_coin_code(symbol: &str, chain_id: u64) -> String {
    let sym = symbol.to_uppercase();
    if chain_id == 1 && sym == "ETH" {
        return "ETH".to_string();
    }
    let chain = coin_code_chain(chain_id).unwrap_or("ERC20");
    format!("{}({})", sym, chain)
}

/// Get token decimals for a known symbol
pub fn token_decimals(symbol: &str) -> u32 {
    match symbol.to_uppercase().as_str() {
        "USDC" | "USDT" => 6,
        _ => 18,
    }
}

/// Well-known token addresses per chain (same as before, used for symbol→address resolution)
pub fn token_address(symbol: &str, chain_id: u64) -> Option<&'static str> {
    let s = symbol.to_uppercase();
    match (s.as_str(), chain_id) {
        ("ETH", 1 | 8453 | 42161 | 10) => Some(NATIVE_TOKEN_ADDRESS),
        ("BNB", 56) => Some(NATIVE_TOKEN_ADDRESS),
        ("POL" | "MATIC", 137) => Some(NATIVE_TOKEN_ADDRESS),
        ("WETH", 1) => Some("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        ("WETH", 8453) => Some("0x4200000000000000000000000000000000000006"),
        ("WETH", 42161) => Some("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"),
        ("WETH", 10) => Some("0x4200000000000000000000000000000000000006"),
        ("USDC", 1) => Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        ("USDC", 8453) => Some("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
        ("USDC", 42161) => Some("0xaf88d065e77c8cC2239327C5EDb3A432268e5831"),
        ("USDC", 10) => Some("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85"),
        ("USDC", 137) => Some("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"),
        ("USDC", 56) => Some("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d"),
        ("USDT", 1) => Some("0xdAC17F958D2ee523a2206206994597C13D831ec7"),
        ("USDT", 8453) => Some("0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2"),
        ("USDT", 42161) => Some("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"),
        ("USDT", 10) => Some("0x94b008aA00579c1307B0EF2c499aD98a8ce58e58"),
        ("USDT", 137) => Some("0xc2132D05D31c914a87C6611C10748AEb04B58e8F"),
        ("USDT", 56) => Some("0x55d398326f99059fF775485246999027B3197955"),
        ("DAI", 1) => Some("0x6B175474E89094C44Da98b954EedeAC495271d0F"),
        ("DAI", 8453) => Some("0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb"),
        ("DAI", 42161) => Some("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"),
        ("DAI", 10) => Some("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"),
        ("WBNB", 56) => Some("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"),
        _ => None,
    }
}

// ─── Response types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BridgersResponse<T> {
    data: Option<T>,
    #[serde(alias = "resCode")]
    res_code: Option<Value>,
    #[serde(alias = "resMsg")]
    res_msg: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteDataWrapper {
    tx_data: Option<QuoteData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteData {
    amount_out_min: Option<Value>,
    to_token_amount: Option<Value>,
    from_token_decimal: Option<Value>,
    instant_rate: Option<Value>,
    chain_fee: Option<Value>,
    fee: Option<Value>,
    contract_address: Option<String>,
    deposit_min: Option<Value>,
    deposit_max: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwapDataWrapper {
    tx_data: Option<SwapTxData>,
}

#[derive(Debug, Deserialize)]
struct SwapTxData {
    data: Option<String>,
    to: Option<String>,
    value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgersToken {
    pub chain: Option<String>,
    pub symbol: Option<String>,
    pub name: Option<String>,
    #[serde(alias = "nickName")]
    pub nick_name: Option<String>,
    pub address: Option<String>,
    pub decimals: Option<u32>,
    #[serde(alias = "logoURI")]
    pub logo_uri: Option<String>,
    #[serde(alias = "isCrossEnable")]
    pub is_cross_enable: Option<Value>,
    #[serde(alias = "chainId")]
    pub chain_id: Option<Value>,
}

// ─── Public output types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SwapQuote {
    pub sell_token: String,
    pub buy_token: String,
    pub sell_amount: String,
    pub buy_amount: String,
    pub buy_amount_min: String,
    pub price: String,
    pub estimated_gas: String,
    pub fee_rate: String,
    pub chain_fee: String,
    pub contract_address: String,
    pub from_chain: String,
    pub to_chain: String,
    pub deposit_min: Option<String>,
    pub deposit_max: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapTransaction {
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas_estimate: String,
    pub sell_token: String,
    pub buy_token: String,
    pub sell_amount: String,
    pub buy_amount: String,
    pub price: String,
    pub allowance_target: Option<String>,
    pub from_chain_id: u64,
    pub to_chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatus {
    pub order_id: String,
    pub status: String,
    pub from_hash: Option<String>,
    pub to_hash: Option<String>,
}

// ─── Public API ─────────────────────────────────────────────────────────────

pub async fn get_quote(
    http: &Client,
    source_flag: &str,
    from_chain_id: u64,
    to_chain_id: u64,
    from_token_address: &str,
    to_token_address: &str,
    from_token_amount: &str,
    from_coin_code: &str,
    to_coin_code: &str,
    user_addr: Option<&str>,
) -> Result<SwapQuote, String> {
    let from_chain = chain_name(from_chain_id)
        .ok_or_else(|| format!("Unsupported source chain: {}", from_chain_id))?;
    let to_chain = chain_name(to_chain_id)
        .ok_or_else(|| format!("Unsupported destination chain: {}", to_chain_id))?;

    let equipment_no = user_addr.unwrap_or("cowallet-default");

    tracing::info!(
        "[Bridgers] quote {}({}) -> {}({}) amount={}",
        from_coin_code, from_chain, to_coin_code, to_chain, from_token_amount
    );

    let mut body = serde_json::json!({
        "fromTokenAddress": from_token_address,
        "toTokenAddress": to_token_address,
        "fromTokenAmount": from_token_amount,
        "fromTokenChain": from_chain,
        "toTokenChain": to_chain,
        "fromCoinCode": from_coin_code,
        "toCoinCode": to_coin_code,
        "equipmentNo": equipment_no,
        "sourceFlag": source_flag,
    });
    if let Some(addr) = user_addr {
        body["userAddr"] = Value::String(addr.to_string());
    }

    let http_c = http.clone();
    let body_c = body.clone();

    let resp: BridgersResponse<QuoteDataWrapper> = retry_with_backoff(
        RetryConfig::conservative(),
        || {
            let h = http_c.clone();
            let b = body_c.clone();
            async move {
                let r = h.post(format!("{}/api/sswap/quote", BRIDGERS_API_BASE))
                    .json(&b).send().await
                    .map_err(|e| format!("Bridgers quote request failed: {}", e))?;
                let status = r.status();
                if !status.is_success() {
                    let t = r.text().await.unwrap_or_default();
                    return Err(format!("Bridgers API returned {}: {}", status, t));
                }
                r.json::<BridgersResponse<QuoteDataWrapper>>().await
                    .map_err(|e| format!("Bridgers quote parse error: {}", e))
            }
        },
        is_retryable_error,
        "bridgers_get_quote",
    ).await?;

    if !is_success_code(&resp.res_code) {
        let msg = resp.res_msg.unwrap_or_else(|| "Unknown error".into());
        return Err(format!("Bridgers quote error: {}", msg));
    }

    let wrapper = resp.data.ok_or("Bridgers returned empty quote data")?;
    let data = wrapper.tx_data.ok_or("Bridgers returned empty txData in quote")?;

    tracing::debug!("[Bridgers] quote raw data: {:?}", data);

    let buy_amount = value_to_string(&data.to_token_amount).unwrap_or_else(|| "0".into());
    let buy_amount_min = value_to_string(&data.amount_out_min).unwrap_or_else(|| buy_amount.clone());
    // Bridgers' instantRate is already buy-per-sell in human-readable terms. Prefer it.
    // Fallback: compute from human buy_amount / human sell_amount (from_token_amount is RAW wei,
    // so convert it down by from_token_decimal first).
    let price = value_to_string(&data.instant_rate)
        .filter(|r| r.parse::<f64>().map(|v| v > 0.0).unwrap_or(false))
        .unwrap_or_else(|| {
            let from_decimals = data.from_token_decimal
                .as_ref()
                .and_then(|v| v.as_u64())
                .unwrap_or(18) as i32;
            let sell_raw: f64 = from_token_amount.parse().unwrap_or(0.0);
            let sell_human = sell_raw / 10f64.powi(from_decimals);
            let buy_f: f64 = buy_amount.parse().unwrap_or(0.0);
            if sell_human > 0.0 { format!("{:.10}", buy_f / sell_human) } else { "0".into() }
        });
    let fee_rate = data.fee.map(|v| match v {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s,
        _ => "0.002".into(),
    }).unwrap_or_else(|| "0.002".into());
    let chain_fee = value_to_string(&data.chain_fee).unwrap_or_else(|| "0".into());

    Ok(SwapQuote {
        sell_token: from_token_address.to_string(),
        buy_token: to_token_address.to_string(),
        sell_amount: from_token_amount.to_string(),
        buy_amount,
        buy_amount_min,
        price,
        estimated_gas: "200000".into(),
        fee_rate,
        chain_fee,
        contract_address: data.contract_address.unwrap_or_default(),
        from_chain: from_chain.to_string(),
        to_chain: to_chain.to_string(),
        deposit_min: value_to_string(&data.deposit_min),
        deposit_max: value_to_string(&data.deposit_max),
    })
}

pub async fn build_swap_tx(
    http: &Client,
    source_flag: &str,
    from_chain_id: u64,
    to_chain_id: u64,
    from_token_address: &str,
    to_token_address: &str,
    from_address: &str,
    to_address: &str,
    from_token_amount: &str,
    amount_out_min: &str,
    from_coin_code: &str,
    to_coin_code: &str,
    slippage: f64,
) -> Result<SwapTransaction, String> {
    let from_chain = chain_name(from_chain_id)
        .ok_or_else(|| format!("Unsupported source chain: {}", from_chain_id))?;
    let to_chain = chain_name(to_chain_id)
        .ok_or_else(|| format!("Unsupported destination chain: {}", to_chain_id))?;

    let equipment_no = &from_address[..std::cmp::min(32, from_address.len())];

    // Sanitize amounts: Bridgers requires integer strings (no decimals)
    let clean_from_amount = sanitize_amount(from_token_amount);
    let clean_amount_out_min = sanitize_amount(amount_out_min);
    // If amountOutMin is "0" or empty, use "1" (accept any positive output)
    let final_amount_out_min = if clean_amount_out_min == "0" || clean_amount_out_min.is_empty() {
        "1".to_string()
    } else {
        clean_amount_out_min.clone()
    };

    tracing::info!(
        "[Bridgers] swap fromTokenAmount={} amountOutMin={} slippage={} (raw inputs: {} / {})",
        clean_from_amount, final_amount_out_min, slippage, from_token_amount, amount_out_min
    );

    let body = serde_json::json!({
        "fromTokenAddress": from_token_address,
        "toTokenAddress": to_token_address,
        "fromAddress": from_address,
        "toAddress": to_address,
        "fromTokenChain": from_chain,
        "toTokenChain": to_chain,
        "fromTokenAmount": clean_from_amount,
        "amountOutMin": final_amount_out_min,
        "fromCoinCode": from_coin_code,
        "toCoinCode": to_coin_code,
        "equipmentNo": equipment_no,
        "sourceFlag": source_flag,
        "slippage": format!("{}", slippage),
    });

    let http_c = http.clone();
    let body_c = body.clone();

    let resp: BridgersResponse<SwapDataWrapper> = retry_with_backoff(
        RetryConfig::conservative(),
        || {
            let h = http_c.clone();
            let b = body_c.clone();
            async move {
                let r = h.post(format!("{}/api/sswap/swap", BRIDGERS_API_BASE))
                    .json(&b).send().await
                    .map_err(|e| format!("Bridgers swap request failed: {}", e))?;
                let status = r.status();
                if !status.is_success() {
                    let t = r.text().await.unwrap_or_default();
                    return Err(format!("Bridgers API returned {}: {}", status, t));
                }
                r.json::<BridgersResponse<SwapDataWrapper>>().await
                    .map_err(|e| format!("Bridgers swap parse error: {}", e))
            }
        },
        is_retryable_error,
        "bridgers_build_swap",
    ).await?;

    if !is_success_code(&resp.res_code) {
        let msg = resp.res_msg.unwrap_or_else(|| "Unknown error".into());
        return Err(format!("Bridgers swap error: {}", msg));
    }

    let wrapper = resp.data.ok_or("Bridgers returned empty swap data")?;
    let tx_data = wrapper.tx_data.ok_or("Bridgers returned empty txData")?;
    let sell_f: f64 = from_token_amount.parse().unwrap_or(1.0);

    let raw_value = value_to_string(&tx_data.value).unwrap_or_else(|| "0".into());
    let decimal_value = hex_to_decimal(&raw_value);

    Ok(SwapTransaction {
        to: tx_data.to.unwrap_or_default(),
        data: tx_data.data.unwrap_or_default(),
        value: decimal_value,
        gas_estimate: "200000".into(),
        sell_token: from_token_address.to_string(),
        buy_token: to_token_address.to_string(),
        sell_amount: from_token_amount.to_string(),
        buy_amount: amount_out_min.to_string(),
        price: format!("{:.10}", sell_f),
        allowance_target: None,
        from_chain_id,
        to_chain_id,
    })
}

pub async fn get_tokens(
    http: &Client,
    chain: Option<&str>,
) -> Result<Vec<BridgersToken>, String> {
    let body = serde_json::json!({ "chain": chain });

    let resp = http.post(format!("{}/api/exchangeRecord/getToken", BRIDGERS_API_BASE))
        .json(&body).send().await
        .map_err(|e| format!("Bridgers getToken request failed: {}", e))?;

    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        return Err(format!("Bridgers getToken error: {}", t));
    }

    let parsed: BridgersResponse<Value> = resp.json().await
        .map_err(|e| format!("Bridgers getToken parse error: {}", e))?;

    let data = parsed.data.unwrap_or(Value::Object(Default::default()));
    let tokens_val = data.get("tokens").cloned().unwrap_or(Value::Array(vec![]));
    let tokens: Vec<BridgersToken> = serde_json::from_value(tokens_val)
        .unwrap_or_default();

    Ok(tokens)
}

pub async fn upload_order_hash(
    http: &Client,
    source_flag: &str,
    hash: &str,
    from_token_address: &str,
    to_token_address: &str,
    from_address: &str,
    to_address: &str,
    from_chain_id: u64,
    to_chain_id: u64,
    from_token_amount: &str,
    amount_out_min: &str,
    from_coin_code: &str,
    to_coin_code: &str,
) -> Result<String, String> {
    let from_chain = chain_name(from_chain_id)
        .ok_or_else(|| format!("Unsupported source chain: {}", from_chain_id))?;
    let to_chain = chain_name(to_chain_id)
        .ok_or_else(|| format!("Unsupported destination chain: {}", to_chain_id))?;

    let equipment_no = &from_address[..std::cmp::min(32, from_address.len())];

    let body = serde_json::json!({
        "hash": hash,
        "fromTokenAddress": from_token_address,
        "toTokenAddress": to_token_address,
        "fromAddress": from_address,
        "toAddress": to_address,
        "fromTokenChain": from_chain,
        "toTokenChain": to_chain,
        "fromTokenAmount": sanitize_amount(from_token_amount),
        "amountOutMin": sanitize_amount(amount_out_min),
        "fromCoinCode": from_coin_code,
        "toCoinCode": to_coin_code,
        "equipmentNo": equipment_no,
        "sourceFlag": source_flag,
    });

    let resp = http.post(format!("{}/api/exchangeRecord/updateDataAndStatus", BRIDGERS_API_BASE))
        .json(&body).send().await
        .map_err(|e| format!("Bridgers upload order failed: {}", e))?;

    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        return Err(format!("Bridgers upload order error: {}", t));
    }

    let parsed: BridgersResponse<Value> = resp.json().await
        .map_err(|e| format!("Bridgers upload order parse error: {}", e))?;

    let order_id = parsed.data
        .and_then(|d| d.get("orderId").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .or_else(|| parsed.res_msg.clone())
        .unwrap_or_else(|| "unknown".into());

    Ok(order_id)
}

pub async fn get_order_status(
    http: &Client,
    order_id: &str,
) -> Result<OrderStatus, String> {
    let body = serde_json::json!({ "orderId": order_id });

    let resp = http.post(format!("{}/api/exchangeRecord/getTransDataById", BRIDGERS_API_BASE))
        .json(&body).send().await
        .map_err(|e| format!("Bridgers order status request failed: {}", e))?;

    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        return Err(format!("Bridgers order status error: {}", t));
    }

    let parsed: BridgersResponse<Value> = resp.json().await
        .map_err(|e| format!("Bridgers order status parse error: {}", e))?;

    let data = parsed.data.unwrap_or(Value::Object(Default::default()));
    let status = data.get("status").and_then(|v| v.as_str())
        .unwrap_or("pending").to_string();
    let from_hash = data.get("fromHash").and_then(|v| v.as_str()).map(|s| s.to_string());
    let to_hash = data.get("toHash").and_then(|v| v.as_str()).map(|s| s.to_string());

    Ok(OrderStatus {
        order_id: order_id.to_string(),
        status,
        from_hash,
        to_hash,
    })
}

pub async fn query_records(
    http: &Client,
    source_flag: &str,
    from_address: &str,
    equipment_no: &str,
    page_no: u32,
    page_size: u32,
) -> Result<Value, String> {
    let body = serde_json::json!({
        "equipmentNo": equipment_no,
        "sourceFlag": source_flag,
        "pageNo": page_no,
        "pageSize": page_size,
        "fromAddress": from_address,
    });

    let resp = http.post(format!("{}/api/exchangeRecord/getTransData", BRIDGERS_API_BASE))
        .json(&body).send().await
        .map_err(|e| format!("Bridgers queryRecords request failed: {}", e))?;

    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        return Err(format!("Bridgers queryRecords error: {}", t));
    }

    let parsed: BridgersResponse<Value> = resp.json().await
        .map_err(|e| format!("Bridgers queryRecords parse error: {}", e))?;

    Ok(parsed.data.unwrap_or(Value::Array(vec![])))
}

// ─── Utility functions ─────────────────────────────────────────────────────

pub fn amount_to_raw(amount: &str, decimals: u32) -> Result<String, String> {
    let amt = amount.trim();
    if amt.is_empty() || amt == "0" {
        return Err("Amount must be positive".to_string());
    }
    let negative = amt.starts_with('-');
    if negative { return Err("Amount must be positive".to_string()); }

    let (integer, fraction) = match amt.split_once('.') {
        Some((i, f)) => (i, f.trim_end_matches('0')),
        None => (amt, ""),
    };

    let frac_len = fraction.len() as u32;
    let mut result = String::with_capacity(integer.len() + decimals as usize);
    result.push_str(integer);
    if frac_len <= decimals {
        result.push_str(fraction);
        for _ in 0..(decimals - frac_len) {
            result.push('0');
        }
    } else {
        result.push_str(&fraction[..decimals as usize]);
    }
    let trimmed = result.trim_start_matches('0');
    if trimmed.is_empty() {
        return Err("Amount must be positive".to_string());
    }
    Ok(trimmed.to_string())
}

pub fn raw_to_amount(raw: &str, decimals: u32) -> String {
    let value: f64 = raw.parse().unwrap_or(0.0);
    let amount = value / 10f64.powi(decimals as i32);
    if decimals <= 6 { format!("{:.2}", amount) } else { format!("{:.6}", amount) }
}

fn sanitize_amount(s: &str) -> String {
    let s = s.trim();
    match s.split_once('.') {
        Some((int, _)) => {
            let t = int.trim_start_matches('0');
            if t.is_empty() { "0".to_string() } else { t.to_string() }
        }
        None => {
            let t = s.trim_start_matches('0');
            if t.is_empty() { "0".to_string() } else { t.to_string() }
        }
    }
}

fn is_success_code(code: &Option<Value>) -> bool {
    match code {
        Some(Value::String(s)) => s == "100",
        Some(Value::Number(n)) => n.as_i64() == Some(100) || n.as_u64() == Some(100),
        _ => false,
    }
}

fn value_to_string(v: &Option<Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                Some(i.to_string())
            } else if let Some(u) = n.as_u64() {
                Some(u.to_string())
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 {
                    Some(format!("{:.0}", f))
                } else {
                    Some(format!("{}", f))
                }
            } else {
                Some(n.to_string())
            }
        }
        _ => None,
    }
}

fn hex_to_decimal(s: &str) -> String {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u128::from_str_radix(hex, 16)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| s.to_string())
    } else {
        s.to_string()
    }
}
