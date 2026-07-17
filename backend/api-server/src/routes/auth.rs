use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::middleware::audit::{AuditLog, AuditResult};
use crate::middleware::auth::{Claims, issue_token_pair, blacklist_token, refresh_access_token, TokenPair, verify_token_unchecked};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/challenge", post(request_challenge))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/session", get(session_info))
        .route("/ws-ticket", post(ws_ticket))
        .route("/audit-log", get(audit_log))
        .route("/email/send-otp", post(send_email_otp))
        .route("/recovery/initiate", post(initiate_recovery))
        .route("/recovery/verify", post(verify_recovery_otp))
}

#[derive(Deserialize)]
pub struct AuditLogQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub action: Option<String>,
}

#[derive(Deserialize)]
struct SendEmailOtpRequest {
    email: String,
    #[serde(default)]
    force: bool,
    /// Cloudflare Turnstile token. Enforced only when TURNSTILE_SECRET_KEY is
    /// configured on the server; otherwise ignored (compat mode).
    #[serde(default)]
    turnstile_token: String,
}

#[derive(Serialize)]
struct SendEmailOtpResponse {
    sent: bool,
    is_registered: bool,
    message: String,
}

/// App Store / Play Store review bypass.
///
/// Returns `Some(fixed_otp)` when `email` is the configured review account.
/// Active ONLY when both `REVIEW_BYPASS_EMAIL` and `REVIEW_BYPASS_OTP` are set,
/// so it is disabled by default and can be switched off after review by simply
/// clearing those env vars — no code change. Scoped to one exact address; it
/// only skips the emailed OTP, never the device-key or MPC steps.
fn review_bypass_otp_for(email: &str) -> Option<String> {
    let allow = std::env::var("REVIEW_BYPASS_EMAIL").ok()?;
    let otp = std::env::var("REVIEW_BYPASS_OTP").ok()?;
    if allow.is_empty() || otp.is_empty() || !allow.eq_ignore_ascii_case(email.trim()) {
        return None;
    }
    Some(otp)
}

/// Send OTP to email for registration verification.
async fn send_email_otp(
    State(state): State<AppState>,
    Json(body): Json<SendEmailOtpRequest>,
) -> Result<Json<SendEmailOtpResponse>, StatusCode> {
    // Review-account bypass: seed a deterministic OTP so App Review can register
    // without receiving an email. Off unless env-configured; scoped to the one
    // address. Skips Turnstile + rate limit + SES; always routes to register
    // (is_registered=false) rather than the recovery flow.
    if let Some(fixed_otp) = review_bypass_otp_for(&body.email) {
        let db = state
            .require_db()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let otp_hash = Sha256::digest(fixed_otp.as_bytes());
        let expires_at = Utc::now() + chrono::Duration::minutes(30);
        let _ = sqlx::query("DELETE FROM email_verifications WHERE email = $1 AND NOT verified")
            .bind(&body.email)
            .execute(db)
            .await;
        sqlx::query("INSERT INTO email_verifications (email, otp_hash, expires_at) VALUES ($1, $2, $3)")
            .bind(&body.email)
            .bind(otp_hash.as_slice())
            .bind(expires_at)
            .execute(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(Json(SendEmailOtpResponse {
            sent: true,
            is_registered: false,
            message: format!("Verification code sent to {}", body.email),
        }));
    }

    // Human/bot check before doing any work. No-op unless TURNSTILE_SECRET_KEY
    // is configured (compat mode for local/dev).
    if let Err(e) =
        crate::services::turnstile::verify(&state.http, &body.turnstile_token, None).await
    {
        tracing::warn!("Turnstile check failed for OTP send: {e}");
        return Err(StatusCode::FORBIDDEN);
    }

    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    // Check if email has a completed wallet (user exists AND has server shard AND active wallet)
    let is_registered: bool = sqlx::query_as::<_, (uuid::Uuid,)>(
        "SELECT u.id FROM users u
         INNER JOIN shard_metadata s ON s.user_id = u.id AND s.location = 'server'
         INNER JOIN wallets w ON w.user_id = u.id AND w.status = 'active'
         WHERE u.email = $1
         LIMIT 1"
    )
    .bind(&body.email)
    .fetch_optional(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .is_some();

    // If already registered with wallet, skip OTP — client should redirect to recovery flow
    // Unless force=true (user has verified backup shard possession on client side)
    if is_registered && !body.force {
        return Ok(Json(SendEmailOtpResponse {
            sent: false,
            is_registered,
            message: "Account already registered. Please use recovery flow.".into(),
        }));
    }

    // Rate limit: max 3 OTP sends per email per hour
    let recent_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM email_verifications
         WHERE email = $1 AND created_at > NOW() - INTERVAL '1 hour'"
    )
    .bind(&body.email)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    if recent_count >= 3 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Generate 6-digit OTP
    let otp = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let otp_hash = Sha256::digest(otp.as_bytes());
    let expires_at = Utc::now() + chrono::Duration::minutes(10);

    // Invalidate previous pending verifications for this email
    let _ = sqlx::query(
        "DELETE FROM email_verifications WHERE email = $1 AND NOT verified"
    )
    .bind(&body.email)
    .execute(db)
    .await;

    // Store verification record
    sqlx::query(
        "INSERT INTO email_verifications (email, otp_hash, expires_at) VALUES ($1, $2, $3)"
    )
    .bind(&body.email)
    .bind(otp_hash.as_slice())
    .bind(expires_at)
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store email verification: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Send OTP via email (AWS SES)
    if let Some(email_service) = &state.email {
        email_service.send_otp(&body.email, &otp).await.map_err(|e| {
            tracing::error!("Failed to send email OTP: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    } else {
        tracing::warn!("⚠️  [NO SES] Email OTP not sent for {} (set SES_FROM_ADDRESS to enable)", body.email);
        #[cfg(debug_assertions)]
        tracing::debug!("DEV ONLY — OTP: {}", otp);
    }

    Ok(Json(SendEmailOtpResponse {
        sent: true,
        is_registered,
        message: format!("Verification code sent to {}", body.email),
    }))
}

#[derive(Deserialize)]
struct RegisterRequest {
    email: String,
    otp: String,
    device_id: String,
    #[serde(default)]
    force: bool,
    /// SHA-256 hash of the user's backup shard (hex). Required when force=true.
    backup_shard_hash: Option<String>,
    /// Device's hardware public key (hex). For P-256: SEC1 (33-byte compressed
    /// or 65-byte uncompressed). For RSA: X.509 SubjectPublicKeyInfo DER.
    /// Registered so challenge-response login can verify the device holds the
    /// matching private key.
    device_pubkey: Option<String>,
    /// Algorithm of `device_pubkey`: "p256" (iOS Secure Enclave) or "rsa"
    /// (Android StrongBox). Required when `device_pubkey` is present.
    device_pubkey_alg: Option<String>,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    refresh_token: String,
    expires_in: usize,
    token_type: &'static str,
    user_id: String,
}

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Deserialize)]
struct LogoutRequest {
    all_devices: Option<bool>,
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    // Review account (env-gated, one exact address): skip the emailed-OTP check
    // entirely so App Review can always re-register on a fresh install, regardless
    // of OTP delivery / expiry / rate limit. Everything below treats
    // `verification_id` as optional; no OTP row is consumed for this account.
    let is_review = review_bypass_otp_for(&body.email).is_some();

    // Verify email OTP
    let otp_hash = Sha256::digest(body.otp.as_bytes());
    let verification: Option<(uuid::Uuid, i32)> = sqlx::query_as(
        "UPDATE email_verifications SET attempts = COALESCE(attempts, 0) + 1
         WHERE email = $1 AND otp_hash = $2 AND NOT verified AND expires_at > NOW()
         RETURNING id, attempts"
    )
    .bind(&body.email)
    .bind(otp_hash.as_slice())
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!("register: OTP verification query failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // If no matching OTP, check if there's a pending one that's been brute-forced.
    // The review account bypasses this entirely (no OTP required).
    if verification.is_none() && !is_review {
        // Increment attempt counter on the latest pending verification for this email
        let _: Option<(i32,)> = sqlx::query_as(
            "UPDATE email_verifications SET attempts = COALESCE(attempts, 0) + 1
             WHERE email = $1 AND NOT verified AND expires_at > NOW()
             RETURNING attempts"
        )
        .bind(&body.email)
        .fetch_optional(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Check if locked out (5+ failed attempts)
        let locked: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM email_verifications
             WHERE email = $1 AND NOT verified AND expires_at > NOW() AND attempts >= 5)"
        )
        .bind(&body.email)
        .fetch_one(db)
        .await
        .unwrap_or(false);

        if locked {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Review account has no OTP row; `verification_id` stays None and no lockout applies.
    let (verification_id, attempts): (Option<uuid::Uuid>, i32) = match verification {
        Some((id, attempts)) => (Some(id), attempts),
        None => (None, 0),
    };

    // SECURITY (F-011): lock on the 5th failed guess (>=), not the 6th.
    if attempts >= 5 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Decode and validate the device's hardware public key (F-001 challenge-response
    // login). Its algorithm is P-256 (iOS Secure Enclave) or RSA (Android StrongBox);
    // login later verifies signatures against this key + `device_pubkey_alg`. Reject
    // malformed keys or an unknown algorithm so logins can actually verify later.
    let device_pubkey_bytes: Option<Vec<u8>> = match &body.device_pubkey {
        Some(pk_hex) => {
            let alg = body.device_pubkey_alg.as_deref().ok_or(StatusCode::BAD_REQUEST)?;
            let bytes = hex::decode(pk_hex.trim_start_matches("0x"))
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            match alg {
                "p256" => {
                    p256::ecdsa::VerifyingKey::from_sec1_bytes(&bytes)
                        .map_err(|_| StatusCode::BAD_REQUEST)?;
                }
                "rsa" => {
                    use rsa::pkcs8::DecodePublicKey;
                    rsa::RsaPublicKey::from_public_key_der(&bytes)
                        .map_err(|_| StatusCode::BAD_REQUEST)?;
                }
                _ => return Err(StatusCode::BAD_REQUEST),
            }
            Some(bytes)
        }
        None => None,
    };

    // NOTE: OTP is intentionally NOT consumed here. It is marked verified only
    // after all pre-conditions pass (backup_shard_hash check for force
    // re-register, device_pubkey validation), so that a recoverable failure
    // (e.g. 428 needing a hash back-fill) lets the client retry with the SAME
    // OTP instead of being stranded with a spent code.

    // Check if user exists and whether they have a completed wallet
    let existing: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT id FROM users WHERE email = $1"
    )
    .bind(&body.email)
    .fetch_optional(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_id = if let Some((existing_id,)) = existing {
        // User exists — check if they have a completed wallet (shard + active wallet)
        let has_shard: bool = sqlx::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM shard_metadata WHERE user_id = $1 AND location = 'server' LIMIT 1"
        )
        .bind(existing_id)
        .fetch_optional(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

        let has_active_wallet: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM wallets WHERE user_id = $1 AND status = 'active')"
        )
        .bind(existing_id)
        .fetch_one(db)
        .await
        .unwrap_or(false);

        // Orphaned DKG state: shard exists but no wallet — clean up and allow re-registration
        if has_shard && !has_active_wallet {
            sqlx::query("DELETE FROM shard_metadata WHERE user_id = $1")
                .bind(existing_id)
                .execute(db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            tracing::info!("Cleaned up orphaned shards for user {}", existing_id);
        }

        let has_wallet = has_shard && has_active_wallet;

        // Review account may re-register on reinstall without holding the backup
        // shard: treat it like a forced reset (env-gated, one address only).
        // `is_review` is computed once at the top of `register`.
        let force_reset = body.force || is_review;

        if has_wallet && !force_reset {
            return Err(StatusCode::CONFLICT);
        }

        if has_wallet && force_reset {
            // Normal force re-register requires proof the client holds the backup
            // shard (SHA-256(backup_shard)). The review account skips this check.
            if !is_review {
                let backup_hash_hex = body.backup_shard_hash.as_deref()
                    .ok_or(StatusCode::PRECONDITION_REQUIRED)?;

                // Verify the backup shard hash matches what we have on record
                let stored_backup_hash: Option<(Option<Vec<u8>>,)> = sqlx::query_as(
                    "SELECT backup_shard_hash FROM shard_metadata
                     WHERE user_id = $1 AND location = 'server' LIMIT 1"
                )
                .bind(existing_id)
                .fetch_optional(db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                match stored_backup_hash {
                    Some((Some(stored_hash),)) => {
                        let provided_hash = hex::decode(backup_hash_hex)
                            .map_err(|_| StatusCode::BAD_REQUEST)?;
                        if provided_hash != stored_hash {
                            return Err(StatusCode::FORBIDDEN);
                        }
                    }
                    _ => {
                        // No backup hash on record — cannot verify, reject force re-register
                        return Err(StatusCode::PRECONDITION_REQUIRED);
                    }
                }
            }

            // Archive shards instead of deleting. Note: shard_metadata has no
            // public_key column (only wallets does), so archive it as NULL.
            sqlx::query(
                "INSERT INTO shard_metadata_archive
                    (original_id, user_id, location, party_index, encrypted_shard, nonce, public_key, archive_reason, created_at)
                 SELECT id, user_id, location, party_index, encrypted_shard, nonce, NULL, 'force_reregister', created_at
                 FROM shard_metadata WHERE user_id = $1"
            )
            .bind(existing_id)
            .execute(db)
            .await
            .map_err(|e| {
                tracing::error!("register: archive shards failed: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // Archive old wallets
            sqlx::query(
                "UPDATE wallets SET status = 'archived' WHERE user_id = $1 AND status = 'active'"
            )
            .bind(existing_id)
            .execute(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Now remove active shards
            sqlx::query("DELETE FROM shard_metadata WHERE user_id = $1")
                .bind(existing_id)
                .execute(db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }

        // Update device_id and reuse the user.
        // Set the device public key + algorithm only when provided (don't clobber an
        // existing key with NULL).
        sqlx::query(
            "UPDATE users SET device_id = $1, public_key = COALESCE($2, public_key), \
             device_pubkey_alg = COALESCE($3, device_pubkey_alg) WHERE id = $4"
        )
            .bind(&body.device_id)
            .bind(device_pubkey_bytes.as_deref())
            .bind(body.device_pubkey_alg.as_deref())
            .bind(existing_id)
            .execute(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        existing_id
    } else {
        // New user
        let new_id = uuid::Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, device_id, public_key, device_pubkey_alg) VALUES ($1, $2, $3, $4, $5)")
            .bind(new_id)
            .bind(&body.email)
            .bind(&body.device_id)
            .bind(device_pubkey_bytes.as_deref())
            .bind(body.device_pubkey_alg.as_deref())
            .execute(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        new_id
    };

    // All pre-conditions passed (OTP matched, backup-hash verified for force
    // re-register, device key valid). Consume the OTP now — deferring to this
    // point means a 428/400 above leaves the code reusable for a retry.
    // The review account has no OTP row to consume (verification_id is None).
    if let Some(vid) = verification_id {
        sqlx::query("UPDATE email_verifications SET verified = TRUE WHERE id = $1")
            .bind(vid)
            .execute(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Register the device's hardware public key + algorithm (if supplied) so
    // challenge-response login can later verify ownership of the device key.
    // iOS Secure Enclave → P-256 (SEC1); Android StrongBox → RSA (SPKI DER).
    if let Some(pubkey_hex) = &body.device_pubkey {
        let alg = body
            .device_pubkey_alg
            .as_deref()
            .ok_or(StatusCode::BAD_REQUEST)?;
        let pubkey_bytes = hex::decode(pubkey_hex.trim_start_matches("0x"))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        // Validate per algorithm; full cryptographic validity is checked at login.
        match alg {
            // SEC1: 33-byte compressed or 65-byte uncompressed point.
            "p256" => {
                if pubkey_bytes.len() != 33 && pubkey_bytes.len() != 65 {
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
            // SPKI/X.509 DER — variable length; reject only the obviously empty.
            "rsa" => {
                if pubkey_bytes.is_empty() {
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
            _ => return Err(StatusCode::BAD_REQUEST),
        }
        sqlx::query("UPDATE users SET public_key = $1, device_pubkey_alg = $2 WHERE id = $3")
            .bind(&pubkey_bytes)
            .bind(alg)
            .bind(user_id)
            .execute(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let token_pair = issue_token_pair(&user_id.to_string(), &body.device_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = state
        .audit_logger
        .log_with_details(
            user_id,
            "auth.register",
            AuditResult::Success,
            None,
            None,
            None,
            None,
            Some(serde_json::json!({ "device_id": body.device_id })),
        )
        .await;

    Ok(Json(AuthResponse {
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
        token_type: token_pair.token_type,
        user_id: user_id.to_string(),
    }))
}

#[derive(Deserialize)]
struct ChallengeRequest {
    device_id: String,
}

#[derive(Serialize)]
struct ChallengeResponse {
    /// Random nonce (hex) the device must sign.
    challenge: String,
    expires_in: u64,
}

/// POST /api/v1/auth/challenge
///
/// Issues a random nonce bound to a device. The device signs it with its
/// registered secp256k1 key and presents the signature to `/login`.
async fn request_challenge(
    State(state): State<AppState>,
    Json(body): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    use rand::RngCore;
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let mut nonce = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut nonce);

    sqlx::query("INSERT INTO login_challenges (device_id, challenge) VALUES ($1, $2)")
        .bind(&body.device_id)
        .bind(&nonce[..])
        .execute(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ChallengeResponse {
        challenge: hex::encode(nonce),
        expires_in: 120,
    }))
}

#[derive(Deserialize)]
struct LoginRequest {
    device_id: String,
    /// hex, must match an unconsumed, unexpired challenge for this device.
    challenge: String,
    /// hex signature over the challenge bytes, produced by the device's
    /// hardware key. Encoding depends on the registered algorithm:
    /// P-256 → X9.62/DER ECDSA; RSA → PKCS#1 v1.5.
    signature: String,
}

/// Verify a device signature over `msg` using the registered public key,
/// dispatching on the key algorithm recorded at registration.
///
/// The device signs the raw challenge bytes; the underlying hardware (iOS
/// Secure Enclave `.ecdsaSignatureMessageX962SHA256`, Android StrongBox
/// `SHA256withRSA`) hashes with SHA-256 internally, so the server verifies
/// against `SHA-256(challenge)`.
///
/// - `p256`: SEC1 public key (33-byte compressed or 65-byte uncompressed),
///   DER-encoded ECDSA signature.
/// - `rsa`: SPKI (X.509 `SubjectPublicKeyInfo`) public key, PKCS#1 v1.5
///   signature over SHA-256.
fn verify_device_signature(
    alg: &str,
    pubkey: &[u8],
    msg: &[u8],
    sig_hex: &str,
) -> Result<(), String> {
    let sig_bytes = hex::decode(sig_hex.trim_start_matches("0x"))
        .map_err(|_| "signature not hex".to_string())?;

    match alg {
        "p256" => {
            use p256::ecdsa::signature::hazmat::PrehashVerifier;
            use p256::ecdsa::{Signature, VerifyingKey};

            let vk = VerifyingKey::from_sec1_bytes(pubkey)
                .map_err(|e| format!("invalid P-256 public key: {}", e))?;
            // Accept either DER or 64-byte compact encoding.
            let sig = Signature::from_der(&sig_bytes)
                .or_else(|_| Signature::from_slice(&sig_bytes))
                .map_err(|_| "malformed P-256 signature".to_string())?;
            let digest = Sha256::digest(msg);
            vk.verify_prehash(digest.as_slice(), &sig)
                .map_err(|_| "P-256 signature verification failed".to_string())
        }
        "rsa" => {
            use rsa::pkcs1v15::{Signature, VerifyingKey};
            use rsa::pkcs8::DecodePublicKey;
            use rsa::sha2::Sha256 as RsaSha256;
            use rsa::signature::Verifier;

            let public_key = rsa::RsaPublicKey::from_public_key_der(pubkey)
                .map_err(|e| format!("invalid RSA public key: {}", e))?;
            let vk: VerifyingKey<RsaSha256> = VerifyingKey::new(public_key);
            let sig = Signature::try_from(sig_bytes.as_slice())
                .map_err(|_| "malformed RSA signature".to_string())?;
            // RSA verifier hashes the message itself with SHA-256.
            vk.verify(msg, &sig)
                .map_err(|_| "RSA signature verification failed".to_string())
        }
        other => Err(format!("unsupported device key algorithm: {}", other)),
    }
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let challenge_bytes = hex::decode(body.challenge.trim_start_matches("0x"))
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Atomically consume a valid, unexpired challenge for this device.
    let consumed = sqlx::query(
        "UPDATE login_challenges SET consumed = TRUE
         WHERE device_id = $1 AND challenge = $2 AND consumed = FALSE AND expires_at > NOW()",
    )
    .bind(&body.device_id)
    .bind(&challenge_bytes)
    .execute(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if consumed.rows_affected() == 0 {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Load the device's registered public key + algorithm and verify the signature.
    let row: Option<(uuid::Uuid, Option<Vec<u8>>, Option<String>)> = sqlx::query_as(
        "SELECT id, public_key, device_pubkey_alg FROM users WHERE device_id = $1",
    )
    .bind(&body.device_id)
    .fetch_optional(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (user_id, pubkey, alg) = match row {
        Some((id, Some(pk), Some(alg))) if !pk.is_empty() => (id, pk, alg),
        // No user, or no registered device key/alg — cannot do challenge-response.
        _ => {
            let _ = state
                .audit_logger
                .log_with_details(
                    uuid::Uuid::nil(),
                    "auth.login",
                    AuditResult::Denied,
                    None,
                    None,
                    None,
                    None,
                    Some(serde_json::json!({ "device_id": body.device_id, "reason": "no device key" })),
                )
                .await;
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    if let Err(e) = verify_device_signature(&alg, &pubkey, &challenge_bytes, &body.signature) {
        let _ = state
            .audit_logger
            .log_with_details(
                user_id,
                "auth.login",
                AuditResult::Denied,
                None,
                None,
                None,
                None,
                Some(serde_json::json!({ "device_id": body.device_id, "reason": e })),
            )
            .await;
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token_pair = issue_token_pair(&user_id.to_string(), &body.device_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log - login success
    let _ = state
        .audit_logger
        .log_with_details(
            user_id,
            "auth.login",
            AuditResult::Success,
            None,
            None,
            None,
            None,
            Some(serde_json::json!({ "device_id": body.device_id })),
        )
        .await;

    Ok(Json(AuthResponse {
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
        token_type: token_pair.token_type,
        user_id: user_id.to_string(),
    }))
}

async fn refresh(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let db = state
        .db
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Device id MUST come from an independent source (request header), not from
    // the refresh token itself, otherwise the binding check is a no-op (F-011).
    let presented_device_id = headers
        .get("X-Device-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;

    let claims = verify_token_unchecked(&body.refresh_token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let token_pair = refresh_access_token(&db, &body.refresh_token, presented_device_id).await?;

    let user_id = claims.sub.parse().unwrap_or(uuid::Uuid::nil());
    let _ = state
        .audit_logger
        .log_with_details(
            user_id,
            "auth.refresh",
            AuditResult::Success,
            None,
            None,
            None,
            None,
            Some(serde_json::json!({ "device_id": claims.device_id })),
        )
        .await;

    Ok(Json(AuthResponse {
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
        token_type: token_pair.token_type,
        user_id: claims.sub,
    }))
}

async fn logout(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Json(body): Json<LogoutRequest>,
) -> Result<StatusCode, StatusCode> {
    let db = state
        .db
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let user_id = claims.0.sub.parse().unwrap_or(uuid::Uuid::nil());

    blacklist_token(
        &db,
        &claims.0.jti,
        &claims.0.sub,
        claims.0.exp,
        Some("User logout".to_string()),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = state
        .audit_logger
        .log_with_details(
            user_id,
            "auth.logout",
            AuditResult::Success,
            None,
            None,
            None,
            None,
            Some(serde_json::json!({
                "device_id": claims.0.device_id,
                "all_devices": body.all_devices.unwrap_or(false)
            })),
        )
        .await;

    Ok(StatusCode::OK)
}

async fn session_info(
    claims: Option<axum::Extension<Claims>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let claims = claims.ok_or(StatusCode::UNAUTHORIZED)?.0;
    Ok(Json(serde_json::json!({
        "user_id": claims.sub,
        "device_id": claims.device_id,
        "expires_at": claims.exp,
    })))
}

#[derive(Serialize)]
struct WsTicketResponse {
    /// Opaque single-use ticket. Pass to the WS handler as `?ticket=`.
    ticket: String,
    /// Seconds until the ticket expires.
    expires_in: u64,
}

/// POST /api/v1/auth/ws-ticket — exchange a valid JWT for a short-lived,
/// single-use WebSocket ticket (F-010).
///
/// The MPC WebSocket cannot send Authorization headers, so previously the raw
/// JWT was passed in the query string (`?token=`), exposing it in logs/proxies
/// and bypassing blacklist checks. This endpoint validates the bearer JWT
/// (signature + expiry + blacklist), then stores a random 32-byte ticket in the
/// DB with a 30-second TTL keyed to the user_id. The WS handler consumes it.
async fn ws_ticket(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<WsTicketResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    // Extract + verify the bearer JWT (mirrors require_auth, incl. blacklist).
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = verify_token_unchecked(token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    if crate::middleware::auth::is_token_blacklisted(db, &claims.jti)
        .await
        .unwrap_or(false)
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Generate an opaque random ticket (32 bytes hex).
    let mut raw = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut raw[..]);
    let ticket = hex::encode(raw);
    let expires_in: u64 = 30;
    let expires_at = Utc::now() + chrono::Duration::seconds(expires_in as i64);

    sqlx::query(
        "INSERT INTO ws_tickets (ticket, user_id, device_id, expires_at) VALUES ($1, $2, $3, $4)"
    )
    .bind(&ticket)
    .bind(user_id)
    .bind(&claims.device_id)
    .bind(expires_at)
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store ws ticket: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(WsTicketResponse { ticket, expires_in }))
}

async fn audit_log(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<Vec<AuditLog>>, StatusCode> {
    let user_id: uuid::Uuid = claims
        .0
        .sub
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let logs = if let Some(action) = &query.action {
        state
            .audit_logger
            .get_logs_by_action(user_id, action, limit)
            .await
    } else {
        state
            .audit_logger
            .get_user_logs(user_id, limit, offset)
            .await
    };

    match logs {
        Ok(logs) => Ok(Json(logs)),
        Err(e) => {
            tracing::error!("Failed to fetch audit logs: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ─── Wallet Recovery Endpoints ───────────────────────────────────────────────

#[derive(Deserialize)]
struct InitiateRecoveryRequest {
    email: String,
}

#[derive(Serialize)]
struct InitiateRecoveryResponse {
    recovery_session_id: String,
    otp_sent: bool,
    message: String,
}

/// Initiate wallet recovery process.
/// Sends OTP to user's email and creates a recovery session.
/// Returns a consistent response regardless of whether the user exists (prevents enumeration).
async fn initiate_recovery(
    State(state): State<AppState>,
    Json(body): Json<InitiateRecoveryRequest>,
) -> Result<Json<InitiateRecoveryResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    // Always generate a session ID (returned even if user doesn't exist to prevent enumeration)
    let recovery_session_id = uuid::Uuid::new_v4();

    // Verify user exists and has a server shard
    let user_row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT u.id FROM users u
         INNER JOIN shard_metadata s ON s.user_id = u.id AND s.location = 'server'
         WHERE u.email = $1
         LIMIT 1"
    )
    .bind(&body.email)
    .fetch_optional(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // If user doesn't exist or has no shard, return success-like response (anti-enumeration)
    let Some((user_id,)) = user_row else {
        return Ok(Json(InitiateRecoveryResponse {
            recovery_session_id: recovery_session_id.to_string(),
            otp_sent: true,
            message: "If an account exists, a recovery code was sent to your email.".into(),
        }));
    };

    // Check for 30-minute cooldown after locked sessions
    let has_recent_lock: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM recovery_sessions
         WHERE user_id = $1 AND status = 'locked'
         AND created_at > NOW() - INTERVAL '30 minutes')"
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(false);

    if has_recent_lock {
        // Return 423 Locked — user must wait before retrying
        return Err(StatusCode::LOCKED);
    }

    // Invalidate all previous pending recovery sessions for this user
    let _ = sqlx::query(
        "UPDATE recovery_sessions SET status = 'expired' WHERE user_id = $1 AND status = 'pending'"
    )
    .bind(user_id)
    .execute(db)
    .await;

    // Generate OTP (6-digit code)
    let otp = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let expires_at = Utc::now() + chrono::Duration::minutes(10);

    // Store recovery session with attempt counter
    sqlx::query(
        "INSERT INTO recovery_sessions (id, user_id, otp_hash, expires_at, status, attempts)
         VALUES ($1, $2, $3, $4, 'pending', 0)"
    )
    .bind(recovery_session_id)
    .bind(user_id)
    .bind(sha2::Sha256::digest(otp.as_bytes()).as_slice())
    .bind(expires_at)
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create recovery session: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(email_service) = &state.email {
        email_service.send_otp(&body.email, &otp).await.map_err(|e| {
            tracing::error!("Failed to send recovery OTP email: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    } else {
        tracing::warn!("⚠️  [NO SES] Recovery OTP not sent for {} (set SES_FROM_ADDRESS to enable)", body.email);
        #[cfg(debug_assertions)]
        tracing::debug!("DEV ONLY — OTP: {}", otp);
    }
    tracing::info!("Recovery initiated for user {} (session: {})", user_id, recovery_session_id);

    // Audit log
    let _ = state
        .audit_logger
        .log_with_details(
            user_id,
            "auth.recovery_initiated",
            AuditResult::Success,
            None,
            None,
            None,
            None,
            Some(serde_json::json!({ "email": body.email })),
        )
        .await;

    Ok(Json(InitiateRecoveryResponse {
        recovery_session_id: recovery_session_id.to_string(),
        otp_sent: true,
        message: "If an account exists, a recovery code was sent to your email.".into(),
    }))
}

#[derive(Deserialize)]
struct VerifyRecoveryOtpRequest {
    recovery_session_id: String,
    otp: String,
    device_id: String,
}

#[derive(Serialize)]
struct VerifyRecoveryOtpResponse {
    token: String,
    refresh_token: String,
    expires_in: usize,
    token_type: &'static str,
    user_id: String,
    public_key_hex: String,
    server_reshare_messages_json: Vec<String>,
    /// Feldman commitment: G * (lambda_1 * s_1), compressed SEC1 hex.
    /// Client verifies: server_commitment + G*(lambda_2 * backup_shard) == PublicKey.
    server_commitment_hex: String,
}

/// Verify recovery OTP and return server's reshare contribution.
/// This starts the recovery protocol where the server (Party 1) and backup (Party 2)
/// collaborate to reconstruct the device shard (Party 0).
async fn verify_recovery_otp(
    State(state): State<AppState>,
    Json(body): Json<VerifyRecoveryOtpRequest>,
) -> Result<Json<VerifyRecoveryOtpResponse>, StatusCode> {
    let db = state
        .require_db()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let recovery_session_id = uuid::Uuid::parse_str(&body.recovery_session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Fetch recovery session with atomic attempt increment
    let session: Option<(uuid::Uuid, Vec<u8>, chrono::DateTime<Utc>, String, i32)> = sqlx::query_as(
        "UPDATE recovery_sessions SET attempts = COALESCE(attempts, 0) + 1
         WHERE id = $1
         RETURNING user_id, otp_hash, expires_at, status, attempts"
    )
    .bind(recovery_session_id)
    .fetch_optional(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (user_id, otp_hash, expires_at, status, attempts) =
        session.ok_or(StatusCode::NOT_FOUND)?;

    // Check expiration
    if Utc::now() > expires_at {
        return Err(StatusCode::GONE);
    }

    // Check status (must be pending — blocks reuse after success)
    if status != "pending" {
        return Err(StatusCode::CONFLICT);
    }

    // Brute-force protection: lock on the 5th failed attempt (F-011: >= not >)
    if attempts >= 5 {
        let _ = sqlx::query("UPDATE recovery_sessions SET status = 'locked' WHERE id = $1")
            .bind(recovery_session_id)
            .execute(db)
            .await;
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Verify OTP with constant-time comparison
    let provided_hash = sha2::Sha256::digest(body.otp.as_bytes());
    let otp_valid = provided_hash.as_slice().len() == otp_hash.len()
        && provided_hash
            .as_slice()
            .iter()
            .zip(otp_hash.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0;

    if !otp_valid {
        // Audit log - failed verification
        let _ = state
            .audit_logger
            .log_with_details(
                user_id,
                "auth.recovery_otp_failed",
                AuditResult::Denied,
                None,
                None,
                None,
                None,
                None,
            )
            .await;

        return Err(StatusCode::UNAUTHORIZED);
    }

    // Mark session as completed (single-use, prevents replay)
    sqlx::query("UPDATE recovery_sessions SET status = 'completed' WHERE id = $1")
        .bind(recovery_session_id)
        .execute(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fetch server shard and public key
    let shard_row: (Vec<u8>, Vec<u8>, i16) = sqlx::query_as(
        "SELECT encrypted_shard, nonce, party_index
         FROM shard_metadata
         WHERE user_id = $1 AND location = 'server'"
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get MPC participant service
    let mpc_participant = state
        .mpc_participant
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Create recovery reshare MPC session in DB.
    // Participants: Server (1) + Backup (2). Target: Device (0).
    let session_id = uuid::Uuid::new_v4();
    let parties = vec![1i16, 2i16]; // Server + Backup (the available old-share holders)
    let threshold = 2i16;

    sqlx::query(
        "INSERT INTO mpc_sessions (id, user_id, session_type, parties, threshold, status, current_round)
         VALUES ($1, $2, 'reshare', $3, $4, 'active', 0)"
    )
    .bind(session_id)
    .bind(user_id)
    .bind(&parties)
    .bind(threshold)
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create reshare session: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Initialize server's reshare protocol in recovery mode.
    // The server uses its shard (Party 1) with Lagrange correction to produce
    // evaluations that will reconstruct Party 0's shard when combined with backup's contribution.
    if let Err(e) = mpc_participant
        .on_session_created(session_id, user_id, "reshare", &parties, threshold, None)
        .await
    {
        tracing::error!("Server reshare init failed for session {}: {}", session_id, e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Fetch server's Round 1 reshare messages (addressed to Party 0 — the target)
    let messages: Vec<(i16, i16, i16, Vec<u8>)> = sqlx::query_as(
        "SELECT from_party, to_party, round, payload
         FROM mpc_messages
         WHERE session_id = $1 AND from_party = 1 AND round = 1
         ORDER BY created_at ASC"
    )
    .bind(session_id)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch reshare messages: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Serialize messages as JSON strings matching ProtocolMessage struct
    let server_reshare_messages_json: Vec<String> = messages
        .into_iter()
        .map(|(from, to, round, payload)| {
            serde_json::to_string(&serde_json::json!({
                "session_id": session_id.to_string(),
                "from": from,
                "to": to,
                "round": round,
                "payload": payload
            }))
            .unwrap_or_default()
        })
        .collect();

    // Get public key from wallets table (use the first active wallet for this user)
    let public_key: Vec<u8> = sqlx::query_scalar(
        "SELECT public_key FROM wallets WHERE user_id = $1 AND status = 'active' ORDER BY created_at ASC LIMIT 1"
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch public key: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or_else(|| {
        tracing::warn!("No active wallet found for user {}", user_id);
        StatusCode::NOT_FOUND
    })?;

    let public_key_hex = hex::encode(&public_key);

    // Compute Feldman commitment G*(lambda_1 * s_1) for client-side backup shard verification
    let server_commitment_hex = match mpc_participant.compute_recovery_commitment(user_id).await {
        Ok(bytes) => hex::encode(&bytes),
        Err(e) => {
            tracing::error!("Failed to compute recovery commitment: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Issue JWT token pair
    let token_pair = issue_token_pair(&user_id.to_string(), &body.device_id)
        .map_err(|e| {
            tracing::error!("Failed to issue token pair: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Audit log - successful recovery
    let _ = state
        .audit_logger
        .log_with_details(
            user_id,
            "auth.recovery_success",
            AuditResult::Success,
            None,
            None,
            None,
            None,
            Some(serde_json::json!({
                "device_id": body.device_id,
                "reshare_session_id": session_id.to_string()
            })),
        )
        .await;

    tracing::info!(
        "Recovery OTP verified for user {}, reshare session {} initiated",
        user_id,
        session_id
    );

    Ok(Json(VerifyRecoveryOtpResponse {
        token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
        token_type: token_pair.token_type,
        user_id: user_id.to_string(),
        public_key_hex,
        server_reshare_messages_json,
        server_commitment_hex,
    }))
}

#[cfg(test)]
mod challenge_login_tests {
    use super::*;

    // ---- P-256 (iOS Secure Enclave) ----
    mod p256_tests {
        use super::*;
        use p256::ecdsa::signature::hazmat::PrehashSigner;
        use p256::ecdsa::{Signature, SigningKey};

        fn key(seed: u8) -> SigningKey {
            SigningKey::from_slice(&[seed; 32]).unwrap()
        }

        fn sign(sk: &SigningKey, challenge: &[u8]) -> String {
            let digest = Sha256::digest(challenge);
            let sig: Signature = sk.sign_prehash(digest.as_slice()).expect("sign");
            hex::encode(sig.to_der().as_bytes())
        }

        #[test]
        fn verifies_valid_signature() {
            let sk = key(7);
            let pk = sk.verifying_key().to_sec1_bytes().to_vec();
            let challenge = [42u8; 32];
            let sig = sign(&sk, &challenge);
            assert!(verify_device_signature("p256", &pk, &challenge, &sig).is_ok());
        }

        #[test]
        fn rejects_tampered_challenge() {
            let sk = key(7);
            let pk = sk.verifying_key().to_sec1_bytes().to_vec();
            let sig = sign(&sk, &[42u8; 32]);
            assert!(verify_device_signature("p256", &pk, &[43u8; 32], &sig).is_err());
        }

        #[test]
        fn rejects_wrong_key() {
            let sk = key(7);
            let other_pk = key(8).verifying_key().to_sec1_bytes().to_vec();
            let challenge = [42u8; 32];
            let sig = sign(&sk, &challenge);
            assert!(verify_device_signature("p256", &other_pk, &challenge, &sig).is_err());
        }

        #[test]
        fn rejects_garbage_signature() {
            let pk = key(7).verifying_key().to_sec1_bytes().to_vec();
            assert!(verify_device_signature("p256", &pk, &[42u8; 32], "not-hex").is_err());
            assert!(verify_device_signature("p256", &pk, &[42u8; 32], "00ff").is_err());
        }
    }

    // ---- RSA (Android StrongBox, SHA256withRSA / PKCS#1 v1.5) ----
    mod rsa_tests {
        use super::*;
        use rsa::pkcs1v15::SigningKey;
        use rsa::pkcs8::EncodePublicKey;
        use rsa::sha2::Sha256 as RsaSha256;
        use rsa::signature::{SignatureEncoding, Signer};
        use rsa::RsaPrivateKey;

        // A small (1024-bit) key keeps the test fast; production keys are 2048+.
        fn keypair(seed: u64) -> (SigningKey<RsaSha256>, Vec<u8>) {
            use rand::SeedableRng;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let priv_key = RsaPrivateKey::new(&mut rng, 1024).expect("rsa keygen");
            let spki = priv_key
                .to_public_key()
                .to_public_key_der()
                .unwrap()
                .as_bytes()
                .to_vec();
            (SigningKey::<RsaSha256>::new(priv_key), spki)
        }

        #[test]
        fn verifies_valid_signature() {
            let (sk, spki) = keypair(1);
            let challenge = [42u8; 32];
            let sig = hex::encode(sk.sign(&challenge).to_bytes());
            assert!(verify_device_signature("rsa", &spki, &challenge, &sig).is_ok());
        }

        #[test]
        fn rejects_tampered_challenge() {
            let (sk, spki) = keypair(1);
            let sig = hex::encode(sk.sign(&[42u8; 32]).to_bytes());
            assert!(verify_device_signature("rsa", &spki, &[43u8; 32], &sig).is_err());
        }

        #[test]
        fn rejects_wrong_key() {
            let (sk, _) = keypair(1);
            let (_, other_spki) = keypair(2);
            let challenge = [42u8; 32];
            let sig = hex::encode(sk.sign(&challenge).to_bytes());
            assert!(verify_device_signature("rsa", &other_spki, &challenge, &sig).is_err());
        }
    }

    #[test]
    fn rejects_unknown_algorithm() {
        assert!(verify_device_signature("ed25519", &[1, 2, 3], &[42u8; 32], "00ff").is_err());
    }
}
