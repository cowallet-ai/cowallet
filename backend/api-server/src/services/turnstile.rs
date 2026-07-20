//! Cloudflare Turnstile verification.
//!
//! Human/bot check for abuse-prone unauthenticated endpoints (currently the
//! email OTP send). The client obtains a token from the Turnstile widget and
//! passes it to the backend, which verifies it server-side against
//! Cloudflare's siteverify API.
//!
//! Compatibility: verification is enforced when `TURNSTILE_SECRET_KEY` is
//! configured. When it is absent the check is skipped in non-production
//! environments (local dev / tests) so existing flows are not blocked. In
//! production (`APP_ENV=production`) a missing secret is a hard error — the
//! service refuses to fail open. See [`verify`].

use reqwest::Client;
use serde::Deserialize;

const SITEVERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

#[derive(Debug, Deserialize)]
struct SiteVerifyResponse {
    success: bool,
    #[serde(rename = "error-codes", default)]
    error_codes: Vec<String>,
}

/// Verify a Turnstile token.
///
/// - Returns `Ok(())` when the token is valid, OR when no secret is configured
///   (compat mode — verification disabled).
/// - Returns `Err(reason)` when a secret IS configured and the token is missing
///   or rejected by Cloudflare. Callers should map this to HTTP 403.
///
/// `remote_ip` is the caller's IP (optional; improves Cloudflare's scoring).
pub async fn verify(
    http: &Client,
    token: &str,
    remote_ip: Option<&str>,
) -> Result<(), String> {
    // Compat mode: no secret configured → skip enforcement, BUT never fail-open
    // in production. If APP_ENV=production and the secret is missing/blank, an
    // accidentally-deleted env would silently disable the human check, so we
    // reject instead. Non-production (local/dev/test/CI) still skips for
    // convenience.
    let secret = match std::env::var("TURNSTILE_SECRET_KEY") {
        Ok(s) if !s.trim().is_empty() => s,
        _ => {
            let is_production = std::env::var("APP_ENV")
                .map(|v| v.eq_ignore_ascii_case("production"))
                .unwrap_or(false);
            if is_production {
                tracing::error!(
                    "TURNSTILE_SECRET_KEY missing in production; refusing to fail open"
                );
                return Err("Turnstile misconfigured (secret missing in production)".to_string());
            }
            tracing::debug!("Turnstile disabled (TURNSTILE_SECRET_KEY not set); skipping check");
            return Ok(());
        }
    };

    if token.trim().is_empty() {
        return Err("missing Turnstile token".to_string());
    }

    let mut form = vec![
        ("secret", secret.as_str()),
        ("response", token),
    ];
    if let Some(ip) = remote_ip {
        form.push(("remoteip", ip));
    }

    let resp = http
        .post(SITEVERIFY_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Turnstile siteverify request failed: {e}"))?;

    let body: SiteVerifyResponse = resp
        .json()
        .await
        .map_err(|e| format!("Turnstile siteverify decode failed: {e}"))?;

    if body.success {
        Ok(())
    } else {
        Err(format!(
            "Turnstile verification rejected: {}",
            body.error_codes.join(",")
        ))
    }
}
