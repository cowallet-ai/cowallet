use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/app-version", get(get_app_version))
}

/// Minimum + latest client build the server supports, plus store links.
///
/// The v1.0.1 MPC signing protocol changed the on-wire `MtARequest` shape with
/// NO version negotiation, so an older client can no longer complete a signature
/// against this server. The client reads `min_build` at startup and forces an
/// upgrade when its own build is lower. Build numbers are the integer after `+`
/// in pubspec `version:` (e.g. 1.0.0+16 -> 16), compared as plain integers.
#[derive(Serialize)]
struct AppVersionResponse {
    /// Clients with a build BELOW this must hard-block and upgrade.
    min_build: i64,
    /// Newest build available (for an optional "update available" nudge).
    latest_build: i64,
    ios_store_url: String,
    android_store_url: String,
}

/// Read an env var as i64, falling back to `default` when unset or unparseable.
fn env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

fn env_str(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

async fn get_app_version() -> Json<AppVersionResponse> {
    // Fail OPEN: default min_build = 0 means "block nobody". A missing or
    // fat-fingered env var must never lock every user out of their wallet —
    // the gate only engages once MIN_APP_BUILD is deliberately set.
    let min_build = env_i64("MIN_APP_BUILD", 0);
    let latest_build = env_i64("LATEST_APP_BUILD", min_build);

    Json(AppVersionResponse {
        min_build,
        latest_build,
        ios_store_url: env_str("IOS_STORE_URL", "https://apps.apple.com/app/cowallet"),
        android_store_url: env_str(
            "ANDROID_STORE_URL",
            "https://play.google.com/store/apps/details?id=com.cowallet.app",
        ),
    })
}
