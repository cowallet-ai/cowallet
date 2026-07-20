use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// JWT token type. Access tokens authorize API calls; refresh tokens may ONLY
/// be exchanged at the refresh endpoint. Keeping them distinct (F-014) prevents
/// a stolen 7-day refresh token from being replayed as a 7-day access token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub jti: String, // token ID (for blacklisting)
    pub device_id: String,
    #[serde(default = "default_token_type")]
    pub token_type: TokenType,
    pub exp: usize,
    pub iat: usize,
}

/// Legacy tokens minted before F-014 carried no `token_type`. Treat them as
/// access tokens so they cannot be silently upgraded to refresh privileges.
fn default_token_type() -> TokenType {
    TokenType::Access
}

impl Claims {
    pub fn new(user_id: &str, device_id: &str, ttl_secs: u64) -> Self {
        Self::new_typed(user_id, device_id, ttl_secs, TokenType::Access)
    }

    pub fn new_typed(user_id: &str, device_id: &str, ttl_secs: u64, token_type: TokenType) -> Self {
        let now = chrono::Utc::now().timestamp() as usize;
        Self {
            sub: user_id.to_string(),
            jti: Uuid::new_v4().to_string(),
            device_id: device_id.to_string(),
            token_type,
            iat: now,
            exp: now + ttl_secs as usize,
        }
    }

    /// Create refresh token claims (longer TTL)
    pub fn new_refresh(user_id: &str, device_id: &str) -> Self {
        Self::new_typed(user_id, device_id, 86400 * 7, TokenType::Refresh) // 7 days
    }
}

/// Response for successful authentication with token pair
#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: usize,
    pub token_type: &'static str,
}

fn jwt_secret() -> Vec<u8> {
    let secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET environment variable must be set");
    // HS256 security depends on secret strength; a short secret is brute-forceable.
    // CLAUDE.md mandates >= 32 chars.
    assert!(
        secret.len() >= 32,
        "JWT_SECRET must be at least 32 characters (got {})",
        secret.len()
    );
    secret.into_bytes()
}

/// Issue a pair of access token (24h) and refresh token (7d)
pub fn issue_token_pair(user_id: &str, device_id: &str) -> Result<TokenPair, jsonwebtoken::errors::Error> {
    let access_claims = Claims::new_typed(user_id, device_id, 86400, TokenType::Access); // 24h
    let refresh_claims = Claims::new_refresh(user_id, device_id);

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(&jwt_secret()),
    )?;

    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(&jwt_secret()),
    )?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        expires_in: 86400,
        token_type: "Bearer",
    })
}

/// Verify a JWT token without checking blacklist (for refresh flow)
pub fn verify_token_unchecked(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&jwt_secret()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

/// Check if a token is blacklisted in the database
pub async fn is_token_blacklisted(db: &PgPool, jti: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM jwt_blacklist WHERE token_id = $1)"
    )
    .bind(Uuid::parse_str(jti).unwrap_or(Uuid::nil()))
    .fetch_one(db)
    .await?;

    Ok(result)
}

/// Add a token to the blacklist (logout/revocation)
pub async fn blacklist_token(
    db: &PgPool,
    jti: &str,
    user_id: &str,
    exp: usize,
    reason: Option<String>,
) -> Result<(), sqlx::Error> {
    let jti_uuid = Uuid::parse_str(jti).unwrap_or_else(|_| Uuid::nil());
    let user_uuid = Uuid::parse_str(user_id).unwrap_or(Uuid::nil());
    let exp_time = chrono::NaiveDateTime::from_timestamp_opt(exp as i64, 0)
        .unwrap_or_else(|| chrono::Utc::now().naive_utc())
        .and_utc();

    sqlx::query(
        "INSERT INTO jwt_blacklist (token_id, user_id, expires_at, reason)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (token_id) DO NOTHING"
    )
    .bind(jti_uuid)
    .bind(user_uuid)
    .bind(exp_time)
    .bind(reason)
    .execute(db)
    .await?;

    Ok(())
}

/// Refresh access token using a valid refresh token.
///
/// `presented_device_id` MUST come from an independent source (the request's
/// `X-Device-ID` header), NOT from the refresh token itself — comparing the
/// token's device_id against itself is a no-op (F-011).
pub async fn refresh_access_token(
    db: &PgPool,
    refresh_token: &str,
    presented_device_id: &str,
) -> Result<TokenPair, StatusCode> {
    // Verify refresh token signature
    let claims = verify_token_unchecked(refresh_token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Only genuine refresh tokens may be exchanged here (F-014).
    if claims.token_type != TokenType::Refresh {
        tracing::warn!("Non-refresh token presented at refresh endpoint for user {}", claims.sub);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Check if refresh token is blacklisted. Fail CLOSED: a DB error must NOT
    // be treated as "not blacklisted" (F-009).
    match is_token_blacklisted(db, &claims.jti).await {
        Ok(true) => return Err(StatusCode::UNAUTHORIZED),
        Ok(false) => {}
        Err(e) => {
            tracing::error!("Blacklist check failed during refresh, denying: {}", e);
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    // Verify device binding against the independently-presented device id (F-011).
    if claims.device_id != presented_device_id {
        tracing::warn!("Token refresh attempted from different device: token bound to {}, presented {}",
            claims.device_id, presented_device_id);
        return Err(StatusCode::FORBIDDEN);
    }

    // Blacklist the old refresh token (one-time use)
    let _ = blacklist_token(
        db,
        &claims.jti,
        &claims.sub,
        claims.exp,
        Some("Token refresh".to_string()),
    ).await;

    // Issue new token pair
    issue_token_pair(&claims.sub, presented_device_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Auth middleware that:
/// 1. Extracts and verifies JWT signature
/// 2. Checks if token is blacklisted (requires DB state)
/// 3. Validates device binding
pub async fn require_auth(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    // Extract AppState first for DB access
    let state = req.extensions()
        .get::<crate::state::AppState>()
        .cloned()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = verify_token_unchecked(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Only access tokens may authorize API calls. A refresh token presented as
    // a bearer token is rejected here so it cannot be replayed for 7 days (F-014).
    if claims.token_type != TokenType::Access {
        tracing::warn!("Non-access token presented as bearer for user {}", claims.sub);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Check blacklist. Fail CLOSED (F-009): a DB error or a missing DB must not
    // grant access — revocation that silently no-ops defeats logout/emergency revoke.
    let db = state.db.as_ref().ok_or_else(|| {
        tracing::error!("Auth blacklist check unavailable (no DB); denying request");
        StatusCode::SERVICE_UNAVAILABLE
    })?;
    match is_token_blacklisted(db, &claims.jti).await {
        Ok(true) => {
            tracing::warn!("Rejected blacklisted token for user {}", claims.sub);
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!("Blacklist check failed, denying request: {}", e);
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    // Device binding is mandatory (F-010): a request that omits X-Device-ID must
    // be rejected, not silently exempted from the check.
    let device_header = req.headers().get("X-Device-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("Missing X-Device-ID header for user {}", claims.sub);
            StatusCode::FORBIDDEN
        })?;
    if device_header != claims.device_id {
        tracing::warn!("Device mismatch for user {}: token={}, header={}",
            claims.sub, claims.device_id, device_header);
        return Err(StatusCode::FORBIDDEN);
    }

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
