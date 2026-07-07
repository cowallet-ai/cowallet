use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::middleware::auth::Claims;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_token))
        .route("/send", post(send_push))
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: msg.to_string() }))
}

#[derive(Debug, Deserialize)]
struct RegisterTokenRequest {
    token: String,
    platform: String,
    device_id: String,
}

async fn register_token(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Json(req): Json<RegisterTokenRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.require_db().map_err(|_| err(StatusCode::SERVICE_UNAVAILABLE, "database not available"))?;
    let user_id: uuid::Uuid = claims.0.sub.parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid user id in token"))?;

    if req.platform != "ios" && req.platform != "android" {
        return Err(err(StatusCode::BAD_REQUEST, "platform must be 'ios' or 'android'"));
    }

    sqlx::query(
        "INSERT INTO push_tokens (user_id, token, platform, device_id)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (token)
         DO UPDATE SET user_id = EXCLUDED.user_id, platform = EXCLUDED.platform,
                       device_id = EXCLUDED.device_id, updated_at = NOW()"
    )
    .bind(user_id)
    .bind(&req.token)
    .bind(&req.platform)
    .bind(&req.device_id)
    .execute(db)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    Ok(Json(json!({ "success": true })))
}

#[derive(Debug, Deserialize)]
struct SendPushRequest {
    title: String,
    body: String,
    data: serde_json::Value,
}

async fn send_push(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
    Json(req): Json<SendPushRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.require_db().map_err(|_| err(StatusCode::SERVICE_UNAVAILABLE, "database not available"))?;
    let caller_id: uuid::Uuid = claims.0.sub.parse()
        .map_err(|_| err(StatusCode::UNAUTHORIZED, "invalid user id in token"))?;
    // A user may only send push notifications to their own devices, so the
    // target is always the authenticated caller.
    let user_id = caller_id;

    let fcm = fcm_credentials()
        .await
        .map_err(|_| err(StatusCode::SERVICE_UNAVAILABLE, "FCM not configured"))?;

    let tokens: Vec<(String, String)> = sqlx::query_as(
        "SELECT token, device_id FROM push_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    let mut sent_count = 0usize;
    for (token, _device_id) in &tokens {
        if send_fcm_push(&state.http, &fcm, token, &req.title, &req.body, &req.data).await.is_ok() {
            sent_count += 1;
        }
    }

    Ok(Json(json!({ "success": true, "sent_count": sent_count })))
}

// ---------------------------------------------------------------------------
// FCM HTTP v1 (OAuth2 + service account). The legacy `fcm/send` + server-key
// endpoint was shut down by Google in June 2024, so all sends go through the
// v1 API: mint an OAuth2 access token from the service account, then POST to
// /v1/projects/{project_id}/messages:send.
// ---------------------------------------------------------------------------

/// Service account material needed to authenticate to FCM, loaded once from
/// the JSON key file pointed to by `FCM_SERVICE_ACCOUNT_PATH`
/// (falling back to the standard `GOOGLE_APPLICATION_CREDENTIALS`).
#[derive(Clone)]
struct FcmCredentials {
    project_id: String,
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Deserialize)]
struct ServiceAccountKey {
    project_id: String,
    client_email: String,
    private_key: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
}

fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".to_string()
}

/// Process-wide cache of the loaded service account and the current access
/// token. The token is reused until shortly before it expires.
static FCM_CREDENTIALS: LazyLock<Mutex<Option<FcmCredentials>>> =
    LazyLock::new(|| Mutex::new(None));
static FCM_TOKEN: LazyLock<Mutex<Option<(String, Instant)>>> =
    LazyLock::new(|| Mutex::new(None));

/// Load (and cache) the service account credentials from disk.
async fn fcm_credentials() -> Result<FcmCredentials, String> {
    let mut guard = FCM_CREDENTIALS.lock().await;
    if let Some(creds) = guard.as_ref() {
        return Ok(creds.clone());
    }

    let path = std::env::var("FCM_SERVICE_ACCOUNT_PATH")
        .or_else(|_| std::env::var("GOOGLE_APPLICATION_CREDENTIALS"))
        .map_err(|_| "FCM_SERVICE_ACCOUNT_PATH not set".to_string())?;

    let raw = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("failed to read service account file {path}: {e}"))?;
    let key: ServiceAccountKey = serde_json::from_str(&raw)
        .map_err(|e| format!("invalid service account json: {e}"))?;

    let creds = FcmCredentials {
        project_id: key.project_id,
        client_email: key.client_email,
        private_key: key.private_key,
        token_uri: key.token_uri,
    };
    *guard = Some(creds.clone());
    Ok(creds)
}

#[derive(Serialize)]
struct OAuthJwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: i64,
}

/// Return a valid OAuth2 access token for FCM, minting a new one via the
/// JWT-bearer grant when the cached token is missing or near expiry.
async fn fcm_access_token(
    client: &reqwest::Client,
    creds: &FcmCredentials,
) -> Result<String, String> {
    {
        let guard = FCM_TOKEN.lock().await;
        if let Some((token, expires_at)) = guard.as_ref() {
            if *expires_at > Instant::now() {
                return Ok(token.clone());
            }
        }
    }

    let now = chrono::Utc::now().timestamp();
    let claims = OAuthJwtClaims {
        iss: creds.client_email.clone(),
        scope: "https://www.googleapis.com/auth/firebase.messaging".to_string(),
        aud: creds.token_uri.clone(),
        iat: now,
        exp: now + 3600,
    };

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(creds.private_key.as_bytes())
        .map_err(|e| format!("invalid service account private key: {e}"))?;
    let assertion = jsonwebtoken::encode(&header, &claims, &encoding_key)
        .map_err(|e| format!("failed to sign oauth jwt: {e}"))?;

    let resp = client
        .post(&creds.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &assertion),
        ])
        .send()
        .await
        .map_err(|e| format!("oauth token request failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("oauth token error: {body}"));
    }

    let token: OAuthTokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("invalid oauth token response: {e}"))?;

    // Refresh a minute before the real expiry to avoid using a stale token.
    let ttl = Duration::from_secs(token.expires_in.max(60) as u64 - 60);
    let mut guard = FCM_TOKEN.lock().await;
    *guard = Some((token.access_token.clone(), Instant::now() + ttl));

    Ok(token.access_token)
}

async fn send_fcm_push(
    client: &reqwest::Client,
    creds: &FcmCredentials,
    token: &str,
    title: &str,
    body: &str,
    data: &serde_json::Value,
) -> Result<(), String> {
    let access_token = fcm_access_token(client, creds).await?;

    // FCM v1 requires string values in the `data` map, so coerce each entry.
    let data_map: serde_json::Map<String, serde_json::Value> = match data {
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(k, v)| {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                (k.clone(), serde_json::Value::String(s))
            })
            .collect(),
        _ => serde_json::Map::new(),
    };

    let payload = json!({
        "message": {
            "token": token,
            "notification": {
                "title": title,
                "body": body,
            },
            "data": data_map,
            "android": {
                "priority": "high",
            },
            "apns": {
                "payload": {
                    "aps": {
                        "sound": "default",
                        "badge": 1,
                        "content-available": 1,
                    }
                }
            },
        }
    });

    let url = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        creds.project_id
    );

    let response = client
        .post(&url)
        .bearer_auth(&access_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("FCM request failed: {e}"))?;

    if !response.status().is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!("FCM error: {error_body}"));
    }

    Ok(())
}

/// Helper to send MPC signing request push notification.
pub async fn send_mpc_signing_notification(
    db: &PgPool,
    http_client: &reqwest::Client,
    user_id: uuid::Uuid,
    session_id: &str,
    amount: &str,
    to_address: &str,
) {
    let creds = match fcm_credentials().await {
        Ok(c) => c,
        Err(_) => return,
    };

    let tokens: Result<Vec<(String,)>, _> = sqlx::query_as(
        "SELECT token FROM push_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_all(db)
    .await;

    let tokens = match tokens {
        Ok(t) => t,
        Err(_) => return,
    };

    let data = json!({
        "type": "mpc_sign_request",
        "session_id": session_id,
        "amount": amount,
        "to": to_address,
    });

    for (token,) in &tokens {
        let _ = send_fcm_push(
            http_client,
            &creds,
            token,
            "Signature Request",
            &format!("Approve transaction: {} to {}", amount, to_address),
            &data,
        )
        .await;
    }
}
