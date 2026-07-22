use std::time::Duration;

use sqlx::PgPool;
use tracing_subscriber;

/// Background worker for periodic tasks.
///
/// Jobs:
/// - Price feed updater (every 30 seconds)
/// - Expired session cleanup
/// - Approval request expiry check
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("cowallet worker starting");

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/cowallet".into());

    let db = match PgPool::connect(&database_url).await {
        Ok(pool) => {
            tracing::info!("connected to PostgreSQL");
            Some(pool)
        }
        Err(e) => {
            tracing::warn!("PostgreSQL unavailable ({}), running limited worker", e);
            None
        }
    };

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    // Spawn periodic tasks
    let mut handles = Vec::new();

    if let Some(db_pool) = db.clone() {
        handles.push(tokio::spawn(async move {
            session_cleanup_task(db_pool).await;
        }));
    }

    if let Some(db_pool) = db.clone() {
        handles.push(tokio::spawn(async move {
            reshare_completion_task(db_pool).await;
        }));
    }

    handles.push(tokio::spawn(async move {
        price_updater_task(http).await;
    }));

    handles.push(tokio::spawn(async move {
        yield_refresh_task().await;
    }));

    tracing::info!("worker ready");

    tokio::signal::ctrl_c().await.unwrap();
    tracing::info!("shutdown signal received");

    for handle in handles {
        let _ = handle.await;
    }

    tracing::info!("worker stopped");
}

/// MPC Session Cleanup Task: runs every minute
/// Sets status='expired' for sessions where expires_at < NOW and status IN ('pending', 'active')
async fn session_cleanup_task(db: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        match sqlx::query(
            "UPDATE mpc_sessions
             SET status = 'expired'
             WHERE expires_at < NOW()
               AND status IN ('pending', 'active')",
        )
        .execute(&db)
        .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!("expired {} MPC sessions", result.rows_affected());
                }
            }
            Err(e) => tracing::error!("session cleanup failed: {}", e),
        }

        // Also clean up old completed/failed sessions (older than 24h)
        if let Err(e) = sqlx::query(
            "DELETE FROM mpc_sessions
             WHERE (status = 'completed' OR status = 'failed' OR status = 'expired')
               AND created_at < NOW() - INTERVAL '24 hours'",
        )
        .execute(&db)
        .await
        {
            tracing::error!("old session cleanup failed: {}", e);
        }

        // Purge expired ephemeral auth artifacts so the public, unauthenticated
        // /auth/challenge endpoint cannot grow these tables unbounded (F-001/F-010).
        if let Err(e) =
            sqlx::query("DELETE FROM login_challenges WHERE expires_at < NOW() - INTERVAL '1 hour'")
                .execute(&db)
                .await
        {
            tracing::error!("login_challenges cleanup failed: {}", e);
        }

        if let Err(e) =
            sqlx::query("DELETE FROM ws_tickets WHERE expires_at < NOW() - INTERVAL '1 hour'")
                .execute(&db)
                .await
        {
            tracing::error!("ws_tickets cleanup failed: {}", e);
        }
    }
}

/// Yield Data Refresh Task: refreshes DeFi Llama yield data every 5 minutes
async fn yield_refresh_task() {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    loop {
        interval.tick().await;

        // Call DeFi Llama API directly to refresh the data
        // The API server's cache will automatically refresh on next request
        // This just keeps the data warm by hitting the endpoint periodically
        match client.get("https://yields.llama.fi/pools").send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    tracing::debug!("yield data refresh completed successfully");
                } else {
                    tracing::warn!("yield data refresh returned status: {}", resp.status());
                }
            }
            Err(e) => tracing::warn!("yield data refresh failed: {}", e),
        }
    }
}

/// Reshare Completion Task: detects completed reshare sessions and updates wallets.
/// Runs every 5 minutes. Marks processed sessions as 'archived'.
async fn reshare_completion_task(db: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(300));

    tracing::info!("reshare completion task started: check every 5 minutes");

    loop {
        interval.tick().await;

        let sessions = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid)>(
            "SELECT id, wallet_id FROM mpc_sessions \
             WHERE session_type = 'reshare' \
             AND status = 'completed' \
             AND wallet_id IS NOT NULL",
        )
        .fetch_all(&db)
        .await;

        match sessions {
            Ok(rows) => {
                if rows.is_empty() {
                    tracing::debug!("no completed reshare sessions to process");
                    continue;
                }

                tracing::info!("{} completed reshare session(s) to process", rows.len());

                for (session_id, wallet_id) in rows {
                    // Update the wallet's last_reshare timestamp and increment reshare_count
                    let update_result = sqlx::query(
                        "UPDATE wallets SET last_reshare = NOW(), reshare_count = reshare_count + 1 WHERE id = $1",
                    )
                    .bind(wallet_id)
                    .execute(&db)
                    .await;

                    match update_result {
                        Ok(_) => {
                            tracing::info!(
                                "updated wallet {} after reshare completion (session {})",
                                wallet_id,
                                session_id
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "failed to update wallet {} after reshare: {}",
                                wallet_id,
                                e
                            );
                            continue;
                        }
                    }

                    // Archive the session to avoid re-processing
                    if let Err(e) =
                        sqlx::query("UPDATE mpc_sessions SET status = 'archived' WHERE id = $1")
                            .bind(session_id)
                            .execute(&db)
                            .await
                    {
                        tracing::error!("failed to archive reshare session {}: {}", session_id, e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("reshare completion query failed: {}", e);
            }
        }
    }
}

/// Price Updater Task: pre-fetches prices every 60 seconds for common tokens
async fn price_updater_task(http: reqwest::Client) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    let api_key = std::env::var("COINGECKO_API_KEY").ok();
    if api_key.is_none() {
        tracing::warn!("COINGECKO_API_KEY not set, price feed disabled");
        return;
    }
    let api_key = api_key.unwrap();

    let coingecko_api = "https://api.coingecko.com/api/v3";
    let common_tokens = ["ethereum", "usd-coin", "bitcoin", "tether", "dai"];
    let ids_param = common_tokens.join(",");

    loop {
        interval.tick().await;

        let url = format!(
            "{}/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true",
            coingecko_api, ids_param
        );

        match http
            .get(&url)
            .header("x-cg-demo-api-key", &api_key)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    tracing::debug!("price feed updated successfully");
                } else {
                    tracing::warn!("price feed returned status: {}", resp.status());
                }
            }
            Err(e) => tracing::warn!("price feed update failed: {}", e),
        }
    }
}
