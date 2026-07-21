use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Header the client stamps with its integer build number (pubspec `version:`
/// value after `+`, e.g. 1.0.0+17 -> "17").
const APP_VERSION_HEADER: &str = "x-app-version";

/// Hard server-side upgrade gate for protected routes.
///
/// The v1.0.1 MPC signing protocol changed the on-wire `MtARequest` shape with
/// NO version negotiation, so a pre-v1.0.1 client cannot complete a signature
/// and must not be allowed to start one. This is the server half of a two-layer
/// check (the client also self-gates at startup): it stops bypassed, cached, or
/// downgraded old clients that never ran the startup check.
///
/// Fail OPEN: when `MIN_APP_BUILD` is unset/0, every build passes — a
/// misconfigured env must never lock users out of their own wallets. The gate
/// engages only once `MIN_APP_BUILD` is deliberately set.
///
/// A missing/garbage `X-App-Version` is treated as build 0, so old clients that
/// predate the header are blocked exactly like any other stale build.
pub async fn version_gate(req: Request<Body>, next: axum::middleware::Next) -> Response {
    let min_build: i64 = std::env::var("MIN_APP_BUILD")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);

    if min_build <= 0 {
        return next.run(req).await;
    }

    let client_build: i64 = req
        .headers()
        .get(APP_VERSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);

    if client_build < min_build {
        tracing::warn!(
            "Rejected outdated client build {} (min {})",
            client_build,
            min_build
        );
        let ios = std::env::var("IOS_STORE_URL")
            .unwrap_or_else(|_| "https://apps.apple.com/app/cowallet".to_string());
        let android = std::env::var("ANDROID_STORE_URL").unwrap_or_else(|_| {
            "https://play.google.com/store/apps/details?id=com.cowallet.app".to_string()
        });
        // 426 Upgrade Required. Body mirrors the /config/app-version shape so a
        // client that hits this gate before its startup check can still render
        // the force-upgrade screen from the response alone.
        return (
            StatusCode::UPGRADE_REQUIRED,
            Json(json!({
                "error": "upgrade_required",
                "message": "This app version is no longer supported. Please update to continue.",
                "min_build": min_build,
                "ios_store_url": ios,
                "android_store_url": android,
            })),
        )
            .into_response();
    }

    next.run(req).await
}
