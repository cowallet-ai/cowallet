use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Extension,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

use crate::middleware::auth::Claims;
use crate::state::AppState;

/// Resolve a wallet identifier (UUID string or 0x-address) to a UUID.
async fn resolve_wallet_id(
    wid: &str,
    db: &sqlx::PgPool,
) -> Result<uuid::Uuid, StatusCode> {
    if let Ok(uid) = uuid::Uuid::parse_str(wid) {
        return Ok(uid);
    }
    if wid.starts_with("0x") || wid.starts_with("0X") {
        let addr_bytes = hex::decode(wid.trim_start_matches("0x").trim_start_matches("0X"))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let row: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM wallets WHERE eth_address = $1"
        )
        .bind(&addr_bytes)
        .fetch_optional(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(row.ok_or(StatusCode::NOT_FOUND)?.0);
    }
    Err(StatusCode::BAD_REQUEST)
}

/// Parse the authenticated user's UUID from JWT claims.
fn claims_user_id(claims: &Claims) -> Result<uuid::Uuid, StatusCode> {
    uuid::Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Fetch the owner (user_id) of an MPC session.
/// Returns NOT_FOUND if the session does not exist.
async fn fetch_session_owner(
    db: &sqlx::PgPool,
    session_id: uuid::Uuid,
) -> Result<uuid::Uuid, StatusCode> {
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM mpc_sessions WHERE id = $1"
    )
    .bind(session_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!("fetch_session_owner query failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(row.ok_or(StatusCode::NOT_FOUND)?.0)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/session", post(create_session))
        .route("/session/{id}", get(get_session))
        .route("/session/{id}", delete(abort_session))
        .route("/session/{id}/msg", post(send_message))
        .route("/session/{id}/msg", get(recv_messages))
        .route("/session/{id}/backup-contribution", get(get_backup_contribution))
        .route("/session/{id}/resume", post(resume_session))
        .route("/sessions/pending", get(list_pending_sessions))
        .route("/presign/status", get(presign_status))
        .route("/presign/generate", post(presign_generate))
}

#[derive(Deserialize)]
pub(crate) struct CreateSessionRequest {
    session_type: String,
    parties: Vec<i16>,
    threshold: Option<i16>,
    /// Optional wallet identifier: UUID or 0x-prefixed ETH address.
    /// When provided, the session is associated with a specific wallet
    /// and uses that wallet's key share for signing.
    wallet_id: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct SessionResponse {
    session_id: String,
    status: String,
    current_round: i32,
    last_activity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wallet_id: Option<String>,
    /// Per-session HMAC key (hex), returned only from create_session to the
    /// authenticated owner. The client signs each server-bound MPC message with
    /// this key (F-004). Never returned by get_session.
    #[serde(skip_serializing_if = "Option::is_none")]
    hmac_key: Option<String>,
}

/// Create a new MPC session
pub async fn create_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let valid_types = ["dkg", "keygen", "presign", "sign", "reshare"];
    if !valid_types.contains(&body.session_type.as_str()) {
        tracing::warn!("Invalid session_type: {}", body.session_type);
        return Err(StatusCode::BAD_REQUEST);
    }

    let session_id = uuid::Uuid::new_v4();
    let threshold = body.threshold.unwrap_or(2);

    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Resolve wallet_id: accept UUID or 0x-prefixed address
    let wallet_id: Option<uuid::Uuid> = match &body.wallet_id {
        Some(wid) => Some(resolve_wallet_id(wid, db).await?),
        None => None,
    };

    // Block sign/presign sessions if wallet is frozen
    if matches!(body.session_type.as_str(), "sign" | "presign") {
        if let Some(wid) = wallet_id {
            let status: Option<(String,)> = sqlx::query_as(
                "SELECT status FROM wallets WHERE id = $1"
            )
            .bind(wid)
            .fetch_optional(db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check wallet freeze status: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            if let Some((s,)) = status {
                if s == "frozen" {
                    tracing::warn!("Wallet {} is frozen, rejecting {} session", wid, body.session_type);
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }
    }

    // Generate a per-session HMAC key (F-004). Handed to the authenticated
    // owner in this response only; used to authenticate server-bound messages.
    let hmac_key_bytes: [u8; 32] = {
        use rand::RngCore;
        let mut k = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut k);
        k
    };

    sqlx::query(
        "INSERT INTO mpc_sessions (id, user_id, session_type, parties, threshold, status, current_round, wallet_id, hmac_key)
         VALUES ($1, $2, $3, $4, $5, 'active', 0, $6, $7)"
    )
    .bind(session_id)
    .bind(user_id)
    .bind(&body.session_type)
    .bind(&body.parties)
    .bind(threshold)
    .bind(wallet_id)
    .bind(&hmac_key_bytes[..])
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create MPC session: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Created MPC session {} for user {}", session_id, claims.sub);

    // Notify the server MPC participant to join this session.
    // If this fails, mark the session as failed immediately — a session without
    // a server participant can never complete, and leaving it "active" causes
    // the client to connect and wait forever for Round 1 that will never arrive.
    if let Some(participant) = &state.mpc_participant {
        if let Err(e) = participant.on_session_created(
            session_id,
            user_id,
            &body.session_type,
            &body.parties,
            threshold,
            wallet_id,
        ).await {
            tracing::error!("Server participant failed to join session {}: {}", session_id, e);
            let _ = sqlx::query(
                "UPDATE mpc_sessions SET status = 'failed' WHERE id = $1"
            )
            .bind(session_id)
            .execute(db)
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    Ok(Json(SessionResponse {
        session_id: session_id.to_string(),
        status: "active".to_string(),
        current_round: 0,
        last_activity: None,
        wallet_id: wallet_id.map(|w| w.to_string()),
        hmac_key: Some(hex::encode(hmac_key_bytes)),
    }))
}

/// Get session status
pub async fn get_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let caller = claims_user_id(&claims)?;
    let owner = fetch_session_owner(db, id).await?;
    if owner != caller {
        tracing::warn!("User {} attempted to read session {} owned by {}", caller, id, owner);
        return Err(StatusCode::FORBIDDEN);
    }

    let row: (String, i32, Option<chrono::DateTime<Utc>>, Option<uuid::Uuid>, uuid::Uuid) = sqlx::query_as(
        "SELECT status, current_round, last_activity, wallet_id, user_id FROM mpc_sessions WHERE id = $1"
    )
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(SessionResponse {
        session_id: id.to_string(),
        status: row.0,
        current_round: row.1,
        last_activity: row.2.map(|t| t.to_rfc3339()),
        wallet_id: row.3.map(|w| w.to_string()),
        // Never re-issue the session HMAC key after creation.
        hmac_key: None,
    }))
}

/// Abort a session
pub async fn abort_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<uuid::Uuid>,
) -> Result<StatusCode, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let caller = claims_user_id(&claims)?;

    let result = sqlx::query(
        "UPDATE mpc_sessions SET status = 'failed'
         WHERE id = $1 AND user_id = $2 AND status IN ('pending', 'active')"
    )
    .bind(id)
    .bind(caller)
    .execute(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::GONE);
    }

    tracing::info!("User {} aborted MPC session {}", caller, id);
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct SendMessageRequest {
    from_party: i16,
    to_party: i16,
    round: i16,
    payload: Vec<u8>,
    /// Optional HMAC for message integrity verification
    hmac: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct SendMessageResponse {
    message_id: i64,
    verified: bool,
}

/// Send a message to another party in the session
pub async fn send_message(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<uuid::Uuid>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    // Fetch session details
    let session: (String, Vec<i16>, i32, uuid::Uuid, Option<Vec<u8>>) = sqlx::query_as(
        "SELECT status, parties, current_round, user_id, hmac_key FROM mpc_sessions WHERE id = $1"
    )
    .bind(session_id)
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let status = &session.0;
    let parties = &session.1;
    let current_round = session.2 as i16;
    let session_user_id = session.3;
    let session_hmac_key = &session.4;

    // Enforce session ownership: only the owner may drive their session.
    let caller = claims_user_id(&claims)?;
    if session_user_id != caller {
        tracing::warn!(
            "User {} attempted to send message to session {} owned by {}",
            caller, session_id, session_user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Session must be active
    if status != "active" {
        return Err(StatusCode::GONE);
    }

    // Validate party indices
    if !parties.contains(&body.from_party) || !parties.contains(&body.to_party) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate round (must be >= current round)
    if body.round < current_round {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify HMAC against the per-session key (F-004). The key was handed to
    // the owner at session creation; each server-bound message must carry an
    // HMAC over (session_id ‖ round ‖ payload). `verify_slice` is constant-time.
    let verified = match (&body.hmac, session_hmac_key) {
        (Some(hmac_value), Some(key)) if !key.is_empty() => {
            use hmac::{Hmac, Mac};
            type HmacSha256 = Hmac<Sha256>;
            match hex::decode(hmac_value) {
                Ok(provided) => {
                    let mut mac = HmacSha256::new_from_slice(key)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    mac.update(session_id.to_string().as_bytes());
                    mac.update(&body.round.to_le_bytes());
                    mac.update(&body.payload);
                    mac.verify_slice(&provided).is_ok()
                }
                Err(_) => false,
            }
        }
        _ => false,
    };

    // Store message
    let message_id: i64 = sqlx::query_scalar(
        "INSERT INTO mpc_messages (session_id, from_party, to_party, round, payload, verified)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id"
    )
    .bind(session_id)
    .bind(body.from_party)
    .bind(body.to_party)
    .bind(body.round)
    .bind(&body.payload)
    .bind(verified)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store message: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Update session activity and round
    sqlx::query(
        "UPDATE mpc_sessions SET last_activity = NOW(), current_round = GREATEST(current_round, $2)
         WHERE id = $1"
    )
    .bind(session_id)
    .bind(body.round as i32)
    .execute(db)
    .await
    .ok();

    // If this message is addressed to the server (Party 1), trigger the participant
    if body.to_party == 1 {
        // Messages that drive the server signing state machine MUST be authenticated.
        if !verified {
            tracing::warn!(
                "Rejected unverified message to server for session {}",
                session_id
            );
            return Err(StatusCode::UNAUTHORIZED);
        }
        if let Some(participant) = &state.mpc_participant {
            match participant.on_message_received(
                session_id,
                body.from_party,
                body.round,
                &body.payload,
            ).await {
                Ok(responses) => {
                    tracing::info!(
                        "Server participant processed message for session {} round {}, {} responses",
                        session_id, body.round, responses.len()
                    );
                    // Publish response messages to NATS so the client's WS gets them in real-time.
                    // Messages are already stored in DB by the participant's store_outbound_message.
                    if let Some(nats) = &state.nats {
                        for (from, to, msg_round, payload) in responses {
                            // to == -1 means broadcast; send to the requesting party
                            let target_party = if to == -1 { body.from_party } else { to };
                            let response_msg = serde_json::json!({
                                "from_party": from,
                                "to_party": target_party,
                                "round": msg_round,
                                "payload": payload,
                            });
                            let subject = format!("cowallet.mpc.{}.{}", session_id, target_party);
                            if let Ok(data) = serde_json::to_vec(&response_msg) {
                                if let Err(e) = nats.publish(subject.clone(), data.into()).await {
                                    tracing::warn!("NATS publish to {} failed: {}", subject, e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Server participant error for session {} round {}: {}",
                        session_id, body.round, e
                    );
                    // Mark session as failed so the client stops polling
                    let _ = sqlx::query(
                        "UPDATE mpc_sessions SET status = 'failed', completed_at = NOW() WHERE id = $1 AND status = 'active'"
                    )
                    .bind(session_id)
                    .execute(db)
                    .await;
                }
            }
        }
    }

    Ok(Json(SendMessageResponse {
        message_id,
        verified,
    }))
}

#[derive(Deserialize)]
pub(crate) struct RecvQuery {
    /// Filter messages addressed to this party (required).
    party: Option<i16>,
    /// Only return messages after this ID (for polling).
    after_id: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct MessageResponse {
    id: i64,
    from_party: i16,
    to_party: i16,
    round: i16,
    payload: Vec<u8>,
    verified: bool,
    created_at: String,
}

/// Receive messages for a session (polling-based).
/// Query params:
///   ?party=0  — filter messages addressed to this party (or broadcast)
///   ?after_id=5 — only return messages with id > this value
pub async fn recv_messages(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<uuid::Uuid>,
    Query(query): Query<RecvQuery>,
) -> Result<Json<Vec<MessageResponse>>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Verify session exists AND belongs to the caller
    let caller = claims_user_id(&claims)?;
    let owner = fetch_session_owner(db, session_id).await?;
    if owner != caller {
        tracing::warn!("User {} attempted to read messages of session {} owned by {}", caller, session_id, owner);
        return Err(StatusCode::FORBIDDEN);
    }

    let after_id = query.after_id.unwrap_or(0);

    let messages: Vec<(i64, i16, i16, i16, Vec<u8>, bool, chrono::DateTime<Utc>)> = if let Some(party) = query.party {
        // Filter: messages addressed to this party OR broadcast (0xFFFF = 65535 as i16 = -1)
        sqlx::query_as(
            "SELECT id, from_party, to_party, round, payload, verified, created_at
             FROM mpc_messages
             WHERE session_id = $1 AND id > $2 AND (to_party = $3 OR to_party = -1)
             ORDER BY round ASC, created_at ASC"
        )
        .bind(session_id)
        .bind(after_id)
        .bind(party)
        .fetch_all(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        // No party filter — return all messages for this session
        sqlx::query_as(
            "SELECT id, from_party, to_party, round, payload, verified, created_at
             FROM mpc_messages
             WHERE session_id = $1 AND id > $2
             ORDER BY round ASC, created_at ASC"
        )
        .bind(session_id)
        .bind(after_id)
        .fetch_all(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    Ok(Json(messages.into_iter().map(|m| MessageResponse {
        id: m.0,
        from_party: m.1,
        to_party: m.2,
        round: m.3,
        payload: m.4,
        verified: m.5,
        created_at: m.6.to_rfc3339(),
    }).collect()))
}

/// Get the server's backup contribution for a completed DKG session.
/// Returns the 32-byte f_server(3) scalar for the client to combine with f_device(3).
/// This is a single-use endpoint: the contribution is removed after fetching.
pub async fn get_backup_contribution(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<uuid::Uuid>,
) -> Result<Json<Vec<u8>>, StatusCode> {
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Verify the session exists and belongs to this user
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let session_user: Option<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT user_id, status FROM mpc_sessions WHERE id = $1"
    )
    .bind(session_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query session: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match session_user {
        Some((session_user_id, status)) => {
            if session_user_id != user_id {
                tracing::warn!(
                    "User {} attempted to access backup contribution for session {} owned by {}",
                    user_id, session_id, session_user_id
                );
                return Err(StatusCode::FORBIDDEN);
            }

            // Only allow fetching for completed DKG sessions
            if status != "completed" {
                tracing::warn!(
                    "Backup contribution requested for session {} with status '{}'",
                    session_id, status
                );
                return Err(StatusCode::CONFLICT);
            }
        }
        None => {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // Fetch the backup contribution from the MPC participant
    if let Some(participant) = &state.mpc_participant {
        if let Some(contribution) = participant.fetch_backup_contribution(session_id, user_id) {
            if contribution.len() != 32 {
                tracing::error!(
                    "Invalid backup contribution length for session {}: {} bytes",
                    session_id, contribution.len()
                );
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }

            tracing::info!(
                "User {} fetched backup contribution for session {}",
                user_id, session_id
            );
            return Ok(Json(contribution));
        }
    }

    // Contribution not available (either never computed, already fetched, or expired)
    tracing::debug!(
        "Backup contribution not available for session {} (user {})",
        session_id, user_id
    );
    Err(StatusCode::NOT_FOUND)
}

// ─── Session Recovery Endpoints ──────────────────────────────────────────────

#[derive(Serialize)]
pub(crate) struct PendingSessionResponse {
    session_id: String,
    session_type: String,
    status: String,
    current_round: i32,
    wallet_id: Option<String>,
    created_at: String,
    last_activity: Option<String>,
}

/// GET /sessions/pending
/// List active or interrupted sessions for the authenticated user that may be resumable.
/// Only returns sessions within the 5-minute expiry window.
pub async fn list_pending_sessions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<PendingSessionResponse>>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let rows: Vec<(uuid::Uuid, String, String, i32, Option<uuid::Uuid>, chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, session_type, status, current_round, wallet_id, created_at, last_activity
         FROM mpc_sessions
         WHERE user_id = $1
           AND status IN ('active', 'interrupted')
           AND expires_at > NOW()
         ORDER BY created_at DESC
         LIMIT 10"
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list pending sessions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let sessions: Vec<PendingSessionResponse> = rows.into_iter().map(|r| {
        PendingSessionResponse {
            session_id: r.0.to_string(),
            session_type: r.1,
            status: r.2,
            current_round: r.3,
            wallet_id: r.4.map(|w| w.to_string()),
            created_at: r.5.to_rfc3339(),
            last_activity: r.6.map(|t| t.to_rfc3339()),
        }
    }).collect();

    Ok(Json(sessions))
}

/// POST /session/{id}/resume
/// Resume an interrupted or active session. Reactivates the session and
/// extends its expiry. Returns the session state + any missed messages
/// so the client can replay from where it left off.
pub async fn resume_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<uuid::Uuid>,
) -> Result<Json<ResumeSessionResponse>, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Fetch session and verify ownership
    let session: Option<(String, String, i32, uuid::Uuid, Option<uuid::Uuid>, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT status, session_type, current_round, user_id, wallet_id, expires_at
         FROM mpc_sessions WHERE id = $1"
    )
    .bind(session_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query session for resume: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (status, session_type, current_round, session_user_id, wallet_id, expires_at) =
        session.ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if session_user_id != user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Only resume active or interrupted sessions
    if status != "active" && status != "interrupted" {
        tracing::info!("Cannot resume session {} with status '{}'", session_id, status);
        return Err(StatusCode::GONE);
    }

    // Check if session has expired (even if status is still active/interrupted)
    if expires_at < Utc::now() {
        // Mark as expired
        let _ = sqlx::query(
            "UPDATE mpc_sessions SET status = 'expired', completed_at = NOW() WHERE id = $1"
        )
        .bind(session_id)
        .execute(db)
        .await;
        return Err(StatusCode::GONE);
    }

    // Reactivate: set status to 'active', extend expiry by 5 minutes
    sqlx::query(
        "UPDATE mpc_sessions
         SET status = 'active', last_activity = NOW(), expires_at = NOW() + INTERVAL '5 minutes'
         WHERE id = $1"
    )
    .bind(session_id)
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to reactivate session: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Re-initialize server participant state for sign sessions.
    // The client will generate fresh crypto state on resume, so the server
    // must also start fresh to keep k values in sync.
    if session_type == "sign" {
        if let Some(participant) = &state.mpc_participant {
            // Remove stale server state (if any) so both sides restart cleanly
            participant.remove_session(session_id);

            // Re-initialize the server sign session
            let parties = vec![0i16, 1i16];
            if let Err(e) = participant.on_session_created(
                session_id,
                user_id,
                &session_type,
                &parties,
                2,
                wallet_id,
            ).await {
                tracing::warn!("Failed to re-init server participant for resume: {}", e);
                // Don't fail the resume — the client will detect this on message send
            }

            // Clear old messages so catch-up doesn't include stale protocol data
            let _ = sqlx::query(
                "DELETE FROM mpc_messages WHERE session_id = $1"
            )
            .bind(session_id)
            .execute(db)
            .await;

            // Reset round counter
            let _ = sqlx::query(
                "UPDATE mpc_sessions SET current_round = 0 WHERE id = $1"
            )
            .bind(session_id)
            .execute(db)
            .await;
        }
    }

    // Fetch messages for client catch-up (all messages addressed to party 0)
    let messages: Vec<(i64, i16, i16, i16, Vec<u8>, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, from_party, to_party, round, payload, created_at
         FROM mpc_messages
         WHERE session_id = $1 AND (to_party = 0 OR to_party = -1)
         ORDER BY round ASC, created_at ASC"
    )
    .bind(session_id)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch catch-up messages: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let catch_up_messages: Vec<CatchUpMessage> = messages.into_iter().map(|m| CatchUpMessage {
        id: m.0,
        from_party: m.1,
        to_party: m.2,
        round: m.3,
        payload: m.4,
        created_at: m.5.to_rfc3339(),
    }).collect();

    tracing::info!(
        "Session {} resumed for user {} (type={}, round={}, {} catch-up messages)",
        session_id, user_id, session_type, current_round, catch_up_messages.len()
    );

    Ok(Json(ResumeSessionResponse {
        session_id: session_id.to_string(),
        session_type,
        status: "active".to_string(),
        current_round,
        wallet_id: wallet_id.map(|w| w.to_string()),
        messages: catch_up_messages,
    }))
}

#[derive(Serialize)]
pub(crate) struct ResumeSessionResponse {
    session_id: String,
    session_type: String,
    status: String,
    current_round: i32,
    wallet_id: Option<String>,
    messages: Vec<CatchUpMessage>,
}

#[derive(Serialize)]
pub(crate) struct CatchUpMessage {
    id: i64,
    from_party: i16,
    to_party: i16,
    round: i16,
    payload: Vec<u8>,
    created_at: String,
}

// ─── Presignature Management Endpoints ───────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct PresignStatusQuery {
    wallet_id: Option<uuid::Uuid>,
    address: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct PresignStatusResponse {
    available: i64,
    wallet_id: String,
}

/// GET /presign/status?wallet_id={uuid} or ?address={0x...}
/// Returns the number of available presignatures for the given wallet.
pub async fn presign_status(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Query(query): Query<PresignStatusQuery>,
) -> Result<Json<PresignStatusResponse>, StatusCode> {
    let presign_mgr = state.presign_manager
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let wallet_id = if let Some(id) = query.wallet_id {
        id
    } else if let Some(addr) = &query.address {
        let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let addr_bytes = hex::decode(addr.trim_start_matches("0x"))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let row: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM wallets WHERE eth_address = $1"
        )
        .bind(addr_bytes)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            tracing::error!("presign_status wallet lookup error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        row.ok_or(StatusCode::NOT_FOUND)?.0
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let available = presign_mgr
        .get_available_count(wallet_id)
        .await
        .map_err(|e| {
            tracing::error!("presign_status error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(PresignStatusResponse {
        available,
        wallet_id: wallet_id.to_string(),
    }))
}

#[derive(Deserialize)]
pub(crate) struct PresignGenerateRequest {
    wallet_id: String,
    count: Option<u32>,
}

#[derive(Serialize)]
pub(crate) struct PresignGenerateResponse {
    generated: u32,
    wallet_id: String,
}

/// POST /presign/generate
/// Triggers presignature generation for the given wallet.
pub async fn presign_generate(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<PresignGenerateRequest>,
) -> Result<Json<PresignGenerateResponse>, StatusCode> {
    let presign_mgr = state.presign_manager
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let wallet_id = resolve_wallet_id(&body.wallet_id, db).await?;

    let count = body.count.unwrap_or(5).min(50);

    let generated = presign_mgr
        .generate_presignatures(user_id, wallet_id, count)
        .await
        .map_err(|e| {
            tracing::error!("presign_generate error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "User {} generated {} presignatures for wallet {}",
        user_id, generated, wallet_id
    );

    Ok(Json(PresignGenerateResponse {
        generated,
        wallet_id: wallet_id.to_string(),
    }))
}

#[cfg(test)]
mod ownership_tests {
    use super::*;

    #[test]
    fn claims_user_id_parses_valid_uuid() {
        let claims = Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            jti: "00000000-0000-0000-0000-000000000000".to_string(),
            device_id: "DEV0000000000001".to_string(),
            exp: 9999999999,
            iat: 0,
            token_type: crate::middleware::auth::TokenType::Access,
        };
        let uid = claims_user_id(&claims).expect("should parse");
        assert_eq!(uid.to_string(), "11111111-1111-1111-1111-111111111111");
    }

    #[test]
    fn claims_user_id_rejects_garbage() {
        let claims = Claims {
            sub: "not-a-uuid".to_string(),
            jti: "00000000-0000-0000-0000-000000000000".to_string(),
            device_id: "DEV0000000000001".to_string(),
            exp: 9999999999,
            iat: 0,
            token_type: crate::middleware::auth::TokenType::Access,
        };
        assert_eq!(claims_user_id(&claims).unwrap_err(), StatusCode::UNAUTHORIZED);
    }
}
