use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::delete,
};

use crate::middleware::audit::AuditResult;
use crate::middleware::auth::Claims;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", delete(delete_account))
}

/// DELETE /api/v1/account
///
/// Permanently deletes the authenticated user's account and every row that
/// references it. Satisfies App Store Guideline 5.1.1(v): an app that supports
/// account creation must offer full account deletion (not just deactivation).
///
/// Most `REFERENCES users(id)` foreign keys are NOT declared `ON DELETE
/// CASCADE`, so we delete child rows explicitly, in FK-safe order, inside a
/// single transaction — either the whole account is gone or nothing changes.
/// (`jwt_blacklist`, `recovery_sessions`, `push_tokens` ARE cascade, so they
/// drop automatically when `users` is removed.)
async fn delete_account(
    State(state): State<AppState>,
    claims: axum::Extension<Claims>,
) -> Result<StatusCode, StatusCode> {
    let db = state.require_db().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let user_id = uuid::Uuid::parse_str(&claims.0.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Audit BEFORE deleting audit_logs, so the record survives... except we then
    // delete it below. We instead log to tracing here and rely on the pre-delete
    // audit row being removed with the rest — the operational log is the trace.
    let _ = state
        .audit_logger
        .log_with_details(
            user_id,
            "account.delete",
            AuditResult::Success,
            None,
            None,
            None,
            None,
            Some(serde_json::json!({ "device_id": claims.0.device_id })),
        )
        .await;

    purge_user_data(db, user_id, &claims.0.device_id)
        .await
        .map_err(|e| {
            tracing::error!("account.delete: purge failed for {user_id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // NOTE: we deliberately do NOT blacklist the presenting access token here.
    // `jwt_blacklist.user_id` is FK'd to `users(id)` ON DELETE CASCADE, so any
    // blacklist row would either fail insertion (user already gone) or be
    // cascade-deleted with the user — blacklisting is structurally impossible
    // post-deletion. The token instead lapses on its own short expiry, and the
    // client wipes its stored tokens immediately after this call succeeds. All
    // user-scoped rows are already gone, so the token grants access to nothing.

    tracing::info!("Account deleted for user {}", user_id);
    Ok(StatusCode::NO_CONTENT)
}

/// Delete every row belonging to `user_id` (plus device/email-keyed rows) in one
/// transaction, in an order that never violates a foreign key. Either the whole
/// account is erased or nothing changes.
///
/// FK ordering notes (most `REFERENCES users(id)` FKs are NOT `ON DELETE
/// CASCADE`, so order is load-bearing):
///   - `presignatures.reserved_by → mpc_sessions(id)` (no cascade) ⇒ presignatures
///     MUST be deleted before mpc_sessions.
///   - `presignatures.wallet_id → wallets(id)` (cascade) ⇒ still delete
///     presignatures before wallets to keep the intent explicit.
///   - `mpc_messages → mpc_sessions(id)` and `chat_messages → chat_sessions(id)`
///     ARE cascade, so deleting the parent clears the children.
///   - `jwt_blacklist`, `recovery_sessions`, `push_tokens` are `ON DELETE
///     CASCADE` on users ⇒ cleared automatically by the final `DELETE FROM users`.
///
/// `email_verifications` is keyed by email (not user_id); `login_challenges` is
/// keyed by device_id. Both are handled explicitly.
pub async fn purge_user_data(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    device_id: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;

    // email_verifications is keyed by the account email, not user_id.
    let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

    // Ordered so no statement removes a row still referenced by a not-yet-deleted
    // one. presignatures precede mpc_sessions AND wallets (see fn docs).
    let user_scoped = [
        "DELETE FROM presignatures WHERE user_id = $1",
        "DELETE FROM mpc_sessions WHERE user_id = $1",
        "DELETE FROM shard_metadata WHERE user_id = $1",
        "DELETE FROM shard_metadata_archive WHERE user_id = $1",
        "DELETE FROM transactions WHERE user_id = $1",
        "DELETE FROM policies WHERE user_id = $1",
        "DELETE FROM user_policies WHERE user_id = $1",
        "DELETE FROM wallets WHERE user_id = $1",
        "DELETE FROM chat_sessions WHERE user_id = $1",
        "DELETE FROM ws_tickets WHERE user_id = $1",
        "DELETE FROM audit_logs WHERE user_id = $1",
    ];
    for stmt in user_scoped {
        sqlx::query(stmt).bind(user_id).execute(&mut *tx).await?;
    }

    // login_challenges are keyed by device_id (no user_id column).
    sqlx::query("DELETE FROM login_challenges WHERE device_id = $1")
        .bind(device_id)
        .execute(&mut *tx)
        .await?;

    if let Some(email) = &email {
        sqlx::query("DELETE FROM email_verifications WHERE email = $1")
            .bind(email)
            .execute(&mut *tx)
            .await?;
    }

    // Finally the user row; cascade FKs (jwt_blacklist / recovery_sessions /
    // push_tokens) drop with it.
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await
}

// DB-backed test. Gated behind the `integration-tests` feature (and thus the
// `sqlx` macros/migrate dev-deps) so `cargo test` stays green without Postgres.
// Run with:  DATABASE_URL=postgres://… cargo test -p api-server --features integration-tests
#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;
    use chrono::Utc;

    /// Insert one row into every table that references this user, including a
    /// *reserved* presignature (`reserved_by → mpc_sessions`) — the case that
    /// makes delete order load-bearing — and the three cascade tables.
    async fn seed_user(pool: &sqlx::PgPool, email: &str, device_id: &str) -> uuid::Uuid {
        let uid = uuid::Uuid::new_v4();
        let q = |sql: &'static str| sqlx::query(sql);
        q("INSERT INTO users (id, email, device_id) VALUES ($1,$2,$3)")
            .bind(uid).bind(email).bind(device_id).execute(pool).await.unwrap();

        let wid = uuid::Uuid::new_v4();
        q("INSERT INTO wallets (id, user_id, public_key, eth_address) VALUES ($1,$2,$3,$4)")
            .bind(wid).bind(uid).bind(&[1u8; 33][..]).bind(&[2u8; 20][..])
            .execute(pool).await.unwrap();

        let sid = uuid::Uuid::new_v4();
        q("INSERT INTO mpc_sessions (id, session_type, user_id, parties) VALUES ($1,'sign',$2,$3)")
            .bind(sid).bind(uid).bind(vec![1i16, 2i16]).execute(pool).await.unwrap();
        q("INSERT INTO mpc_messages (session_id, from_party, to_party, round, payload) VALUES ($1,1,2,1,$2)")
            .bind(sid).bind(&[9u8; 4][..]).execute(pool).await.unwrap();
        // reserved_by references the session above → presignatures MUST be
        // deletable before mpc_sessions, or this row aborts the whole purge.
        q("INSERT INTO presignatures (wallet_id, user_id, presig_data, status, reserved_by) VALUES ($1,$2,$3,'reserved',$4)")
            .bind(wid).bind(uid).bind(&[7u8; 8][..]).bind(sid).execute(pool).await.unwrap();
        seed_user_rest(pool, uid, wid, email, device_id).await;
        uid
    }

    async fn seed_user_rest(
        pool: &sqlx::PgPool,
        uid: uuid::Uuid,
        _wid: uuid::Uuid,
        email: &str,
        device_id: &str,
    ) {
        let q = |sql: &'static str| sqlx::query(sql);
        q("INSERT INTO shard_metadata (user_id, location, party_index) VALUES ($1,'server',1)")
            .bind(uid).execute(pool).await.unwrap();
        q("INSERT INTO shard_metadata_archive (original_id, user_id, location, party_index, encrypted_shard, nonce, created_at) VALUES ($1,$2,'server',1,$3,$4,NOW())")
            .bind(uuid::Uuid::new_v4()).bind(uid).bind(&[1u8; 4][..]).bind(&[2u8; 12][..])
            .execute(pool).await.unwrap();
        q("INSERT INTO transactions (user_id, chain_id, from_addr, to_addr, value) VALUES ($1,1,$2,$3,'0')")
            .bind(uid).bind(&[1u8; 20][..]).bind(&[2u8; 20][..]).execute(pool).await.unwrap();
        q("INSERT INTO policies (user_id, name, rules, action) VALUES ($1,'p','{}','{}')")
            .bind(uid).execute(pool).await.unwrap();
        q("INSERT INTO user_policies (user_id) VALUES ($1)")
            .bind(uid).execute(pool).await.unwrap();
        let cs = uuid::Uuid::new_v4();
        q("INSERT INTO chat_sessions (id, user_id) VALUES ($1,$2)")
            .bind(cs).bind(uid).execute(pool).await.unwrap();
        q("INSERT INTO chat_messages (session_id, role, content) VALUES ($1,'user','hi')")
            .bind(cs).execute(pool).await.unwrap();
        q("INSERT INTO ws_tickets (ticket, user_id, device_id, expires_at) VALUES ($1,$2,$3,$4)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(uid).bind(device_id)
            .bind(Utc::now() + chrono::Duration::minutes(1)).execute(pool).await.unwrap();
        q("INSERT INTO audit_logs (user_id, action, result) VALUES ($1,'x','success')")
            .bind(uid).execute(pool).await.unwrap();
        // Cascade-on-users tables (should vanish with the user row).
        q("INSERT INTO jwt_blacklist (token_id, user_id, expires_at) VALUES ($1,$2,$3)")
            .bind(uuid::Uuid::new_v4()).bind(uid).bind(Utc::now() + chrono::Duration::hours(1))
            .execute(pool).await.unwrap();
        q("INSERT INTO recovery_sessions (id, user_id, otp_hash, expires_at) VALUES ($1,$2,$3,$4)")
            .bind(uuid::Uuid::new_v4()).bind(uid).bind(&[3u8; 4][..])
            .bind(Utc::now() + chrono::Duration::minutes(10)).execute(pool).await.unwrap();
        q("INSERT INTO push_tokens (user_id, token, platform, device_id) VALUES ($1,$2,'ios',$3)")
            .bind(uid).bind(uuid::Uuid::new_v4().to_string()).bind(device_id)
            .execute(pool).await.unwrap();
        // email- / device-keyed (no user_id FK).
        q("INSERT INTO email_verifications (email, otp_hash, expires_at) VALUES ($1,$2,$3)")
            .bind(email).bind(&[4u8; 4][..]).bind(Utc::now() + chrono::Duration::minutes(10))
            .execute(pool).await.unwrap();
        q("INSERT INTO login_challenges (device_id, challenge) VALUES ($1,$2)")
            .bind(device_id).bind(&[5u8; 32][..]).execute(pool).await.unwrap();
    }

    /// Total rows across every table that should belong to this user.
    async fn count_user_rows(
        pool: &sqlx::PgPool,
        uid: uuid::Uuid,
        email: &str,
        device_id: &str,
    ) -> i64 {
        let by_uid = [
            "wallets", "mpc_sessions", "presignatures", "shard_metadata",
            "shard_metadata_archive", "transactions", "policies", "user_policies",
            "chat_sessions", "ws_tickets", "audit_logs", "jwt_blacklist",
            "recovery_sessions", "push_tokens",
        ];
        let mut total = 0i64;
        for t in by_uid {
            let sql = format!("SELECT COUNT(*) FROM {t} WHERE user_id = $1");
            total += sqlx::query_scalar::<_, i64>(&sql).bind(uid).fetch_one(pool).await.unwrap();
        }
        // users (by id), plus email- and device-keyed tables.
        total += sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE id = $1")
            .bind(uid).fetch_one(pool).await.unwrap();
        total += sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM email_verifications WHERE email = $1")
            .bind(email).fetch_one(pool).await.unwrap();
        total += sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM login_challenges WHERE device_id = $1")
            .bind(device_id).fetch_one(pool).await.unwrap();
        total
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn purge_removes_all_rows_for_user(pool: sqlx::PgPool) {
        let uid = seed_user(&pool, "alice@example.com", "device-alice").await;
        assert!(count_user_rows(&pool, uid, "alice@example.com", "device-alice").await >= 17,
            "seed should have populated every table");

        purge_user_data(&pool, uid, "device-alice").await.expect("purge must succeed");

        assert_eq!(
            count_user_rows(&pool, uid, "alice@example.com", "device-alice").await,
            0,
            "every user-owned row (incl. cascade + email/device-keyed) must be gone",
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn purge_leaves_other_users_untouched(pool: sqlx::PgPool) {
        let alice = seed_user(&pool, "alice@example.com", "device-alice").await;
        let bob = seed_user(&pool, "bob@example.com", "device-bob").await;

        purge_user_data(&pool, alice, "device-alice").await.expect("purge must succeed");

        assert_eq!(count_user_rows(&pool, alice, "alice@example.com", "device-alice").await, 0);
        // Bob is fully intact — no over-broad deletes.
        assert!(count_user_rows(&pool, bob, "bob@example.com", "device-bob").await >= 17,
            "unrelated user's data must survive");
    }
}
