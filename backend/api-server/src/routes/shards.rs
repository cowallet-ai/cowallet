use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::middleware::auth::Claims;
use crate::services::crypto::{EncryptedData, EncryptionService};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/shard", post(upload_shard))
        .route("/shard/{location}", get(get_shard))
        .route("/backup-hash", post(store_backup_hash))
        .route("/status", get(shard_status))
}

#[derive(Deserialize)]
pub struct UploadShardRequest {
    location: String, // 'device' or 'backup' (the 'server' shard is DKG-owned, not uploadable here)
    party_index: i16,
    shard_hex: String, // Hex-encoded shard data (33 bytes for Shamir)
}

#[derive(Serialize)]
pub struct UploadShardResponse {
    success: bool,
    shard_id: Uuid,
}

#[derive(Deserialize)]
pub struct GetShardQuery {
    client_ephemeral_key: Option<String>,
}

#[derive(Serialize)]
pub struct GetShardResponse {
    location: String,
    party_index: i16,
    encrypted_shard_hex: String,
    server_ephemeral_key_hex: String,
    transport_nonce_hex: String,
    status: String,
}

#[derive(Serialize)]
pub struct ShardStatusItem {
    location: String,
    party_index: i16,
    status: String,
    last_used: Option<String>,
    last_verified: Option<String>,
}

#[derive(Serialize)]
pub struct ShardStatusResponse {
    shards: Vec<ShardStatusItem>,
    server_time: String,
}

/// Get status of all key shards for the authenticated user
async fn shard_status(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<ShardStatusResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let rows: Vec<(
        String,
        i16,
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT location, party_index, status, last_used, last_verified
         FROM shard_metadata
         WHERE user_id = $1
         ORDER BY party_index",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch shard status: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let shards = rows
        .into_iter()
        .map(
            |(location, party_index, status, last_used, last_verified)| ShardStatusItem {
                location,
                party_index,
                status,
                last_used: last_used.map(|t| t.to_rfc3339()),
                last_verified: last_verified.map(|t| t.to_rfc3339()),
            },
        )
        .collect();

    Ok(Json(ShardStatusResponse {
        shards,
        server_time: chrono::Utc::now().to_rfc3339(),
    }))
}

/// Upload an encrypted key shard
async fn upload_shard(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Extension(encryption): Extension<EncryptionService>,
    Json(body): Json<UploadShardRequest>,
) -> Result<Json<UploadShardResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Validate location. The 'server' shard is owned exclusively by the DKG
    // participant (encrypted with AES-GCM + AAD identity binding via
    // MpcParticipant/shard_store); allowing it here would let a REST upload
    // overwrite the DKG-managed row with an unbound, incompatibly-encrypted blob
    // and brick signing.
    let valid_locations = ["device", "backup"];
    if !valid_locations.contains(&body.location.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Decode hex shard
    let shard_bytes = hex::decode(&body.shard_hex).map_err(|_| StatusCode::BAD_REQUEST)?;

    if shard_bytes.len() != 33 {
        // Shamir share: 1 byte x + 32 bytes y
        return Err(StatusCode::BAD_REQUEST);
    }

    // Encrypt the shard
    let encrypted = encryption.encrypt(&shard_bytes).map_err(|e| {
        tracing::error!("Encryption failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Upsert shard metadata + encrypted content
    let shard_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO shard_metadata
         (user_id, location, party_index, status, encrypted_shard, nonce, encryption_key_id)
         VALUES ($1, $2, $3, 'healthy', $4, $5, $6)
         ON CONFLICT (user_id, location) DO UPDATE SET
             encrypted_shard = EXCLUDED.encrypted_shard,
             nonce = EXCLUDED.nonce,
             encryption_key_id = EXCLUDED.encryption_key_id,
             status = 'healthy',
             last_verified = NOW()
         RETURNING id",
    )
    .bind(user_id)
    .bind(&body.location)
    .bind(body.party_index)
    .bind(&encrypted.ciphertext)
    .bind(&encrypted.nonce.as_slice())
    .bind(encryption.key_id())
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store shard: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(
        "Stored shard for user {} location {} party {}",
        user_id, body.location, body.party_index
    );

    Ok(Json(UploadShardResponse {
        success: true,
        shard_id: shard_id.0,
    }))
}

/// Retrieve an encrypted key shard (envelope-encrypted for transport)
async fn get_shard(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Extension(encryption): Extension<EncryptionService>,
    Path(location): Path<String>,
    Query(query): Query<GetShardQuery>,
) -> Result<Json<GetShardResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Require client ephemeral key for envelope encryption
    let client_key_hex = query.client_ephemeral_key.as_deref().ok_or_else(|| {
        tracing::warn!("Shard retrieval without client_ephemeral_key rejected");
        StatusCode::BAD_REQUEST
    })?;

    let client_key_bytes = hex::decode(client_key_hex).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse client's ephemeral public key (secp256k1)
    let client_pk = k256::PublicKey::from_sec1_bytes(&client_key_bytes).map_err(|_| {
        tracing::error!("Invalid client ephemeral public key");
        StatusCode::BAD_REQUEST
    })?;

    // Validate location. 'server' shards are DKG-owned and encrypted with
    // AES-GCM + AAD identity binding (see upload_shard); they can never be read
    // back through this transport path, which decrypts with the unbound
    // 'default-key' context and would fail.
    let valid_locations = ["device", "backup"];
    if !valid_locations.contains(&location.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Fetch shard from DB
    let row: (Vec<u8>, Vec<u8>, i16, String) = sqlx::query_as(
        "SELECT encrypted_shard, nonce, party_index, status
         FROM shard_metadata
         WHERE user_id = $1 AND location = $2",
    )
    .bind(user_id)
    .bind(&location)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch shard: {}", e);
        StatusCode::NOT_FOUND
    })?;

    let encrypted_shard = row.0;
    let nonce_vec = row.1;
    let party_index = row.2;
    let status = row.3;

    if nonce_vec.len() != 12 {
        tracing::error!("Invalid nonce length: {}", nonce_vec.len());
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_vec);

    // Decrypt the shard
    let encrypted = EncryptedData {
        nonce,
        ciphertext: encrypted_shard,
    };

    let decrypted = encryption.decrypt(&encrypted).map_err(|e| {
        tracing::error!("Decryption failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Update last_used timestamp
    let _ = sqlx::query(
        "UPDATE shard_metadata SET last_used = NOW()
         WHERE user_id = $1 AND location = $2",
    )
    .bind(user_id)
    .bind(&location)
    .execute(db)
    .await;

    // Envelope encryption: ECDH with client's ephemeral key, then AES-GCM
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};
    use hkdf::Hkdf;
    use k256::ecdh::EphemeralSecret;
    use sha2::Sha256;

    let server_secret = EphemeralSecret::random(&mut rand::thread_rng());
    let server_pk = server_secret.public_key();
    let shared_secret = server_secret.diffie_hellman(&client_pk);

    // Derive AES-256 key via HKDF-SHA256 (RFC 5869).
    // salt=None (uses HKDF zero-length salt, equivalent to all-zeros),
    // info binds the key to this protocol context.
    let hkdf = Hkdf::<Sha256>::new(None, shared_secret.raw_secret_bytes());
    let mut aes_key_bytes = [0u8; 32];
    hkdf.expand(b"cowallet-shard-transport-v1", &mut aes_key_bytes)
        .expect("HKDF expand: 32-byte output is always valid");
    let cipher =
        Aes256Gcm::new_from_slice(&aes_key_bytes).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let transport_nonce_bytes: [u8; 12] = rand::random();
    let nonce_for_transport = aes_gcm::Nonce::from_slice(&transport_nonce_bytes);

    let encrypted_for_transport = cipher
        .encrypt(nonce_for_transport, decrypted.as_slice())
        .map_err(|_| {
            tracing::error!("Transport encryption failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let server_pk_bytes = server_pk.to_sec1_bytes();

    Ok(Json(GetShardResponse {
        location,
        party_index,
        encrypted_shard_hex: hex::encode(encrypted_for_transport),
        server_ephemeral_key_hex: hex::encode(server_pk_bytes),
        transport_nonce_hex: hex::encode(transport_nonce_bytes),
        status,
    }))
}

#[derive(Deserialize)]
struct StoreBackupHashRequest {
    backup_shard_hash_hex: String,
}

#[derive(Serialize)]
struct StoreBackupHashResponse {
    success: bool,
}

/// POST /api/v1/shards/backup-hash — store SHA-256(backup_shard) for future force re-register verification.
/// Called by the client after DKG completes and backup shard is stored.
async fn store_backup_hash(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<StoreBackupHashRequest>,
) -> Result<Json<StoreBackupHashResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let hash_bytes =
        hex::decode(&body.backup_shard_hash_hex).map_err(|_| StatusCode::BAD_REQUEST)?;

    if hash_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query(
        "UPDATE shard_metadata SET backup_shard_hash = $1
         WHERE user_id = $2 AND location = 'server'",
    )
    .bind(&hash_bytes)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store backup shard hash: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("Stored backup shard hash for user {}", user_id);

    Ok(Json(StoreBackupHashResponse { success: true }))
}
