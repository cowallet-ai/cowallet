use std::sync::Arc;
use std::time::Duration;

use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::elliptic_curve::Field;
use k256::{AffinePoint, ProjectivePoint, Scalar};
use rand::rngs::OsRng;
use sqlx::PgPool;
use tokio::sync::Notify;
use uuid::Uuid;

use crate::services::crypto::{EncryptedData, EncryptionService};

/// Serialized presignature data: server's ephemeral secret k_1 and commitment R_1.
/// Layout: [k_1: 32 bytes (scalar)] [R_1: 33 bytes (compressed SEC1 point)]
const PRESIG_DATA_LEN: usize = 32 + 33;

/// Manages presignature lifecycle: generation, storage, consumption.
/// Pre-computes signing material so that online signing only needs 1 round.
#[derive(Clone)]
pub struct PresignManager {
    db: PgPool,
    encryption: EncryptionService,
    shutdown: Arc<Notify>,
}

/// Decrypted presignature data returned when reserving.
#[derive(Debug)]
pub struct PresignatureData {
    pub id: Uuid,
    /// Server's ephemeral secret scalar k_1.
    pub k: Vec<u8>,
    /// Server's commitment point R_1 = k_1 * G (compressed SEC1, 33 bytes).
    pub big_r: Vec<u8>,
}

impl PresignManager {
    pub fn new(db: PgPool, encryption: EncryptionService) -> Self {
        Self {
            db,
            encryption,
            shutdown: Arc::new(Notify::new()),
        }
    }

    /// Generate `count` presignatures for a wallet and store them encrypted in the DB.
    ///
    /// Each presignature consists of:
    /// - An ephemeral secret scalar k_1 (random, from OsRng)
    /// - A commitment R_1 = k_1 * G (the corresponding curve point)
    ///
    /// Both are stored encrypted with AES-256-GCM via the EncryptionService.
    pub async fn generate_presignatures(
        &self,
        user_id: Uuid,
        wallet_id: Uuid,
        count: u32,
    ) -> Result<u32, String> {
        let count = count.min(50); // Cap at 50 per call to avoid abuse

        let mut generated = 0u32;

        for _ in 0..count {
            // Generate ephemeral k_1
            let k = Scalar::random(&mut OsRng);
            let big_r_projective = ProjectivePoint::GENERATOR * k;
            let big_r_affine: AffinePoint = big_r_projective.into();
            let big_r_encoded = big_r_affine.to_encoded_point(true); // compressed

            // Serialize: [k_1 scalar bytes (32)] [R_1 compressed point (33)]
            let k_bytes = k.to_bytes();
            let r_bytes = big_r_encoded.as_bytes();

            let mut plaintext = Vec::with_capacity(PRESIG_DATA_LEN);
            plaintext.extend_from_slice(&k_bytes);
            plaintext.extend_from_slice(r_bytes);

            // Encrypt with AES-256-GCM, bound to the wallet via AAD so a
            // presignature row copied/swapped to another wallet will not decrypt
            // (parity with shard encryption).
            let encrypted = self
                .encryption
                .encrypt_bound(&plaintext, wallet_id.as_bytes())
                .map_err(|e| format!("encryption failed: {}", e))?;

            // Combine nonce + ciphertext for DB storage
            let mut presig_data = Vec::with_capacity(12 + encrypted.ciphertext.len());
            presig_data.extend_from_slice(&encrypted.nonce);
            presig_data.extend_from_slice(&encrypted.ciphertext);

            // Store in presignatures table
            sqlx::query(
                "INSERT INTO presignatures (wallet_id, user_id, presig_data, status, expires_at)
                 VALUES ($1, $2, $3, 'available', NOW() + INTERVAL '24 hours')",
            )
            .bind(wallet_id)
            .bind(user_id)
            .bind(&presig_data)
            .execute(&self.db)
            .await
            .map_err(|e| format!("DB insert failed: {}", e))?;

            generated += 1;
        }

        tracing::info!(
            "Generated {} presignatures for wallet {} (user {})",
            generated,
            wallet_id,
            user_id
        );

        Ok(generated)
    }

    /// Reserve one available presignature for a signing session.
    ///
    /// Uses SELECT ... FOR UPDATE SKIP LOCKED to avoid contention.
    /// Returns the decrypted presignature data (k_1 scalar + R_1 point).
    pub async fn reserve_presignature(
        &self,
        wallet_id: Uuid,
        session_id: Uuid,
    ) -> Result<Option<PresignatureData>, String> {
        // Atomic reserve: find an available presignature and mark it reserved
        let row: Option<(Uuid, Vec<u8>)> = sqlx::query_as(
            "UPDATE presignatures
             SET status = 'reserved', reserved_by = $2, reserved_at = NOW()
             WHERE id = (
                 SELECT id FROM presignatures
                 WHERE wallet_id = $1 AND status = 'available' AND expires_at > NOW()
                 ORDER BY created_at ASC
                 LIMIT 1
                 FOR UPDATE SKIP LOCKED
             )
             RETURNING id, presig_data",
        )
        .bind(wallet_id)
        .bind(session_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| format!("DB reserve failed: {}", e))?;

        let (presig_id, presig_data) = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        // Decrypt the presignature data, bound to the same wallet.
        let data = self.decrypt_presig_data(&presig_data, wallet_id)?;

        Ok(Some(PresignatureData {
            id: presig_id,
            k: data.0,
            big_r: data.1,
        }))
    }

    /// Mark a presignature as consumed after a successful signing operation.
    pub async fn consume_presignature(&self, presig_id: Uuid) -> Result<(), String> {
        sqlx::query(
            "UPDATE presignatures SET status = 'consumed', consumed_at = NOW()
             WHERE id = $1 AND status = 'reserved'",
        )
        .bind(presig_id)
        .execute(&self.db)
        .await
        .map_err(|e| format!("DB consume failed: {}", e))?;

        tracing::debug!("Consumed presignature {}", presig_id);
        Ok(())
    }

    /// Mark expired presignatures (past their expires_at) as 'expired'.
    pub async fn cleanup_expired(&self) -> Result<u64, String> {
        let result = sqlx::query(
            "UPDATE presignatures SET status = 'expired'
             WHERE status = 'available' AND expires_at <= NOW()",
        )
        .execute(&self.db)
        .await
        .map_err(|e| format!("DB cleanup failed: {}", e))?;

        let count = result.rows_affected();
        if count > 0 {
            tracing::info!("Expired {} presignatures", count);
        }
        Ok(count)
    }

    /// Expire presignatures that have been reserved too long (>10 min) without
    /// being consumed — likely from failed sessions.
    ///
    /// SECURITY: A reserved presignature's ephemeral nonce k_1 may have already
    /// been exposed to the client as R_1 = k_1*G during a partially-completed
    /// signing round. Reusing that k_1 for a second signature with a different
    /// message — combined with a repeated or attacker-controlled client nonce
    /// k_0 — reuses the aggregate nonce and leaks the private key via the
    /// classic ECDSA nonce-reuse equation. Therefore stale reservations are
    /// marked 'expired' (terminal) and NEVER returned to 'available'.
    pub async fn cleanup_stale_reservations(&self) -> Result<u64, String> {
        let result = sqlx::query(
            "UPDATE presignatures SET status = 'expired'
             WHERE status = 'reserved'
             AND reserved_at < NOW() - INTERVAL '10 minutes'
             AND consumed_at IS NULL",
        )
        .execute(&self.db)
        .await
        .map_err(|e| format!("DB stale cleanup failed: {}", e))?;

        let count = result.rows_affected();
        if count > 0 {
            tracing::info!(
                "Expired {} stale reserved presignatures (never reused)",
                count
            );
        }
        Ok(count)
    }

    /// Get the count of available presignatures for a wallet.
    pub async fn get_available_count(&self, wallet_id: Uuid) -> Result<i64, String> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM presignatures
             WHERE wallet_id = $1 AND status = 'available' AND expires_at > NOW()",
        )
        .bind(wallet_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| format!("DB count failed: {}", e))?;

        Ok(count)
    }

    /// Ensure a minimum number of presignatures are available for a wallet.
    /// If the available count is below `min_count`, generate enough to reach it.
    pub async fn ensure_minimum(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
        min_count: u32,
    ) -> Result<(), String> {
        let available = self.get_available_count(wallet_id).await?;

        if (available as u32) < min_count {
            let deficit = min_count - available as u32;
            tracing::debug!(
                "Wallet {} has {} presignatures, need {}, generating {}",
                wallet_id,
                available,
                min_count,
                deficit
            );
            self.generate_presignatures(user_id, wallet_id, deficit)
                .await?;
        }

        Ok(())
    }

    /// Spawn a background task that periodically:
    /// 1. Cleans up expired presignatures
    /// 2. Cleans up stale reservations
    /// 3. Ensures minimum presignature counts for active wallets
    pub fn spawn_background_task(self: &Arc<Self>, min_presignatures: u32) {
        let this = Arc::clone(self);
        let interval_secs = std::env::var("PRESIGN_REFRESH_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60u64);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // 1. Cleanup expired
                        if let Err(e) = this.cleanup_expired().await {
                            tracing::error!("Presign cleanup_expired failed: {}", e);
                        }

                        // 2. Cleanup stale reservations
                        if let Err(e) = this.cleanup_stale_reservations().await {
                            tracing::error!("Presign cleanup_stale failed: {}", e);
                        }

                        // 3. Top up active wallets
                        if let Err(e) = this.topup_active_wallets(min_presignatures).await {
                            tracing::error!("Presign topup failed: {}", e);
                        }
                    }
                    _ = this.shutdown.notified() => {
                        tracing::info!("PresignManager background task shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Top up presignatures for all active wallets that are below the minimum.
    async fn topup_active_wallets(&self, min_count: u32) -> Result<(), String> {
        // Query all active wallets
        let wallets: Vec<(Uuid, Uuid)> =
            sqlx::query_as("SELECT id, user_id FROM wallets WHERE status = 'active'")
                .fetch_all(&self.db)
                .await
                .map_err(|e| format!("DB fetch wallets failed: {}", e))?;

        for (wallet_id, user_id) in wallets {
            if let Err(e) = self.ensure_minimum(wallet_id, user_id, min_count).await {
                tracing::warn!(
                    "Failed to ensure minimum presignatures for wallet {}: {}",
                    wallet_id,
                    e
                );
            }
        }

        Ok(())
    }

    /// Signal the background task to stop.
    pub fn shutdown(&self) {
        self.shutdown.notify_one();
    }

    /// Decrypt stored presig_data bytes into (k_bytes, R_bytes).
    fn decrypt_presig_data(
        &self,
        stored: &[u8],
        wallet_id: Uuid,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        if stored.len() < 12 {
            return Err("presig_data too short (missing nonce)".into());
        }

        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&stored[..12]);
        let ciphertext = stored[12..].to_vec();

        let encrypted = EncryptedData { nonce, ciphertext };
        let plaintext = self
            .encryption
            .decrypt_bound(&encrypted, wallet_id.as_bytes())
            .map_err(|e| format!("presig decryption failed: {}", e))?;

        if plaintext.len() != PRESIG_DATA_LEN {
            return Err(format!(
                "presig plaintext wrong size: expected {}, got {}",
                PRESIG_DATA_LEN,
                plaintext.len()
            ));
        }

        let k_bytes = plaintext[..32].to_vec();
        let r_bytes = plaintext[32..].to_vec();

        Ok((k_bytes, r_bytes))
    }
}

/// DB-backed integration tests for the production presignature lifecycle
/// (generate → reserve → consume). Distributed presign is NOT a protocol in
/// this codebase — `PresignSession::generate_round1` is a stub; production
/// pre-computes signing material through THIS manager (ephemeral k_1 + R_1,
/// AES-256-GCM encrypted, AAD-bound to the wallet). So the "distributed presign
/// path" the co-signer actually runs is exactly what these tests cover.
///
/// Gated behind the `integration-tests` feature so `cargo test` stays green
/// without Postgres. Run with:
///   DATABASE_URL=postgres://… cargo test -p api-server --features integration-tests
#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;

    /// Seed one user + one wallet and return (user_id, wallet_id).
    async fn seed_user_wallet(pool: &PgPool) -> (Uuid, Uuid) {
        let uid = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, device_id) VALUES ($1,$2,$3)")
            .bind(uid)
            .bind(format!("presign-{uid}@example.com"))
            .bind(format!("device-{uid}"))
            .execute(pool)
            .await
            .unwrap();

        let wid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO wallets (id, user_id, public_key, eth_address) VALUES ($1,$2,$3,$4)",
        )
        .bind(wid)
        .bind(uid)
        .bind(&[1u8; 33][..])
        .bind(&[2u8; 20][..])
        .execute(pool)
        .await
        .unwrap();

        (uid, wid)
    }

    /// Full production presign lifecycle: generate N → available count == N →
    /// reserve one (decrypts to a valid k_1 scalar + R_1 point, status flips to
    /// 'reserved') → consume (status flips to 'consumed', no longer available).
    #[sqlx::test(migrations = "../migrations")]
    async fn presign_generate_reserve_consume_lifecycle(pool: PgPool) {
        let mgr = PresignManager::new(pool.clone(), EncryptionService::for_test());
        let (uid, wid) = seed_user_wallet(&pool).await;

        // 1) Generate 3 presignatures.
        let n = mgr.generate_presignatures(uid, wid, 3).await.unwrap();
        assert_eq!(n, 3, "should have generated exactly 3");
        assert_eq!(
            mgr.get_available_count(wid).await.unwrap(),
            3,
            "all 3 must be available immediately after generation"
        );

        // 2) Reserve one for a signing session.
        let session_id = Uuid::new_v4();
        let reserved = mgr
            .reserve_presignature(wid, session_id)
            .await
            .unwrap()
            .expect("a presignature must be available to reserve");

        // The decrypted material must be a well-formed k_1 scalar (32 bytes,
        // non-zero, a valid secp256k1 scalar) and R_1 (33-byte compressed SEC1
        // point) with R_1 == k_1 * G — exactly what the signing Round 1 consumes
        // via generate_round1_with_presign.
        assert_eq!(reserved.k.len(), 32, "k_1 must be 32 bytes");
        assert_eq!(reserved.big_r.len(), 33, "R_1 must be 33 compressed bytes");
        assert_ne!(reserved.k, vec![0u8; 32], "k_1 must be non-zero");

        let k_arr: [u8; 32] = reserved.k.as_slice().try_into().unwrap();
        let k_scalar = Option::<Scalar>::from(
            <Scalar as k256::elliptic_curve::PrimeField>::from_repr(k_arr.into()),
        )
        .expect("k_1 must be a valid secp256k1 scalar");
        let expected_r = ProjectivePoint::GENERATOR * k_scalar;
        let expected_r_affine: AffinePoint = expected_r.into();
        let expected_r_bytes = expected_r_affine.to_encoded_point(true);
        assert_eq!(
            reserved.big_r.as_slice(),
            expected_r_bytes.as_bytes(),
            "R_1 must equal k_1 * G (commitment consistency)"
        );

        // Reserving decremented the available pool.
        assert_eq!(
            mgr.get_available_count(wid).await.unwrap(),
            2,
            "reserving one must drop available from 3 to 2"
        );
        let status: String = sqlx::query_scalar("SELECT status FROM presignatures WHERE id = $1")
            .bind(reserved.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "reserved");

        // 3) Consume it after a "successful" sign.
        mgr.consume_presignature(reserved.id).await.unwrap();
        let status: String = sqlx::query_scalar("SELECT status FROM presignatures WHERE id = $1")
            .bind(reserved.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "consumed");
        assert_eq!(
            mgr.get_available_count(wid).await.unwrap(),
            2,
            "consumed presignature must not return to the available pool"
        );
    }

    /// A presignature is AES-GCM encrypted with the wallet id as AAD. Reserving
    /// under a DIFFERENT wallet id must fail to decrypt — a row copied/swapped
    /// across wallets is unusable (parity with shard encryption). Here we assert
    /// the binding by reserving with the wrong wallet and expecting an error.
    #[sqlx::test(migrations = "../migrations")]
    async fn reserved_presignature_is_wallet_bound(pool: PgPool) {
        let mgr = PresignManager::new(pool.clone(), EncryptionService::for_test());
        let (uid, wid) = seed_user_wallet(&pool).await;
        let (_uid2, wid2) = seed_user_wallet(&pool).await;

        mgr.generate_presignatures(uid, wid, 1).await.unwrap();

        // Move the row to wid2 directly in the DB (simulating a swap), then try
        // to reserve under wid2. The AAD (wid) no longer matches → decrypt fails.
        sqlx::query("UPDATE presignatures SET wallet_id = $1 WHERE wallet_id = $2")
            .bind(wid2)
            .bind(wid)
            .execute(&pool)
            .await
            .unwrap();

        let result = mgr.reserve_presignature(wid2, Uuid::new_v4()).await;
        assert!(
            result.is_err(),
            "decrypting a presignature under a mismatched wallet AAD must fail"
        );
    }
}

#[cfg(test)]
mod nonce_safety_tests {
    /// Guards against regressing to releasing stale reservations back to
    /// 'available'. If someone reintroduces "SET status = 'available'" in the
    /// stale-cleanup SQL, this test's source-level check fails.
    #[test]
    fn stale_cleanup_never_releases_to_available() {
        let src = include_str!("presign_manager.rs");
        // Find the cleanup_stale_reservations function body.
        let start = src
            .find("pub async fn cleanup_stale_reservations")
            .expect("function must exist");
        let body = &src[start..start + 600.min(src.len() - start)];
        assert!(
            !body.contains("'available'"),
            "cleanup_stale_reservations must NOT release nonces back to 'available' (ECDSA nonce-reuse risk)"
        );
        assert!(
            body.contains("'expired'"),
            "stale reservations must be marked 'expired'"
        );
    }
}
