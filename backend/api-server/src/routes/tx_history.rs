use axum::{
    Json, Router,
    extract::{Query, Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth::Claims;
use crate::services::okx;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/history", get(get_history))
        .route("/tx-history", get(get_onchain_history))
        .route("/all-history", get(get_all_chain_history))
        .route("/{hash}", get(get_transaction))
}

#[derive(Deserialize)]
struct HistoryQuery {
    address: String,
    chain_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct TransactionRecord {
    tx_hash: String,
    from: String,
    to: String,
    value: String,
    token_address: Option<String>,
    status: String,
    block_number: Option<i64>,
    timestamp: Option<String>,
    chain_id: i64,
}

#[derive(Serialize)]
struct HistoryResponse {
    transactions: Vec<TransactionRecord>,
    total: i64,
}

/// GET /api/v1/tx/history?address={addr}&chain_id={id}&limit=50&offset=0
///
/// Get paginated transaction history for an address
async fn get_history(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.require_db().map_err(|_| db_unavailable())?;

    // Parse address
    let address_str = q.address.strip_prefix("0x").unwrap_or(&q.address);
    let address_bytes = hex::decode(address_str)
        .map_err(|_| validation_error("invalid address hex"))?;

    if address_bytes.len() != 20 {
        return Err(validation_error("address must be 20 bytes"));
    }

    // Only the owner of the address may read its history (F-008).
    let user_id = parse_user_id(&claims.0)?;
    assert_address_owned(db, user_id, &address_bytes).await?;

    let limit = q.limit.unwrap_or(50).min(100).max(1);
    let offset = q.offset.unwrap_or(0).max(0);

    // Build query with optional chain_id filter
    let query = if let Some(chain_id) = q.chain_id {
        sqlx::query_as::<_, (Vec<u8>, Vec<u8>, Vec<u8>, String, Option<Vec<u8>>, String, Option<i64>, Option<chrono::DateTime<chrono::Utc>>, i64)>(
            r#"
            SELECT tx_hash, from_addr, to_addr, value, token_address, status, block_number, created_at, chain_id
            FROM transactions
            WHERE (from_addr = $1 OR to_addr = $1)
              AND chain_id = $2
            ORDER BY block_number DESC NULLS LAST, created_at DESC
            LIMIT $3 OFFSET $4
            "#
        )
        .bind(&address_bytes)
        .bind(chain_id)
        .bind(limit)
        .bind(offset)
    } else {
        sqlx::query_as::<_, (Vec<u8>, Vec<u8>, Vec<u8>, String, Option<Vec<u8>>, String, Option<i64>, Option<chrono::DateTime<chrono::Utc>>, i64)>(
            r#"
            SELECT tx_hash, from_addr, to_addr, value, token_address, status, block_number, created_at, chain_id
            FROM transactions
            WHERE from_addr = $1 OR to_addr = $1
            ORDER BY block_number DESC NULLS LAST, created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(&address_bytes)
        .bind(limit)
        .bind(offset)
    };

    let rows = query.fetch_all(db).await
        .map_err(|e| db_error(&e.to_string()))?;

    // Get total count for pagination
    let total_query = if let Some(chain_id) = q.chain_id {
        sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM transactions WHERE (from_addr = $1 OR to_addr = $1) AND chain_id = $2"
        )
        .bind(&address_bytes)
        .bind(chain_id)
    } else {
        sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM transactions WHERE from_addr = $1 OR to_addr = $1"
        )
        .bind(&address_bytes)
    };

    let total = total_query.fetch_one(db).await
        .map(|(count,)| count)
        .unwrap_or(0);

    let transactions = rows
        .into_iter()
        .map(|(tx_hash, from_addr, to_addr, value, token_address, status, block_number, created_at, chain_id)| {
            TransactionRecord {
                tx_hash: format!("0x{}", hex::encode(&tx_hash)),
                from: format!("0x{}", hex::encode(&from_addr)),
                to: format!("0x{}", hex::encode(&to_addr)),
                value,
                token_address: token_address.map(|addr| {
                    // Zero address means native ETH
                    if addr.iter().all(|&b| b == 0) {
                        "native".to_string()
                    } else {
                        format!("0x{}", hex::encode(&addr))
                    }
                }),
                status,
                block_number,
                timestamp: created_at.map(|t| t.to_rfc3339()),
                chain_id,
            }
        })
        .collect();

    Ok(Json(HistoryResponse {
        transactions,
        total,
    }))
}

#[derive(Serialize)]
struct TransactionDetail {
    tx_hash: String,
    from: String,
    to: String,
    value: String,
    token_address: Option<String>,
    status: String,
    block_number: Option<i64>,
    timestamp: Option<String>,
    chain_id: i64,
    gas_used: Option<i64>,
}

/// GET /api/v1/tx/{hash}
///
/// Get single transaction details by hash
async fn get_transaction(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Path(hash): Path<String>,
) -> Result<Json<TransactionDetail>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.require_db().map_err(|_| db_unavailable())?;
    let user_id = parse_user_id(&claims.0)?;

    // Parse transaction hash
    let hash_str = hash.strip_prefix("0x").unwrap_or(&hash);
    let hash_bytes = hex::decode(hash_str)
        .map_err(|_| validation_error("invalid transaction hash"))?;

    let row: (Vec<u8>, Vec<u8>, Vec<u8>, String, Option<Vec<u8>>, String, Option<i64>, Option<chrono::DateTime<chrono::Utc>>, i64, Option<i64>) =
        sqlx::query_as(
            r#"
            SELECT tx_hash, from_addr, to_addr, value, token_address, status, block_number, created_at, chain_id, gas_used
            FROM transactions
            WHERE tx_hash = $1
            LIMIT 1
            "#
        )
        .bind(&hash_bytes)
        .fetch_optional(db)
        .await
        .map_err(|e| db_error(&e.to_string()))?
        .ok_or_else(|| not_found("transaction not found"))?;

    let (tx_hash, from_addr, to_addr, value, token_address, status, block_number, created_at, chain_id, gas_used) = row;

    // The caller must own at least one side of the transaction (F-008). Return
    // NOT_FOUND rather than FORBIDDEN so tx existence isn't leaked by status code.
    let owns_from = assert_address_owned(db, user_id, &from_addr).await.is_ok();
    let owns_to = assert_address_owned(db, user_id, &to_addr).await.is_ok();
    if !owns_from && !owns_to {
        return Err(not_found("transaction not found"));
    }

    Ok(Json(TransactionDetail {
        tx_hash: format!("0x{}", hex::encode(&tx_hash)),
        from: format!("0x{}", hex::encode(&from_addr)),
        to: format!("0x{}", hex::encode(&to_addr)),
        value,
        token_address: token_address.map(|addr| {
            if addr.iter().all(|&b| b == 0) {
                "native".to_string()
            } else {
                format!("0x{}", hex::encode(&addr))
            }
        }),
        status,
        block_number,
        timestamp: created_at.map(|t| t.to_rfc3339()),
        chain_id,
        gas_used,
    }))
}

// ─── OKX-based on-chain tx history ───────────────────────────────────────────

#[derive(Deserialize)]
struct OnchainHistoryQuery {
    address: String,
    chain_id: Option<u64>,
}

#[derive(Serialize)]
struct OnchainHistoryResponse {
    transactions: Vec<OnchainTxInfo>,
    total: usize,
}

#[derive(Serialize)]
struct OnchainTxInfo {
    tx_hash: String,
    from: String,
    to: String,
    value: String,
    timestamp: String,
    status: String,
    gas_used: u64,
    token_symbol: String,
    value_quote: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    token_transfers: Vec<okx::TokenTransfer>,
}

/// GET /api/v1/tx/tx-history?address={addr}&chain_id={id}
///
/// Get on-chain transaction history via OKX Wallet API (no DB required)
async fn get_onchain_history(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Query(q): Query<OnchainHistoryQuery>,
) -> Result<Json<OnchainHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate address
    if !q.address.starts_with("0x") || q.address.len() != 42 {
        return Err(validation_error("invalid address format"));
    }

    // Only the owner may query this address's on-chain history (F-008).
    let db = state.require_db().map_err(|_| db_unavailable())?;
    let user_id = parse_user_id(&claims.0)?;
    let address_bytes = parse_address_bytes(&q.address)?;
    assert_address_owned(db, user_id, &address_bytes).await?;

    let chain_id = q.chain_id.ok_or_else(|| validation_error("chain_id is required"))?;

    let creds = state
        .okx_credentials
        .as_ref()
        .ok_or_else(|| (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "OKX Wallet API not configured".into(),
            }),
        ))?;

    let items = okx::get_transactions(&state.http, creds, &q.address, chain_id)
        .await
        .map_err(|e| (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: format!("Transaction history query failed: {}", e),
            }),
        ))?;

    let total = items.len();
    let transactions: Vec<OnchainTxInfo> = items
        .into_iter()
        .map(|item| OnchainTxInfo {
            tx_hash: item.tx_hash,
            from: item.from,
            to: item.to,
            value: item.value,
            timestamp: item.timestamp,
            status: item.status,
            gas_used: item.gas_used,
            token_symbol: item.token_symbol,
            value_quote: item.value_quote,
            token_transfers: item.token_transfers,
        })
        .collect();

    Ok(Json(OnchainHistoryResponse {
        transactions,
        total,
    }))
}

// ─── Multi-chain history endpoint ─────────────────────────────────────────────

#[derive(Deserialize)]
struct AllChainHistoryQuery {
    address: String,
    chains: Option<String>,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct AllChainHistoryResponse {
    transactions: Vec<AllChainTxInfo>,
    total: usize,
}

#[derive(Serialize)]
struct AllChainTxInfo {
    chain_id: u64,
    chain_name: String,
    tx_hash: String,
    from: String,
    to: String,
    value: String,
    timestamp: String,
    status: String,
    gas_used: u64,
    token_symbol: String,
    value_quote: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    token_transfers: Vec<okx::TokenTransfer>,
}

/// GET /api/v1/tx/all-history?address={addr}&chains={chain_ids}&limit={n}
///
/// Get transaction history across multiple chains in parallel
async fn get_all_chain_history(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Query(q): Query<AllChainHistoryQuery>,
) -> Result<Json<AllChainHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate address
    if !q.address.starts_with("0x") || q.address.len() != 42 {
        return Err(validation_error("invalid address format"));
    }

    // Only the owner may query this address's history across chains (F-008).
    let db = state.require_db().map_err(|_| db_unavailable())?;
    let user_id = parse_user_id(&claims.0)?;
    let address_bytes = parse_address_bytes(&q.address)?;
    assert_address_owned(db, user_id, &address_bytes).await?;

    // Parse chain IDs from comma-separated string
    let chain_ids: Vec<u64> = if let Some(chains_str) = q.chains {
        chains_str
            .split(',')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect()
    } else {
        // Default to all mainnets
        vec![1, 8453, 42161, 10, 56, 137]
    };

    if chain_ids.is_empty() {
        return Err(validation_error("no valid chain IDs provided"));
    }

    let creds = state
        .okx_credentials
        .as_ref()
        .ok_or_else(|| (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "OKX Wallet API not configured".into(),
            }),
        ))?;

    let items = okx::get_all_chain_transactions(&state.http, creds, &q.address, &chain_ids)
        .await
        .map_err(|e| (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: format!("Multi-chain transaction query failed: {}", e),
            }),
        ))?;

    // Apply limit after merging
    let limit = q.limit.unwrap_or(50).min(200);
    let limited_items: Vec<_> = items.into_iter().take(limit).collect();
    let total = limited_items.len();

    let transactions: Vec<AllChainTxInfo> = limited_items
        .into_iter()
        .map(|item| AllChainTxInfo {
            chain_id: item.chain_id,
            chain_name: item.chain_name,
            tx_hash: item.tx_hash,
            from: item.from,
            to: item.to,
            value: item.value,
            timestamp: item.timestamp,
            status: item.status,
            gas_used: item.gas_used,
            token_symbol: item.token_symbol,
            value_quote: item.value_quote,
            token_transfers: item.token_transfers,
        })
        .collect();

    Ok(Json(AllChainHistoryResponse {
        transactions,
        total,
    }))
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn db_unavailable() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "database not available".into(),
        }),
    )
}

fn db_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

fn validation_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

fn not_found(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

fn forbidden(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

/// Parse a user_id from JWT claims.
fn parse_user_id(claims: &Claims) -> Result<uuid::Uuid, (StatusCode, Json<ErrorResponse>)> {
    claims.sub.parse().map_err(|_| validation_error("invalid user id in token"))
}

/// Verify that `address_bytes` (20-byte EVM address) belongs to the authenticated
/// user. Prevents querying arbitrary addresses' history/details (F-008).
async fn assert_address_owned(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    address_bytes: &[u8],
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let owned: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM wallets WHERE eth_address = $1 AND user_id = $2 LIMIT 1"
    )
    .bind(address_bytes)
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| db_error(&e.to_string()))?;

    if owned.is_none() {
        return Err(forbidden("address does not belong to authenticated user"));
    }
    Ok(())
}

/// Parse and validate a 0x-prefixed 20-byte address into raw bytes.
fn parse_address_bytes(addr: &str) -> Result<Vec<u8>, (StatusCode, Json<ErrorResponse>)> {
    let s = addr.strip_prefix("0x").unwrap_or(addr);
    let bytes = hex::decode(s).map_err(|_| validation_error("invalid address hex"))?;
    if bytes.len() != 20 {
        return Err(validation_error("address must be 20 bytes"));
    }
    Ok(bytes)
}
