use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use uuid::Uuid;

use crate::middleware::auth::Claims;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_wallets))
        .route("/", post(create_wallet))
        .route("/{id}", get(get_wallet))
        .route("/{id}/chains", post(add_chain))
        .route("/{id}/chains/{chain_id}", delete(remove_chain))
        .route("/{id}/freeze", post(freeze_wallet))
        .route("/{id}/unfreeze", post(unfreeze_wallet))
}

#[derive(Serialize)]
pub struct WalletResponse {
    pub id: String,
    pub name: String,
    pub eth_address: String,
    pub chain_ids: Vec<i64>,
    pub status: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
    pub public_key_hex: String,
    pub chain_ids: Option<Vec<i64>>,
}

#[derive(Deserialize)]
pub struct AddChainRequest {
    pub chain_id: i64,
}

/// Compute Ethereum address from uncompressed public key (04 || x || y).
/// Takes keccak256 of x||y (64 bytes), returns last 20 bytes.
fn eth_address_from_pubkey(pubkey_hex: &str) -> Result<[u8; 20], StatusCode> {
    let pubkey_bytes = hex::decode(pubkey_hex).map_err(|_| StatusCode::BAD_REQUEST)?;

    let xy_bytes = if pubkey_bytes.len() == 65 && pubkey_bytes[0] == 0x04 {
        pubkey_bytes[1..].to_vec()
    } else if pubkey_bytes.len() == 64 {
        pubkey_bytes.to_vec()
    } else if pubkey_bytes.len() == 33 && (pubkey_bytes[0] == 0x02 || pubkey_bytes[0] == 0x03) {
        let pk = k256::PublicKey::from_sec1_bytes(&pubkey_bytes).map_err(|_| {
            tracing::error!("Failed to decompress public key");
            StatusCode::BAD_REQUEST
        })?;
        let uncompressed = pk.to_encoded_point(false);
        uncompressed.as_bytes()[1..].to_vec()
    } else {
        tracing::error!("Invalid public key length: {}", pubkey_bytes.len());
        return Err(StatusCode::BAD_REQUEST);
    };

    // keccak256 of x||y
    let hash = Keccak256::digest(xy_bytes);

    // Take last 20 bytes
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    Ok(addr)
}

/// GET /api/v1/wallets — list all wallets for the authenticated user
async fn list_wallets(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<WalletResponse>>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let rows: Vec<(Uuid, String, Vec<u8>, Vec<i64>, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, name, eth_address, chain_ids, status, created_at
         FROM wallets
         WHERE user_id = $1 AND status != 'archived'
         ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list wallets: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let wallets: Vec<WalletResponse> = rows
        .into_iter()
        .map(|row| WalletResponse {
            id: row.0.to_string(),
            name: row.1,
            eth_address: format!("0x{}", hex::encode(&row.2)),
            chain_ids: row.3,
            status: row.4,
            created_at: row.5.to_rfc3339(),
        })
        .collect();

    Ok(Json(wallets))
}

/// POST /api/v1/wallets — create a new wallet entry (called after DKG completes)
async fn create_wallet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateWalletRequest>,
) -> Result<(StatusCode, Json<WalletResponse>), StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Validate and compute eth_address from public key
    let eth_addr = eth_address_from_pubkey(&body.public_key_hex)?;

    let public_key_bytes =
        hex::decode(&body.public_key_hex).map_err(|_| StatusCode::BAD_REQUEST)?;

    let chain_ids = body
        .chain_ids
        .unwrap_or_else(|| vec![1, 8453, 42161, 10, 56, 137]);

    // Atomic insert — reject if user already has an active wallet
    // Uses a subquery to avoid check-then-insert race condition
    let row: Option<(Uuid, String, Vec<u8>, Vec<i64>, String, DateTime<Utc>)> = sqlx::query_as(
        "INSERT INTO wallets (user_id, name, public_key, eth_address, chain_ids, status)
         SELECT $1, $2, $3, $4, $5, 'active'
         WHERE NOT EXISTS (SELECT 1 FROM wallets WHERE user_id = $1 AND status = 'active')
         RETURNING id, name, eth_address, chain_ids, status, created_at",
    )
    .bind(user_id)
    .bind(&body.name)
    .bind(&public_key_bytes)
    .bind(&eth_addr.as_slice())
    .bind(&chain_ids)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create wallet: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let row = row.ok_or(StatusCode::CONFLICT)?;

    tracing::info!("Created wallet {} for user {}", row.0, user_id);

    Ok((
        StatusCode::CREATED,
        Json(WalletResponse {
            id: row.0.to_string(),
            name: row.1,
            eth_address: format!("0x{}", hex::encode(&row.2)),
            chain_ids: row.3,
            status: row.4,
            created_at: row.5.to_rfc3339(),
        }),
    ))
}

/// GET /api/v1/wallets/{id} — get a single wallet by ID
async fn get_wallet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<WalletResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let row: (Uuid, String, Vec<u8>, Vec<i64>, String, DateTime<Utc>) = sqlx::query_as(
        "SELECT id, name, eth_address, chain_ids, status, created_at
         FROM wallets
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get wallet {}: {}", id, e);
        StatusCode::NOT_FOUND
    })?;

    Ok(Json(WalletResponse {
        id: row.0.to_string(),
        name: row.1,
        eth_address: format!("0x{}", hex::encode(&row.2)),
        chain_ids: row.3,
        status: row.4,
        created_at: row.5.to_rfc3339(),
    }))
}

/// POST /api/v1/wallets/{id}/chains — add a supported chain to a wallet
async fn add_chain(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(body): Json<AddChainRequest>,
) -> Result<Json<WalletResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Append chain_id to array if not already present
    let row: (Uuid, String, Vec<u8>, Vec<i64>, String, DateTime<Utc>) = sqlx::query_as(
        "UPDATE wallets
         SET chain_ids = array_append(chain_ids, $3)
         WHERE id = $1 AND user_id = $2 AND NOT ($3 = ANY(chain_ids))
         RETURNING id, name, eth_address, chain_ids, status, created_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(body.chain_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        // If no rows returned, either wallet not found or chain already exists
        // Try fetching the wallet to distinguish
        tracing::warn!("add_chain update returned no rows for wallet {}: {}", id, e);
        StatusCode::NOT_FOUND
    })?;

    tracing::info!("Added chain {} to wallet {}", body.chain_id, id);

    Ok(Json(WalletResponse {
        id: row.0.to_string(),
        name: row.1,
        eth_address: format!("0x{}", hex::encode(&row.2)),
        chain_ids: row.3,
        status: row.4,
        created_at: row.5.to_rfc3339(),
    }))
}

/// Resolve wallet identifier: UUID or 0x-prefixed ETH address.
async fn resolve_wallet(wid: &str, db: &sqlx::PgPool) -> Result<Uuid, StatusCode> {
    if let Ok(uid) = Uuid::parse_str(wid) {
        return Ok(uid);
    }
    if wid.starts_with("0x") || wid.starts_with("0X") {
        let addr_bytes = hex::decode(wid.trim_start_matches("0x").trim_start_matches("0X"))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM wallets WHERE eth_address = $1")
            .bind(&addr_bytes)
            .fetch_optional(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(row.ok_or(StatusCode::NOT_FOUND)?.0);
    }
    Err(StatusCode::BAD_REQUEST)
}

/// POST /api/v1/wallets/{id}/freeze — freeze wallet, blocking all transactions
/// {id} can be a UUID or 0x-prefixed ETH address.
async fn freeze_wallet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<WalletResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let wallet_id = resolve_wallet(&id, db).await?;

    let row: (Uuid, String, Vec<u8>, Vec<i64>, String, DateTime<Utc>) = sqlx::query_as(
        "UPDATE wallets SET status = 'frozen' WHERE id = $1 AND user_id = $2
         RETURNING id, name, eth_address, chain_ids, status, created_at",
    )
    .bind(wallet_id)
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to freeze wallet {}: {}", wallet_id, e);
        StatusCode::NOT_FOUND
    })?;

    tracing::info!("Wallet {} frozen by user {}", wallet_id, user_id);

    Ok(Json(WalletResponse {
        id: row.0.to_string(),
        name: row.1,
        eth_address: format!("0x{}", hex::encode(&row.2)),
        chain_ids: row.3,
        status: row.4,
        created_at: row.5.to_rfc3339(),
    }))
}

/// POST /api/v1/wallets/{id}/unfreeze — unfreeze wallet, re-enabling transactions
/// {id} can be a UUID or 0x-prefixed ETH address.
async fn unfreeze_wallet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<WalletResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let wallet_id = resolve_wallet(&id, db).await?;

    let row: (Uuid, String, Vec<u8>, Vec<i64>, String, DateTime<Utc>) = sqlx::query_as(
        "UPDATE wallets SET status = 'active' WHERE id = $1 AND user_id = $2 AND status = 'frozen'
         RETURNING id, name, eth_address, chain_ids, status, created_at",
    )
    .bind(wallet_id)
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to unfreeze wallet {}: {}", wallet_id, e);
        StatusCode::NOT_FOUND
    })?;

    tracing::info!("Wallet {} unfrozen by user {}", wallet_id, user_id);

    Ok(Json(WalletResponse {
        id: row.0.to_string(),
        name: row.1,
        eth_address: format!("0x{}", hex::encode(&row.2)),
        chain_ids: row.3,
        status: row.4,
        created_at: row.5.to_rfc3339(),
    }))
}

/// DELETE /api/v1/wallets/{id}/chains/{chain_id} — remove a chain from a wallet
async fn remove_chain(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((id, chain_id)): Path<(Uuid, i64)>,
) -> Result<Json<WalletResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let row: (Uuid, String, Vec<u8>, Vec<i64>, String, DateTime<Utc>) = sqlx::query_as(
        "UPDATE wallets
         SET chain_ids = array_remove(chain_ids, $3)
         WHERE id = $1 AND user_id = $2
         RETURNING id, name, eth_address, chain_ids, status, created_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(chain_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to remove chain {} from wallet {}: {}",
            chain_id,
            id,
            e
        );
        StatusCode::NOT_FOUND
    })?;

    tracing::info!("Removed chain {} from wallet {}", chain_id, id);

    Ok(Json(WalletResponse {
        id: row.0.to_string(),
        name: row.1,
        eth_address: format!("0x{}", hex::encode(&row.2)),
        chain_ids: row.3,
        status: row.4,
        created_at: row.5.to_rfc3339(),
    }))
}
