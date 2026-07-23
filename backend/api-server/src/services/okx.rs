//! OKX Web3 Wallet API client (Onchain OS) for multi-chain balance and
//! transaction-history queries. Replaces the deprecated Covalent/GoldRush client.
//!
//! Auth: HMAC-SHA256 request signing. Required env credentials:
//!   `OKX_API_KEY`, `OKX_SECRET_KEY`, `OKX_PASSPHRASE`. `OKX_PROJECT_ID` is
//!   optional (only needed for project-scoped WaaS endpoints).
//!
//! Signature: `OK-ACCESS-SIGN = Base64(HMAC-SHA256(timestamp + method + requestPath + body, secret))`
//! where `requestPath` includes the query string and `timestamp` is ISO-8601 UTC
//! with millisecond precision (e.g. `2023-10-18T12:21:41.274Z`).
//!
//! Endpoints used:
//!   GET /api/v6/dex/balance/all-token-balances-by-address  (multi-chain in one call)
//!   GET /api/v6/dex/post-transaction/transactions-by-address
//!
//! NOTE on data-shape differences vs Covalent (these drive the field mapping below):
//!   * OKX `balance` is already a human-formatted decimal string; there is no
//!     reliable raw integer (`rawBalance` is empty for most chains) and no
//!     `decimals`. We surface `balance` as `balance_formatted` and best-effort
//!     `decimals` (18 default).
//!   * OKX has no token logos, no 24h price/quote history.
//!   * Transaction `amount` is a formatted decimal token amount (e.g. "0.02"),
//!     NOT raw wei. `TransactionItem::value` therefore carries a formatted
//!     decimal. Consumers that previously treated `value` as raw wei have been
//!     updated accordingly (see routes/policy.rs, services/ai_executor.rs).

use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;
use tracing::warn;

use crate::retry::{retry_with_backoff, RetryConfig};

type HmacSha256 = Hmac<Sha256>;

const BASE_URL: &str = "https://web3.okx.com";

/// OKX REST API credentials. `api_key`, `secret_key` and `passphrase` are
/// required; `project_id` (sent as `OK-ACCESS-PROJECT`) is optional and only
/// needed for certain WaaS project-scoped endpoints.
#[derive(Debug, Clone)]
pub struct OkxCredentials {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
    pub project_id: Option<String>,
}

impl OkxCredentials {
    /// Load from environment. Returns `None` unless the three required vars
    /// (`OKX_API_KEY`, `OKX_SECRET_KEY`, `OKX_PASSPHRASE`) are set and non-empty.
    /// `OKX_PROJECT_ID` is optional.
    pub fn from_env() -> Option<Self> {
        let api_key = non_empty_env("OKX_API_KEY")?;
        let secret_key = non_empty_env("OKX_SECRET_KEY")?;
        let passphrase = non_empty_env("OKX_PASSPHRASE")?;
        let project_id = non_empty_env("OKX_PROJECT_ID");
        Some(Self {
            api_key,
            secret_key,
            passphrase,
            project_id,
        })
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

// Signature must be `&String` to satisfy `retry_with_backoff`'s
// `ShouldRetryFn: FnMut(&E)` bound where `E = String`.
#[allow(clippy::ptr_arg)]
fn is_retryable_okx_error(err: &String) -> bool {
    err.contains("request failed")
        || err.contains("timed out")
        || err.contains("connection")
        || err.contains("502")
        || err.contains("503")
        || err.contains("429")
        || err.contains("50001") // OKX: service temporarily unavailable
}

// ─── Chain mapping ───────────────────────────────────────────────────────────
//
// OKX `chainIndex` is the decimal EVM chain id rendered as a string, so the
// mapping is the identity (chain_id.to_string()). We keep an explicit support
// list to mirror the previous Covalent behaviour of skipping unknown chains.

/// OKX `chainIndex` string for a chain id. `Some` only for chains we support.
pub fn chain_index(chain_id: u64) -> Option<String> {
    if is_supported(chain_id) {
        Some(chain_id.to_string())
    } else {
        None
    }
}

fn is_supported(chain_id: u64) -> bool {
    matches!(
        chain_id,
        // Mainnets
        1 | 8453 | 42161 | 10 | 56 | 137
        // Testnets
        | 11155111 | 84532 | 421614 | 11155420 | 80002
    )
}

pub fn chain_display_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "Ethereum",
        8453 => "Base",
        42161 => "Arbitrum One",
        10 => "Optimism",
        56 => "BNB Chain",
        137 => "Polygon",
        11155111 => "Ethereum Sepolia",
        84532 => "Base Sepolia",
        421614 => "Arbitrum Sepolia",
        11155420 => "Optimism Sepolia",
        80002 => "Polygon Amoy",
        _ => "Unknown",
    }
}

pub fn native_symbol(chain_id: u64) -> &'static str {
    match chain_id {
        137 | 80002 => "POL",
        56 => "BNB",
        _ => "ETH",
    }
}

/// TrustWallet assets CDN slug for a chain's native-coin / token logos. `None`
/// for chains TrustWallet doesn't host (the frontend then shows its fallback icon).
fn trustwallet_chain_slug(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 | 11155111 => Some("ethereum"),
        8453 | 84532 => Some("base"),
        42161 | 421614 => Some("arbitrum"),
        10 | 11155420 => Some("optimism"),
        56 => Some("smartchain"),
        137 | 80002 => Some("polygon"),
        _ => None,
    }
}

/// Best-effort token logo URL via the TrustWallet assets CDN. Native coins use
/// the chain's `info/logo.png`; ERC-20s use the EIP-55 checksum-less contract
/// path (CDN tolerates lowercase). Returns `None` for unsupported chains — the
/// mobile UI renders a symbol fallback when the URL is absent or 404s.
fn token_logo_url(chain_id: u64, contract: &str, is_native: bool) -> Option<String> {
    let slug = trustwallet_chain_slug(chain_id)?;
    const CDN: &str = "https://raw.githubusercontent.com/trustwallet/assets/master/blockchains";
    if is_native {
        Some(format!("{}/{}/info/logo.png", CDN, slug))
    } else if !contract.is_empty() {
        Some(format!("{}/{}/assets/{}/logo.png", CDN, slug, contract))
    } else {
        None
    }
}

// ─── Public output types (kept API-compatible with the former covalent module) ─

/// All token balance fields needed for display and business logic.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenBalance {
    pub symbol: String,
    pub name: String,
    pub balance: String,
    pub balance_formatted: String,
    pub balance_24h: Option<String>,
    pub usd: String,
    pub usd_24h: Option<String>,
    pub quote_rate: Option<f64>,
    pub quote_rate_24h: Option<f64>,
    pub pretty_quote: Option<String>,
    pub contract_address: Option<String>,
    pub decimals: u32,
    pub native_token: bool,
    pub is_spam: bool,
    pub token_type: Option<String>,
    pub logo_url: Option<String>,
    pub chain_logo_url: Option<String>,
    pub chain_id: Option<u64>,
    pub chain_name: Option<String>,
    pub last_transferred_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionItem {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    /// Raw integer token amount in the token's base unit (wei for 18-decimals).
    /// OKX returns a formatted decimal `amount`; we convert it back to raw here so
    /// `value` keeps the same wei semantics the former Covalent client produced
    /// (consumers parse it as an integer — see routes/policy.rs, the mobile UI).
    pub value: String,
    pub timestamp: String,
    pub status: String,
    pub gas_used: u64,
    pub token_symbol: String,
    pub value_quote: f64,
    pub chain_id: u64,
    pub chain_name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub token_transfers: Vec<TokenTransfer>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenTransfer {
    pub from: String,
    pub to: String,
    pub value: String,
    pub token_symbol: String,
    pub token_address: String,
    pub decimals: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChainBalance {
    pub chain_id: u64,
    pub chain_name: String,
    pub tokens: Vec<TokenBalance>,
    pub total_usd: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AllChainsBalance {
    pub address: String,
    pub chains: Vec<ChainBalance>,
    pub total_usd: String,
}

/// OKX `balance` is already formatted; this is retained for the few callers that
/// formatted a raw value. With OKX inputs the value is normally already decimal,
/// so we pass it through unchanged when it isn't an integer.
pub fn format_value(raw: &str, decimals: u32) -> String {
    // If the input already contains a decimal point it is pre-formatted.
    if raw.contains('.') {
        return raw.to_string();
    }
    format_units(raw, decimals)
}

fn format_units(raw: &str, decimals: u32) -> String {
    if raw == "0" || raw.is_empty() {
        return "0".into();
    }
    let value = match raw.parse::<u128>() {
        Ok(v) => v,
        Err(_) => return raw.to_string(),
    };
    if value == 0 {
        return "0".into();
    }
    let divisor = 10u128.pow(decimals);
    let whole = value / divisor;
    let frac = value % divisor;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let trimmed = frac_str.trim_end_matches('0');
        let display = if trimmed.len() > 6 {
            &trimmed[..6]
        } else {
            trimmed
        };
        format!("{}.{}", whole, display)
    }
}

// ─── Request signing ───────────────────────────────────────────────────────────

/// Current UTC time as an ISO-8601 millisecond timestamp (OKX `OK-ACCESS-TIMESTAMP`).
fn iso_timestamp() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// Build the signed headers for a GET request. `request_path` must include the
/// query string (e.g. `/api/v6/...?address=0x..&chains=1`).
fn signed_get(
    http: &Client,
    creds: &OkxCredentials,
    request_path: &str,
) -> Result<reqwest::RequestBuilder, String> {
    let timestamp = iso_timestamp();
    let prehash = format!("{}GET{}", timestamp, request_path);

    let mut mac = HmacSha256::new_from_slice(creds.secret_key.as_bytes())
        .map_err(|e| format!("OKX HMAC key error: {}", e))?;
    mac.update(prehash.as_bytes());
    let signature = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
    };

    let url = format!("{}{}", BASE_URL, request_path);
    let mut req = http
        .get(&url)
        .header("OK-ACCESS-KEY", &creds.api_key)
        .header("OK-ACCESS-SIGN", signature)
        .header("OK-ACCESS-TIMESTAMP", timestamp)
        .header("OK-ACCESS-PASSPHRASE", &creds.passphrase)
        .header("Content-Type", "application/json");

    // OK-ACCESS-PROJECT is only required for project-scoped WaaS endpoints.
    if let Some(project_id) = &creds.project_id {
        req = req.header("OK-ACCESS-PROJECT", project_id);
    }
    Ok(req)
}

// ─── Balances ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OkxEnvelope<T> {
    code: String,
    msg: Option<String>,
    data: Option<Vec<T>>,
}

#[derive(Debug, Deserialize)]
struct BalanceData {
    #[serde(default, rename = "tokenAssets")]
    token_assets: Vec<OkxTokenAsset>,
}

#[derive(Debug, Deserialize)]
struct OkxTokenAsset {
    #[serde(rename = "chainIndex")]
    chain_index: Option<String>,
    #[serde(rename = "tokenContractAddress")]
    token_contract_address: Option<String>,
    symbol: Option<String>,
    balance: Option<String>,
    #[serde(rename = "rawBalance")]
    raw_balance: Option<String>,
    #[serde(rename = "tokenPrice")]
    token_price: Option<String>,
    #[serde(rename = "isRiskToken")]
    is_risk_token: Option<bool>,
}

/// Map a single OKX token asset to the public `TokenBalance`. `default_chain_id`
/// is used when the asset omits `chainIndex`.
fn map_token_asset(asset: OkxTokenAsset, default_chain_id: u64) -> Option<TokenBalance> {
    let chain_id = asset
        .chain_index
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default_chain_id);

    let contract = asset.token_contract_address.unwrap_or_default();
    let is_native = contract.is_empty();
    let balance = asset.balance.unwrap_or_else(|| "0".into());

    let is_zero = balance == "0" || balance.is_empty();
    if is_zero && !is_native {
        return None;
    }

    let price = asset
        .token_price
        .as_deref()
        .and_then(|p| p.parse::<f64>().ok());
    let balance_num = balance.parse::<f64>().unwrap_or(0.0);
    let usd_value = price.map(|p| balance_num * p).unwrap_or(0.0);

    let symbol = asset.symbol.filter(|s| !s.is_empty()).unwrap_or_else(|| {
        if is_native {
            native_symbol(chain_id).into()
        } else {
            "???".into()
        }
    });

    let logo_url = token_logo_url(chain_id, &contract, is_native);

    Some(TokenBalance {
        // OKX returns no decimals; infer by symbol (stablecoins=6, else 18) so the
        // token-info UI shows a sane value. Balances themselves are pre-formatted.
        decimals: decimals_for_symbol(&symbol),
        name: symbol.clone(),
        symbol,
        // OKX has no reliable raw integer; surface rawBalance when present.
        balance: asset
            .raw_balance
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| balance.clone()),
        balance_formatted: balance,
        balance_24h: None,
        usd: format!("{:.2}", usd_value),
        usd_24h: None,
        quote_rate: price,
        quote_rate_24h: None,
        pretty_quote: None,
        contract_address: if is_native { None } else { Some(contract) },
        native_token: is_native,
        is_spam: asset.is_risk_token.unwrap_or(false),
        token_type: None,
        logo_url,
        chain_logo_url: None,
        chain_id: Some(chain_id),
        chain_name: Some(chain_display_name(chain_id).to_string()),
        last_transferred_at: None,
    })
}

async fn fetch_balances_raw(
    http: &Client,
    creds: &OkxCredentials,
    address: &str,
    chains_param: &str,
) -> Result<Vec<OkxTokenAsset>, String> {
    let request_path = format!(
        "/api/v6/dex/balance/all-token-balances-by-address?address={}&chains={}&excludeRiskToken=0",
        address, chains_param
    );
    tracing::info!(
        "[OKX] get_balances address={} chains={}",
        address,
        chains_param
    );

    let http = http.clone();
    let creds = creds.clone();
    let path = request_path.clone();

    let body: OkxEnvelope<BalanceData> = retry_with_backoff(
        RetryConfig::conservative(),
        || {
            let http = http.clone();
            let creds = creds.clone();
            let path = path.clone();
            async move {
                let resp = signed_get(&http, &creds, &path)?
                    .send()
                    .await
                    .map_err(|e| format!("OKX balance request failed: {}", e))?;

                let status = resp.status();
                if !status.is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    tracing::error!("[OKX] balance HTTP {} body: {}", status, text);
                    return Err(format!("OKX balance API returned {}", status));
                }

                let body: OkxEnvelope<BalanceData> = resp
                    .json()
                    .await
                    .map_err(|e| format!("OKX balance parse error: {}", e))?;

                if body.code != "0" {
                    return Err(format!(
                        "OKX balance error {}: {}",
                        body.code,
                        body.msg.clone().unwrap_or_default()
                    ));
                }
                Ok(body)
            }
        },
        is_retryable_okx_error,
        "okx_get_balances",
    )
    .await?;

    Ok(body
        .data
        .unwrap_or_default()
        .into_iter()
        .flat_map(|d| d.token_assets)
        .collect())
}

/// Single-chain balances.
pub async fn get_balances(
    http: &Client,
    creds: &OkxCredentials,
    address: &str,
    chain_id: u64,
) -> Result<Vec<TokenBalance>, String> {
    let idx = chain_index(chain_id).ok_or_else(|| format!("Unsupported chain_id: {}", chain_id))?;
    let assets = fetch_balances_raw(http, creds, address, &idx).await?;

    let mut tokens: Vec<TokenBalance> = assets
        .into_iter()
        .filter_map(|a| map_token_asset(a, chain_id))
        .collect();

    tokens.sort_by_key(|t| !t.native_token);
    Ok(tokens)
}

/// Cross-chain balances in a single OKX request (`chains` is comma-separated).
pub async fn get_all_chain_balances(
    http: &Client,
    creds: &OkxCredentials,
    address: &str,
    chain_ids: &[u64],
) -> Result<AllChainsBalance, String> {
    use std::collections::HashMap;

    let supported: Vec<u64> = chain_ids
        .iter()
        .copied()
        .filter(|&id| is_supported(id))
        .collect();
    if supported.is_empty() {
        return Ok(AllChainsBalance {
            address: address.to_string(),
            chains: Vec::new(),
            total_usd: "0.00".into(),
        });
    }

    let chains_param = supported
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let assets = fetch_balances_raw(http, creds, address, &chains_param).await?;

    // Group by chain id (asset carries its own chainIndex in multi-chain mode).
    let mut chain_map: HashMap<u64, Vec<TokenBalance>> = HashMap::new();
    for asset in assets {
        let default_chain = supported.first().copied().unwrap_or(1);
        if let Some(token) = map_token_asset(asset, default_chain) {
            let cid = token.chain_id.unwrap_or(default_chain);
            chain_map.entry(cid).or_default().push(token);
        }
    }

    let mut chains: Vec<ChainBalance> = Vec::new();
    let mut grand_total_usd = 0.0;

    for (chain_id, mut tokens) in chain_map {
        tokens.sort_by_key(|t| !t.native_token);
        let chain_total: f64 = tokens
            .iter()
            .filter_map(|t| t.usd.parse::<f64>().ok())
            .sum();
        grand_total_usd += chain_total;

        chains.push(ChainBalance {
            chain_id,
            chain_name: chain_display_name(chain_id).to_string(),
            tokens,
            total_usd: format!("{:.2}", chain_total),
        });
    }

    chains.sort_by_key(|c| c.chain_id);

    Ok(AllChainsBalance {
        address: address.to_string(),
        chains,
        total_usd: format!("{:.2}", grand_total_usd),
    })
}

// ─── Transaction history ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TxData {
    // OKX returns `"transactions": null` for addresses with no history (not an
    // empty array), and #[serde(default)] does NOT cover an explicit null — it
    // only covers a missing field. Deserialize as Option and coalesce null→[].
    #[serde(default, deserialize_with = "null_to_empty_vec")]
    transactions: Vec<OkxTransaction>,
}

/// Deserialize a field that may be `null` into an empty Vec.
fn null_to_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
struct OkxTransaction {
    #[serde(rename = "chainIndex")]
    chain_index: Option<String>,
    #[serde(rename = "txHash")]
    tx_hash: Option<String>,
    #[serde(rename = "txTime")]
    tx_time: Option<String>,
    from: Option<Vec<OkxTxParty>>,
    to: Option<Vec<OkxTxParty>>,
    amount: Option<String>,
    symbol: Option<String>,
    #[serde(rename = "txStatus")]
    tx_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OkxTxParty {
    address: Option<String>,
}

/// Convert OKX `txTime` (Unix milliseconds as a string) to an RFC-3339 timestamp,
/// matching the format the previous client produced (callers parse RFC-3339).
fn ms_to_rfc3339(ms: &str) -> String {
    ms.parse::<i64>()
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

/// Best-effort token decimals by symbol. OKX balance/tx responses omit decimals;
/// stablecoins use 6, everything else defaults to 18. Mirrors the assumption the
/// mobile client and ai_executor already make for tx-value formatting.
fn decimals_for_symbol(symbol: &str) -> u32 {
    match symbol.to_uppercase().as_str() {
        "USDC" | "USDT" => 6,
        _ => 18,
    }
}

/// Convert a formatted decimal amount (e.g. "0.02", "1.5", "1") to a raw integer
/// string in the token's base unit using `decimals`. OKX `amount` is always a
/// formatted token amount, so "1" means 1 whole token (10^decimals base units),
/// never 1 base unit. Malformed input yields "0".
fn decimal_to_raw_string(amount: &str, decimals: u32) -> String {
    let amount = amount.trim();
    if amount.is_empty() {
        return "0".into();
    }
    let (whole_str, frac_str) = match amount.split_once('.') {
        Some((w, f)) => (w, f),
        None => (amount, ""),
    };

    let whole = match whole_str.parse::<u128>() {
        Ok(v) => v,
        Err(_) => return "0".into(),
    };
    let mut frac_digits: String = frac_str.chars().take(decimals as usize).collect();
    while (frac_digits.len() as u32) < decimals {
        frac_digits.push('0');
    }
    let frac = frac_digits.parse::<u128>().unwrap_or(0);

    let scale = 10u128.pow(decimals);
    match whole.checked_mul(scale).and_then(|w| w.checked_add(frac)) {
        Some(v) => v.to_string(),
        None => "0".into(),
    }
}

fn map_transaction(tx: OkxTransaction, default_chain_id: u64) -> TransactionItem {
    let chain_id = tx
        .chain_index
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default_chain_id);

    let from = tx
        .from
        .as_ref()
        .and_then(|v| v.first())
        .and_then(|p| p.address.clone())
        .unwrap_or_default();
    let to = tx
        .to
        .as_ref()
        .and_then(|v| v.first())
        .and_then(|p| p.address.clone())
        .unwrap_or_default();

    let status = match tx.tx_status.as_deref() {
        Some("success") => "confirmed".to_string(),
        Some("pending") => "pending".to_string(),
        _ => "failed".to_string(),
    };

    let symbol = tx
        .symbol
        .unwrap_or_else(|| native_symbol(chain_id).to_string());
    let decimals = decimals_for_symbol(&symbol);
    let value = tx
        .amount
        .as_deref()
        .map(|a| decimal_to_raw_string(a, decimals))
        .unwrap_or_else(|| "0".into());

    TransactionItem {
        tx_hash: tx.tx_hash.unwrap_or_default(),
        from,
        to,
        value,
        timestamp: tx.tx_time.as_deref().map(ms_to_rfc3339).unwrap_or_default(),
        status,
        gas_used: 0, // OKX history returns txFee (string), not gas units.
        token_symbol: symbol,
        value_quote: 0.0, // OKX history has no USD quote.
        chain_id,
        chain_name: chain_display_name(chain_id).to_string(),
        token_transfers: Vec::new(),
    }
}

async fn fetch_transactions_raw(
    http: &Client,
    creds: &OkxCredentials,
    address: &str,
    chains_param: &str,
    limit: u32,
) -> Result<Vec<OkxTransaction>, String> {
    let request_path = format!(
        "/api/v6/dex/post-transaction/transactions-by-address?address={}&chains={}&limit={}",
        address, chains_param, limit
    );
    tracing::info!(
        "[OKX] get_transactions address={} chains={}",
        address,
        chains_param
    );

    let http = http.clone();
    let creds = creds.clone();
    let path = request_path.clone();

    let body: OkxEnvelope<TxData> = retry_with_backoff(
        RetryConfig::conservative(),
        || {
            let http = http.clone();
            let creds = creds.clone();
            let path = path.clone();
            async move {
                let resp = signed_get(&http, &creds, &path)?
                    .send()
                    .await
                    .map_err(|e| format!("OKX tx request failed: {}", e))?;

                let status = resp.status();
                if !status.is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    tracing::error!("[OKX] tx HTTP {} body: {}", status, text);
                    return Err(format!("OKX tx API returned {}", status));
                }

                // Read the raw body first so a schema mismatch logs OKX's actual
                // JSON (the fields are all Option/defaulted, so a parse failure
                // means the *shape* differs — capture it to fix precisely).
                let text = resp
                    .text()
                    .await
                    .map_err(|e| format!("OKX tx read error: {}", e))?;
                let body: OkxEnvelope<TxData> = serde_json::from_str(&text).map_err(|e| {
                    tracing::error!("[OKX] tx parse error: {} — raw body: {}", e, text);
                    format!("OKX tx parse error: {}", e)
                })?;

                if body.code != "0" {
                    return Err(format!(
                        "OKX tx error {}: {}",
                        body.code,
                        body.msg.clone().unwrap_or_default()
                    ));
                }
                Ok(body)
            }
        },
        is_retryable_okx_error,
        "okx_get_transactions",
    )
    .await?;

    Ok(body
        .data
        .unwrap_or_default()
        .into_iter()
        .flat_map(|d| d.transactions)
        .collect())
}

/// Single-chain transaction history (most recent first).
pub async fn get_transactions(
    http: &Client,
    creds: &OkxCredentials,
    address: &str,
    chain_id: u64,
) -> Result<Vec<TransactionItem>, String> {
    let idx = chain_index(chain_id).ok_or_else(|| format!("Unsupported chain_id: {}", chain_id))?;
    // Single-chain queries support up to 100 records.
    let txs = fetch_transactions_raw(http, creds, address, &idx, 100).await?;
    Ok(txs
        .into_iter()
        .map(|t| map_transaction(t, chain_id))
        .collect())
}

/// Cross-chain transaction history in a single OKX request.
pub async fn get_all_chain_transactions(
    http: &Client,
    creds: &OkxCredentials,
    address: &str,
    chain_ids: &[u64],
) -> Result<Vec<TransactionItem>, String> {
    let supported: Vec<u64> = chain_ids
        .iter()
        .copied()
        .filter(|&id| is_supported(id))
        .collect();
    if supported.is_empty() {
        return Ok(Vec::new());
    }

    let chains_param = supported
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");

    // Multi-chain queries are capped at 20 records by OKX.
    let txs = match fetch_transactions_raw(http, creds, address, &chains_param, 20).await {
        Ok(txs) => txs,
        Err(e) => {
            warn!("OKX multi-chain tx query failed: {}", e);
            return Err(e);
        }
    };

    let default_chain = supported.first().copied().unwrap_or(1);
    let mut all: Vec<TransactionItem> = txs
        .into_iter()
        .map(|t| map_transaction(t, default_chain))
        .collect();

    all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_index_is_decimal_id() {
        assert_eq!(chain_index(1).as_deref(), Some("1"));
        assert_eq!(chain_index(8453).as_deref(), Some("8453"));
        assert_eq!(chain_index(137).as_deref(), Some("137"));
        assert_eq!(chain_index(999999), None); // unsupported
    }

    #[test]
    fn decimal_to_raw_handles_native_and_stable() {
        // 18-decimals native
        assert_eq!(decimal_to_raw_string("0.02", 18), "20000000000000000");
        assert_eq!(decimal_to_raw_string("1", 18), "1000000000000000000");
        assert_eq!(decimal_to_raw_string("1.5", 18), "1500000000000000000");
        // 6-decimals stablecoin
        assert_eq!(decimal_to_raw_string("69564", 6), "69564000000");
        assert_eq!(decimal_to_raw_string("0.5", 6), "500000");
        // edge cases
        assert_eq!(decimal_to_raw_string("0", 18), "0");
        assert_eq!(decimal_to_raw_string("", 18), "0");
        // integer input is a whole-token amount, scaled by decimals
        assert_eq!(
            decimal_to_raw_string("12345", 18),
            "12345000000000000000000"
        );
        // excess fractional digits truncated, not rounded
        assert_eq!(
            decimal_to_raw_string("1.0000000000000000009", 18),
            "1000000000000000000"
        );
    }

    #[test]
    fn decimals_inferred_by_symbol() {
        assert_eq!(decimals_for_symbol("USDC"), 6);
        assert_eq!(decimals_for_symbol("usdt"), 6);
        assert_eq!(decimals_for_symbol("ETH"), 18);
        assert_eq!(decimals_for_symbol("WBTC"), 18);
    }

    #[test]
    fn ms_to_rfc3339_parses_okx_txtime() {
        // OKX returns Unix ms as a string, e.g. "1724213411000".
        let out = ms_to_rfc3339("1724213411000");
        assert!(out.starts_with("2024-08-21T"), "got {}", out);
        assert_eq!(ms_to_rfc3339("bad"), "");
    }

    #[test]
    fn parse_balance_envelope_from_okx_sample() {
        // Trimmed real response shape from OKX all-token-balances-by-address.
        let raw = r#"{
            "code": "0",
            "msg": "success",
            "data": [{
                "tokenAssets": [
                    {"chainIndex":"1","tokenContractAddress":"","symbol":"ETH","balance":"8.135546539084933","tokenPrice":"3638.63","isRiskToken":false,"rawBalance":"","address":"0xabc"},
                    {"chainIndex":"1","tokenContractAddress":"0x4c9edd5852cd905f086c759e8383e09bff1e68b3","symbol":"USDe","balance":"69564","tokenPrice":"0.99977","isRiskToken":false,"rawBalance":"","address":"0xabc"}
                ]
            }]
        }"#;
        let env: OkxEnvelope<BalanceData> = serde_json::from_str(raw).unwrap();
        assert_eq!(env.code, "0");
        let assets: Vec<_> = env
            .data
            .unwrap()
            .into_iter()
            .flat_map(|d| d.token_assets)
            .collect();
        assert_eq!(assets.len(), 2);

        let eth = map_token_asset(
            OkxTokenAsset {
                chain_index: Some("1".into()),
                token_contract_address: Some("".into()),
                symbol: Some("ETH".into()),
                balance: Some("8.135546539084933".into()),
                raw_balance: Some("".into()),
                token_price: Some("3638.63".into()),
                is_risk_token: Some(false),
            },
            1,
        )
        .unwrap();
        assert!(eth.native_token);
        assert_eq!(eth.symbol, "ETH");
        assert_eq!(eth.contract_address, None);
        // native logo points at the chain's TrustWallet info logo
        assert_eq!(
            eth.logo_url.as_deref(),
            Some("https://raw.githubusercontent.com/trustwallet/assets/master/blockchains/ethereum/info/logo.png")
        );
        // usd = balance * price ≈ 8.1355 * 3638.63
        let usd: f64 = eth.usd.parse().unwrap();
        assert!((usd - 29602.0).abs() < 50.0, "usd was {}", eth.usd);
    }

    #[test]
    fn token_logo_url_native_and_erc20() {
        assert_eq!(
            token_logo_url(1, "", true).as_deref(),
            Some("https://raw.githubusercontent.com/trustwallet/assets/master/blockchains/ethereum/info/logo.png")
        );
        assert_eq!(
            token_logo_url(8453, "0xabc", false).as_deref(),
            Some("https://raw.githubusercontent.com/trustwallet/assets/master/blockchains/base/assets/0xabc/logo.png")
        );
        // unsupported chain -> None (frontend falls back to symbol icon)
        assert_eq!(token_logo_url(999999, "0xabc", false), None);
    }

    #[test]
    fn erc20_balance_infers_stablecoin_decimals() {
        let usde = map_token_asset(
            OkxTokenAsset {
                chain_index: Some("1".into()),
                token_contract_address: Some("0x4c9edd5852cd905f086c759e8383e09bff1e68b3".into()),
                symbol: Some("USDT".into()),
                balance: Some("69564".into()),
                raw_balance: Some("".into()),
                token_price: Some("0.99977".into()),
                is_risk_token: Some(false),
            },
            1,
        )
        .unwrap();
        assert_eq!(usde.decimals, 6); // stablecoin
        assert!(!usde.native_token);
    }

    #[test]
    fn parse_tx_envelope_from_okx_sample() {
        // Trimmed real response from OKX transactions-by-address.
        let raw = r#"{
            "code": "0",
            "msg": "success",
            "data": [{
                "cursor": "1706197403",
                "transactions": [{
                    "chainIndex":"1",
                    "txHash":"0x963767",
                    "methodId":"",
                    "nonce":"",
                    "txTime":"1724213411000",
                    "from":[{"address":"0xfromaddr","amount":""}],
                    "to":[{"address":"0xtoaddr","amount":""}],
                    "tokenContractAddress":"0xe13c851c",
                    "amount":"1.5",
                    "symbol":"ETH",
                    "txFee":"",
                    "txStatus":"success",
                    "hitBlacklist":false,
                    "itype":"2"
                }]
            }]
        }"#;
        let env: OkxEnvelope<TxData> = serde_json::from_str(raw).unwrap();
        let txs: Vec<_> = env
            .data
            .unwrap()
            .into_iter()
            .flat_map(|d| d.transactions)
            .collect();
        assert_eq!(txs.len(), 1);

        let item = map_transaction(txs.into_iter().next().unwrap(), 1);
        assert_eq!(item.from, "0xfromaddr");
        assert_eq!(item.to, "0xtoaddr");
        assert_eq!(item.status, "confirmed"); // "success" -> "confirmed"
        assert_eq!(item.token_symbol, "ETH");
        // amount "1.5" ETH -> raw wei
        assert_eq!(item.value, "1500000000000000000");
        assert!(item.timestamp.starts_with("2024-08-21T"));
    }
}
