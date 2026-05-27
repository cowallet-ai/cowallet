//! Swap routes for token exchange via Bridgers cross-chain aggregator.
//!
//! POST /api/v1/swap/quote       — Get swap price quote
//! POST /api/v1/swap/build       — Build swap transaction calldata
//! GET  /api/v1/swap/tokens      — List supported tokens
//! POST /api/v1/swap/order       — Upload order tx hash
//! GET  /api/v1/swap/order/{id}  — Get order status

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::errors::{ApiError, Result};
use crate::services::bridgers;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/quote", post(get_swap_quote))
        .route("/build", post(build_swap_tx))
        .route("/tokens", get(get_tokens))
        .route("/order", post(upload_order))
        .route("/order/{id}", get(get_order_status))
}

// ─── Request / Response types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct QuoteRequest {
    from_chain_id: u64,
    to_chain_id: Option<u64>,
    sell_token: String,
    buy_token: String,
    sell_amount: String,
    taker_address: Option<String>,
}

#[derive(Debug, Serialize)]
struct QuoteResponse {
    sell_token: String,
    buy_token: String,
    sell_amount: String,
    buy_amount: String,
    buy_amount_min: String,
    buy_amount_formatted: String,
    price: String,
    estimated_gas: String,
    fee_rate: String,
    chain_fee: String,
    from_chain_id: u64,
    to_chain_id: u64,
    contract_address: String,
    deposit_min: Option<String>,
    deposit_max: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BuildRequest {
    from_chain_id: u64,
    to_chain_id: Option<u64>,
    sell_token: String,
    buy_token: String,
    sell_amount: String,
    slippage: Option<f64>,
    taker_address: String,
    to_address: Option<String>,
}

#[derive(Debug, Serialize)]
struct BuildResponse {
    to: String,
    data: String,
    value: String,
    gas_estimate: String,
    sell_token: String,
    buy_token: String,
    sell_amount: String,
    buy_amount: String,
    price: String,
    allowance_target: Option<String>,
    from_chain_id: u64,
    to_chain_id: u64,
}

#[derive(Debug, Deserialize)]
struct TokensQuery {
    chain: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrderRequest {
    hash: String,
    from_chain_id: u64,
    to_chain_id: u64,
    sell_token: String,
    buy_token: String,
    sell_amount: String,
    buy_amount_min: String,
    from_address: String,
    to_address: String,
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// POST /swap/quote
async fn get_swap_quote(
    State(state): State<AppState>,
    Json(req): Json<QuoteRequest>,
) -> Result<Json<QuoteResponse>> {
    let from_chain_id = req.from_chain_id;
    let to_chain_id = req.to_chain_id.unwrap_or(from_chain_id);

    let sell_addr = resolve_token_address(&req.sell_token, from_chain_id)?;
    let buy_addr = resolve_token_address(&req.buy_token, to_chain_id)?;

    let sell_decimals = bridgers::token_decimals(&req.sell_token);
    let raw_amount = bridgers::amount_to_raw(&req.sell_amount, sell_decimals)
        .map_err(|e| ApiError::bad_request(&e))?;

    let sell_code = token_code(&req.sell_token);
    let buy_code = token_code(&req.buy_token);

    let quote = bridgers::get_quote(
        &state.http,
        &state.bridgers_source_flag,
        from_chain_id,
        to_chain_id,
        &sell_addr,
        &buy_addr,
        &raw_amount,
        sell_code,
        buy_code,
        req.taker_address.as_deref(),
    )
    .await
    .map_err(|e| ApiError::external_service(&e))?;

    let buy_decimals = bridgers::token_decimals(&req.buy_token);
    let buy_formatted = bridgers::raw_to_amount(&quote.buy_amount, buy_decimals);

    Ok(Json(QuoteResponse {
        sell_token: req.sell_token,
        buy_token: req.buy_token,
        sell_amount: req.sell_amount,
        buy_amount: quote.buy_amount.clone(),
        buy_amount_min: quote.buy_amount_min,
        buy_amount_formatted: buy_formatted,
        price: quote.price,
        estimated_gas: quote.estimated_gas,
        fee_rate: quote.fee_rate,
        chain_fee: quote.chain_fee,
        from_chain_id,
        to_chain_id,
        contract_address: quote.contract_address,
        deposit_min: quote.deposit_min,
        deposit_max: quote.deposit_max,
    }))
}

/// POST /swap/build
async fn build_swap_tx(
    State(state): State<AppState>,
    Json(req): Json<BuildRequest>,
) -> Result<Json<BuildResponse>> {
    let from_chain_id = req.from_chain_id;
    let to_chain_id = req.to_chain_id.unwrap_or(from_chain_id);

    let sell_addr = resolve_token_address(&req.sell_token, from_chain_id)?;
    let buy_addr = resolve_token_address(&req.buy_token, to_chain_id)?;

    let sell_decimals = bridgers::token_decimals(&req.sell_token);
    let raw_amount = bridgers::amount_to_raw(&req.sell_amount, sell_decimals)
        .map_err(|e| ApiError::bad_request(&e))?;

    // Bridgers expects slippage as percentage (e.g. 0.5 means 0.5%)
    let slippage = req.slippage.unwrap_or(0.5);

    if !req.taker_address.starts_with("0x") || req.taker_address.len() != 42 {
        return Err(ApiError::bad_request("Invalid taker_address format"));
    }

    let to_addr = req.to_address.as_deref().unwrap_or(&req.taker_address);
    let sell_code = token_code(&req.sell_token);
    let buy_code = token_code(&req.buy_token);

    // Get quote first to determine amount_out_min
    let quote = bridgers::get_quote(
        &state.http,
        &state.bridgers_source_flag,
        from_chain_id,
        to_chain_id,
        &sell_addr,
        &buy_addr,
        &raw_amount,
        sell_code,
        buy_code,
        Some(&req.taker_address),
    )
    .await
    .map_err(|e| ApiError::external_service(&e))?;

    let tx = bridgers::build_swap_tx(
        &state.http,
        &state.bridgers_source_flag,
        from_chain_id,
        to_chain_id,
        &sell_addr,
        &buy_addr,
        &req.taker_address,
        to_addr,
        &raw_amount,
        &quote.buy_amount_min,
        sell_code,
        buy_code,
        slippage,
    )
    .await
    .map_err(|e| ApiError::external_service(&e))?;

    Ok(Json(BuildResponse {
        to: tx.to,
        data: tx.data,
        value: tx.value,
        gas_estimate: tx.gas_estimate,
        sell_token: req.sell_token,
        buy_token: req.buy_token,
        sell_amount: req.sell_amount,
        buy_amount: tx.buy_amount,
        price: tx.price,
        allowance_target: tx.allowance_target,
        from_chain_id,
        to_chain_id,
    }))
}

/// GET /swap/tokens?chain=ETH
async fn get_tokens(
    State(state): State<AppState>,
    Query(query): Query<TokensQuery>,
) -> Result<Json<Vec<bridgers::BridgersToken>>> {
    let tokens = bridgers::get_tokens(&state.http, query.chain.as_deref())
        .await
        .map_err(|e| ApiError::external_service(&e))?;
    Ok(Json(tokens))
}

/// POST /swap/order
async fn upload_order(
    State(state): State<AppState>,
    Json(req): Json<OrderRequest>,
) -> Result<Json<serde_json::Value>> {
    let sell_addr = resolve_token_address(&req.sell_token, req.from_chain_id)?;
    let buy_addr = resolve_token_address(&req.buy_token, req.to_chain_id)?;
    let sell_code = token_code(&req.sell_token);
    let buy_code = token_code(&req.buy_token);

    let sell_decimals = bridgers::token_decimals(&req.sell_token);
    let raw_amount = bridgers::amount_to_raw(&req.sell_amount, sell_decimals)
        .map_err(|e| ApiError::bad_request(&e))?;
    let buy_decimals = bridgers::token_decimals(&req.buy_token);
    let raw_min = bridgers::amount_to_raw(&req.buy_amount_min, buy_decimals)
        .map_err(|e| ApiError::bad_request(&e))?;

    let order_id = bridgers::upload_order_hash(
        &state.http,
        &state.bridgers_source_flag,
        &req.hash,
        &sell_addr,
        &buy_addr,
        &req.from_address,
        &req.to_address,
        req.from_chain_id,
        req.to_chain_id,
        &raw_amount,
        &raw_min,
        sell_code,
        buy_code,
    )
    .await
    .map_err(|e| ApiError::external_service(&e))?;

    Ok(Json(serde_json::json!({ "order_id": order_id })))
}

/// GET /swap/order/{id}
async fn get_order_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<bridgers::OrderStatus>> {
    let status = bridgers::get_order_status(&state.http, &id)
        .await
        .map_err(|e| ApiError::external_service(&e))?;
    Ok(Json(status))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Resolve a token symbol or address to a contract address
fn resolve_token_address(token: &str, chain_id: u64) -> Result<String> {
    if token.starts_with("0x") && token.len() == 42 {
        return Ok(token.to_string());
    }
    bridgers::token_address(token, chain_id)
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::bad_request(&format!(
            "Unknown token '{}' on chain {}. Provide a contract address instead.",
            token, chain_id
        )))
}

/// Get coin code from token string (symbol or address -> symbol)
fn token_code(token: &str) -> &str {
    if token.starts_with("0x") { "UNKNOWN" } else { token }
}
