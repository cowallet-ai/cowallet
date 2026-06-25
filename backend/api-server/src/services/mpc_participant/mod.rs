pub mod shard_store;
pub mod types;

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use mpc_core::dkls23::dkg::DkgSession;
use mpc_core::dkls23::reshare::ReshareSession;
use mpc_core::dkls23::sign::SignSession;
use mpc_core::dkls23::{KeyShare, ProtocolMessage, SessionConfig};
use sqlx::PgPool;
use tokio::sync::Notify;
use uuid::Uuid;

use crate::services::crypto::EncryptionService;
use crate::services::presign_manager::{PresignManager, PresignatureData};

use self::shard_store::ShardStore;
use self::types::*;

const SESSION_TIMEOUT: Duration = Duration::from_secs(300);

/// A structured signing request extracted from the client's Round 1 payload.
/// `fields` are the authoritative EIP-1559 transaction fields the server uses
/// to recompute the signing hash and enforce policy; `claimed_hash` is the
/// client's self-reported digest, used only for a cross-check.
struct SigningRequest {
    /// Serialized client SignRound1Message (the MPC protocol R_0 payload).
    r0: Vec<u8>,
    fields: chain_evm::transaction::Eip1559Fields,
    claimed_hash: Option<[u8; 32]>,
}

/// Server-side MPC participant that automatically processes protocol rounds as Party 1.
///
/// Lifecycle:
/// 1. When a new MPC session is created, `on_session_created` initializes the server's
///    protocol state and generates its Round 1 message.
/// 2. When client (Party 0) sends a message to Party 1, `on_message_received` advances
///    the state machine and generates response messages.
/// 3. On DKG completion, the server's KeyShare is encrypted and stored.
/// 4. On Sign, the stored KeyShare is loaded and used for the signing protocol.
pub struct MpcParticipant {
    shard_store: Arc<ShardStore>,
    db: PgPool,
    dkg_sessions: Arc<DashMap<Uuid, DkgSession>>,
    sign_sessions: Arc<DashMap<Uuid, SignSession>>,
    reshare_sessions: Arc<DashMap<Uuid, ReshareSession>>,
    session_meta: Arc<DashMap<Uuid, ActiveSession>>,
    /// Cached presignature data per session (reserved during init_sign_session).
    reserved_presignatures: Arc<DashMap<Uuid, PresignatureData>>,
    /// Server's backup shard contributions (f_server(3) for each session).
    /// Ephemeral storage: client fetches within seconds of DKG completion.
    backup_contributions: Arc<DashMap<Uuid, Vec<u8>>>,
    /// Optional presign manager for pre-computing signing material.
    presign_manager: Option<Arc<PresignManager>>,
    shutdown: Arc<Notify>,
}

impl MpcParticipant {
    pub fn new(db: PgPool, encryption: EncryptionService) -> Self {
        let shard_store = Arc::new(ShardStore::new(db.clone(), encryption));
        Self {
            shard_store,
            db,
            dkg_sessions: Arc::new(DashMap::new()),
            sign_sessions: Arc::new(DashMap::new()),
            reshare_sessions: Arc::new(DashMap::new()),
            session_meta: Arc::new(DashMap::new()),
            reserved_presignatures: Arc::new(DashMap::new()),
            backup_contributions: Arc::new(DashMap::new()),
            presign_manager: None,
            shutdown: Arc::new(Notify::new()),
        }
    }

    /// Set the presign manager for this participant.
    /// Called after both are initialized in AppState.
    pub fn set_presign_manager(&mut self, mgr: Arc<PresignManager>) {
        self.presign_manager = Some(mgr);
    }

    /// Start background cleanup task for expired sessions.
    pub fn spawn_cleanup(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        this.cleanup_expired();
                    }
                    _ = this.shutdown.notified() => break,
                }
            }
        });
    }

    /// Called when a new MPC session is created via HTTP.
    /// The server (Party 1) initializes its protocol state and generates Round 1.
    pub async fn on_session_created(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        session_type: &str,
        parties: &[i16],
        threshold: i16,
        wallet_id: Option<Uuid>,
    ) -> Result<(), String> {
        let Some(mpc_type) = MpcSessionType::from_str(session_type) else {
            tracing::debug!("Session {} has type '{}' — server participant not needed, skipping", session_id, session_type);
            return Ok(());
        };

        // Only participate if Party 1 is in the party list
        if !parties.contains(&(SERVER_PARTY_INDEX as i16)) {
            tracing::debug!("Session {} does not include server party, skipping", session_id);
            return Ok(());
        }

        let config = SessionConfig {
            session_id: session_id.to_string(),
            threshold: threshold as u16,
            total_parties: parties.len() as u16,
            party_index: SERVER_PARTY_INDEX,
        };

        match mpc_type {
            MpcSessionType::Dkg | MpcSessionType::Keygen => {
                self.init_dkg_session(session_id, user_id, config).await
            }
            MpcSessionType::Sign => {
                self.init_sign_session(session_id, user_id, config, wallet_id).await
            }
            MpcSessionType::Reshare => {
                self.init_reshare_session(session_id, user_id, config).await
            }
        }
    }

    /// Called when a message addressed to Party 1 is stored.
    /// Processes the message and generates a response.
    /// Returns a list of (from_party, to_party, round, payload) tuples.
    pub async fn on_message_received(
        &self,
        session_id: Uuid,
        from_party: i16,
        round: i16,
        payload: &[u8],
    ) -> Result<Vec<(i16, i16, i16, Vec<u8>)>, String> {
        // Try to get session from memory; if missing, attempt recovery from DB
        let (session_type, user_id) = match self.session_meta.get(&session_id) {
            Some(meta) => (meta.session_type, meta.user_id),
            None => {
                // Session not in memory — possibly lost due to server restart.
                // Try to recover from DB.
                self.try_recover_session(session_id).await?
            }
        };

        match session_type {
            MpcSessionType::Dkg | MpcSessionType::Keygen => {
                self.process_dkg_message(session_id, user_id, from_party, round, payload).await
            }
            MpcSessionType::Sign => {
                self.process_sign_message(session_id, user_id, from_party, round, payload).await
            }
            MpcSessionType::Reshare => {
                self.process_reshare_message(session_id, user_id, from_party, round, payload).await
            }
        }
    }

    /// Attempt to recover a session from DB when in-memory state is lost (e.g. after restart).
    /// Only sign sessions on round 1 can be recovered (server re-initializes from stored shard).
    /// DKG/reshare sessions cannot be recovered because they require ephemeral crypto state.
    async fn try_recover_session(&self, session_id: Uuid) -> Result<(MpcSessionType, Uuid), String> {
        let row: Option<(String, Uuid, Vec<i16>, i16, Option<Uuid>)> = sqlx::query_as(
            "SELECT session_type, user_id, parties, threshold, wallet_id
             FROM mpc_sessions WHERE id = $1 AND status = 'active'"
        )
        .bind(session_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| format!("DB error recovering session {}: {}", session_id, e))?;

        let (session_type_str, user_id, parties, threshold, wallet_id) =
            row.ok_or_else(|| format!("no active session {} (not in memory or DB)", session_id))?;

        let Some(mpc_type) = MpcSessionType::from_str(&session_type_str) else {
            return Err(format!("session {} has unrecoverable type '{}'", session_id, session_type_str));
        };

        // Only sign sessions can be recovered (they re-init from stored shard)
        if mpc_type != MpcSessionType::Sign {
            return Err(format!(
                "session {} type '{}' cannot be recovered after restart (requires ephemeral crypto state)",
                session_id, session_type_str
            ));
        }

        tracing::info!(
            "Recovering sign session {} from DB (in-memory state was lost)",
            session_id
        );

        let config = SessionConfig {
            session_id: session_id.to_string(),
            threshold: threshold as u16,
            total_parties: parties.len() as u16,
            party_index: SERVER_PARTY_INDEX,
        };

        self.init_sign_session(session_id, user_id, config, wallet_id).await?;

        Ok((mpc_type, user_id))
    }

    /// Initialize a DKG session: create the session and generate server's Round 1.
    async fn init_dkg_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        config: SessionConfig,
    ) -> Result<(), String> {
        let mut dkg = DkgSession::new(config.clone());

        // Generate server's Round 1 message immediately
        let round1_msg = dkg.generate_round1()
            .map_err(|e| format!("DKG round 1 generation failed: {}", e))?;

        // Store the server's Round 1 message in DB so client can poll it
        self.store_outbound_message(
            session_id,
            SERVER_PARTY_INDEX as i16,
            0, // to client (Party 0) — broadcast actually
            1,
            &round1_msg.payload,
        ).await?;

        self.dkg_sessions.insert(session_id, dkg);
        self.session_meta.insert(session_id, ActiveSession {
            session_id,
            user_id,
            session_type: MpcSessionType::Dkg,
            phase: SessionPhase::AwaitingClientRound1,
            config,
            created_at: Instant::now(),
            wallet_id: None,
        });

        tracing::info!("Server DKG session {} initialized, Round 1 sent", session_id);
        Ok(())
    }

    /// Initialize a Sign session: store metadata and wait for client's Round 1.
    /// The message hash arrives with the client's first message payload.
    async fn init_sign_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        config: SessionConfig,
        wallet_id: Option<Uuid>,
    ) -> Result<(), String> {
        // Verify server shard exists before accepting the session
        // Use wallet-specific shard if wallet_id is provided
        if let Some(wid) = wallet_id {
            let _key_share = self.shard_store.load_key_share_for_wallet(user_id, wid).await?
                .ok_or_else(|| format!("no server shard for user {} wallet {}, DKG must complete first", user_id, wid))?;
        } else {
            let _key_share = self.shard_store.load_key_share(user_id).await?
                .ok_or_else(|| format!("no server shard for user {}, DKG must complete first", user_id))?;
        }

        // Try to reserve a presignature for this signing session.
        // If available, the pre-computed k_1 can be used instead of generating fresh
        // randomness during Round 1, reducing online signing latency.
        if let (Some(wid), Some(presign_mgr)) = (wallet_id, &self.presign_manager) {
            match presign_mgr.reserve_presignature(wid, session_id).await {
                Ok(Some(presig_data)) => {
                    tracing::info!(
                        "Reserved presignature {} for sign session {} (wallet {})",
                        presig_data.id, session_id, wid
                    );
                    self.reserved_presignatures.insert(session_id, presig_data);
                }
                Ok(None) => {
                    tracing::debug!(
                        "No presignature available for wallet {}, will generate fresh k during Round 1",
                        wid
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to reserve presignature for session {}: {} — proceeding without",
                        session_id, e
                    );
                }
            }
        }

        self.session_meta.insert(session_id, ActiveSession {
            session_id,
            user_id,
            session_type: MpcSessionType::Sign,
            phase: SessionPhase::SignAwaitingRound1,
            config,
            created_at: Instant::now(),
            wallet_id,
        });

        tracing::info!("Server Sign session {} initialized (wallet: {:?}), awaiting client Round 1", session_id, wallet_id);
        Ok(())
    }

    /// Process an inbound DKG message from the client.
    async fn process_dkg_message(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        _from_party: i16,
        round: i16,
        payload: &[u8],
    ) -> Result<Vec<(i16, i16, i16, Vec<u8>)>, String> {
        let mut dkg = self.dkg_sessions.get_mut(&session_id)
            .ok_or("DKG session not found")?;

        let incoming = ProtocolMessage {
            session_id: session_id.to_string(),
            from: 0, // client is party 0
            to: SERVER_PARTY_INDEX,
            round: round as u16,
            payload: payload.to_vec(),
        };

        let mut outbound = Vec::new();

        match round {
            1 => {
                // Client sent their Round 1 (commitments)
                dkg.process_round1(vec![incoming])
                    .map_err(|e| format!("process_round1 failed: {}", e))?;

                // Generate server's Round 2 messages
                let round2_msgs = dkg.generate_round2()
                    .map_err(|e| format!("generate_round2 failed: {}", e))?;

                let next_round = round + 1;
                for msg in round2_msgs {
                    // Only send messages addressed to Party 0 (client)
                    if msg.to == 0 || msg.to == BROADCAST_PARTY {
                        outbound.push((SERVER_PARTY_INDEX as i16, msg.to as i16, next_round, msg.payload));
                    }
                }

                if let Some(mut meta) = self.session_meta.get_mut(&session_id) {
                    meta.phase = SessionPhase::AwaitingClientRound2;
                }

                tracing::info!("DKG session {}: processed client R1, sent server R2", session_id);
            }
            2 => {
                // Client sent their Round 2 (share evaluations)
                let key_share = dkg.process_round2(vec![incoming])
                    .map_err(|e| format!("process_round2 failed: {}", e))?;

                // Compute server's backup contribution: f_server(3) for backup party index 2
                // CRITICAL: Compute BEFORE removing the DKG session
                let backup_contribution = dkg.derive_backup_share(2) // party index 2
                    .map(|share| share.secret_share.as_bytes().to_vec())
                    .unwrap_or_default();

                // Store backup contribution in ephemeral in-memory cache for client to fetch
                if !backup_contribution.is_empty() {
                    self.backup_contributions.insert(session_id, backup_contribution);
                    tracing::debug!(
                        "Stored backup contribution for session {} (32 bytes)",
                        session_id
                    );
                }

                // Compute eth_address from the KeyShare's public key
                let eth_addr = key_share.eth_address();

                // Count existing wallets for naming
                let wallet_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM wallets WHERE user_id = $1"
                )
                .bind(user_id)
                .fetch_one(&self.db)
                .await
                .unwrap_or(0);

                let wallet_name = format!("Wallet {}", wallet_count + 1);
                let default_chain_ids: Vec<i64> = vec![1, 8453, 42161, 10, 56, 137];

                // Atomically create wallet entry and store server shard.
                // If either step fails, both are rolled back — preventing a wallet without a shard.
                let mut db_tx = self.db.begin().await
                    .map_err(|e| format!("failed to begin transaction: {}", e))?;

                let wallet_id: Uuid = sqlx::query_scalar(
                    "INSERT INTO wallets (user_id, name, public_key, eth_address, chain_ids, status)
                     VALUES ($1, $2, $3, $4, $5, 'active')
                     RETURNING id"
                )
                .bind(user_id)
                .bind(&wallet_name)
                .bind(&key_share.public_key)
                .bind(&eth_addr.as_slice())
                .bind(&default_chain_ids)
                .fetch_one(&mut *db_tx)
                .await
                .map_err(|e| format!("failed to create wallet entry: {}", e))?;

                // Store the server's encrypted KeyShare within the same transaction
                self.shard_store.store_key_share_for_wallet_tx(&mut db_tx, user_id, wallet_id, &key_share).await?;

                db_tx.commit().await
                    .map_err(|e| format!("failed to commit wallet+shard transaction: {}", e))?;

                if let Some(mut meta) = self.session_meta.get_mut(&session_id) {
                    meta.phase = SessionPhase::DkgComplete;
                }

                // Update session status and wallet_id in DB
                let _ = sqlx::query(
                    "UPDATE mpc_sessions SET status = 'completed', completed_at = NOW(), wallet_id = $2 WHERE id = $1"
                )
                .bind(session_id)
                .bind(wallet_id)
                .execute(&self.db)
                .await;

                // Clean up in-memory state
                drop(dkg);
                self.dkg_sessions.remove(&session_id);

                tracing::info!(
                    "DKG session {} COMPLETE. Wallet {} created, server shard stored for user {}",
                    session_id, wallet_id, user_id
                );
            }
            _ => {
                return Err(format!("unexpected DKG round {}", round));
            }
        }

        // Store outbound messages in DB using the round embedded in each tuple
        for (from, to, msg_round, ref payload) in &outbound {
            self.store_outbound_message(session_id, *from, *to, *msg_round, payload).await?;
        }

        Ok(outbound)
    }

    /// Process an inbound Sign message from the client.
    ///
    /// Protocol flow (server is higher-indexed Party 1):
    /// Round 1: Receive client R_0 + msg_hash → create SignSession → process R_0 → generate R_1
    /// Round 2: Receive MtARequest → process_round2 (homomorphic Enc(s)) → send ServerSignature(Enc(s))
    async fn process_sign_message(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        _from_party: i16,
        round: i16,
        payload: &[u8],
    ) -> Result<Vec<(i16, i16, i16, Vec<u8>)>, String> {
        let mut outbound = Vec::new();

        match round {
            1 => {
                // SECURITY GATE (P0): the client's Round 1 payload carries the
                // structured EIP-1559 transaction. The server recomputes the
                // signing hash itself, rejects any client/server mismatch, and
                // evaluates policy BEFORE contributing a signature share.
                let req = Self::extract_signing_request(payload)?;

                // 1) Recompute the signing hash from the raw transaction fields.
                let recomputed = chain_evm::transaction::eip1559_signing_hash(&req.fields);
                let msg_hash: [u8; 32] = recomputed.0;

                // 2) Cross-check the client's claimed hash, if provided.
                if let Some(claimed) = req.claimed_hash {
                    if claimed != msg_hash {
                        tracing::warn!(
                            "Sign hash mismatch session={} (client claim != server recompute)",
                            session_id
                        );
                        return Err("msg_hash does not match raw_tx".into());
                    }
                }

                // Load server's key share and create the actual SignSession now
                let meta = self.session_meta.get(&session_id)
                    .ok_or("session meta not found")?;
                let config = meta.config.clone();
                let wallet_id = meta.wallet_id;
                drop(meta);

                // 3) Evaluate policy (limits, whitelist, chain, time, etc.).
                //    A denial aborts the signing protocol before any share leaks.
                self.enforce_signing_policy(user_id, &req.fields).await?;

                // Use wallet-specific shard if wallet_id is available
                let key_share = if let Some(wid) = wallet_id {
                    self.shard_store.load_key_share_for_wallet(user_id, wid).await?
                        .ok_or_else(|| format!("no server shard for user {} wallet {}", user_id, wid))?
                } else {
                    self.shard_store.load_key_share(user_id).await?
                        .ok_or_else(|| format!("no server shard for user {}", user_id))?
                };

                tracing::debug!(
                    "[MPC Sign] session={} user={} wallet={:?}",
                    session_id, user_id, wallet_id,
                );

                let mut sign = SignSession::new_distributed(config, key_share, msg_hash);

                // Use reserved presignature if available, otherwise generate fresh
                let server_r1 = if let Some(presig) = self.reserved_presignatures.get(&session_id) {
                    let k_bytes: [u8; 32] = presig.k.as_slice().try_into()
                        .map_err(|_| "presign k wrong length".to_string())?;
                    sign.generate_round1_with_presign(&k_bytes, &presig.big_r)
                        .map_err(|e| format!("sign generate_round1_with_presign failed: {}", e))?
                } else {
                    sign.generate_round1()
                        .map_err(|e| format!("sign generate_round1 failed: {}", e))?
                };

                // Now process client's R_0 (the serialized SignRound1Message).
                let incoming = ProtocolMessage {
                    session_id: session_id.to_string(),
                    from: 0,
                    to: SERVER_PARTY_INDEX,
                    round: 1,
                    payload: req.r0.clone(),
                };
                sign.process_round1(vec![incoming])
                    .map_err(|e| format!("sign process_round1 failed: {}", e))?;

                // Send R_1 back to client (response round equals the incoming round)
                let next_round = round;
                outbound.push((SERVER_PARTY_INDEX as i16, 0i16, next_round, server_r1.payload));

                // Store the session for round 2
                self.sign_sessions.insert(session_id, sign);

                if let Some(mut meta) = self.session_meta.get_mut(&session_id) {
                    meta.phase = SessionPhase::SignAwaitingRound2;
                }

                tracing::info!("Sign session {}: processed client R1, sent server R1", session_id);
            }
            2 => {
                // Client sent MtARequest (Paillier-encrypted values + range proofs)
                let mut sign = self.sign_sessions.get_mut(&session_id)
                    .ok_or("Sign session not found")?;

                let incoming = ProtocolMessage {
                    session_id: session_id.to_string(),
                    from: 0,
                    to: SERVER_PARTY_INDEX,
                    round: 2,
                    payload: payload.to_vec(),
                };

                // Server computes Enc(s) homomorphically and stores ServerSignature internally
                let _placeholder = sign.process_round2(vec![incoming])
                    .map_err(|e| format!("sign process_round2 failed: {}", e))?;

                // Extract the actual ServerSignature (contains Enc(s) ciphertext)
                let server_sig_payload = sign.get_server_response()
                    .ok_or_else(|| "server did not produce ServerSignature".to_string())?;


                outbound.push((SERVER_PARTY_INDEX as i16, 0i16, round + 1, server_sig_payload));

                if let Some(mut meta) = self.session_meta.get_mut(&session_id) {
                    meta.phase = SessionPhase::SignComplete;
                }

                let _ = sqlx::query(
                    "UPDATE mpc_sessions SET status = 'completed', completed_at = NOW() WHERE id = $1"
                )
                .bind(session_id)
                .execute(&self.db)
                .await;

                drop(sign);
                self.sign_sessions.remove(&session_id);

                // Mark the reserved presignature as consumed (if one was used)
                if let Some((_, presig_data)) = self.reserved_presignatures.remove(&session_id) {
                    if let Some(presign_mgr) = &self.presign_manager {
                        if let Err(e) = presign_mgr.consume_presignature(presig_data.id).await {
                            tracing::warn!("Failed to mark presignature {} as consumed: {}", presig_data.id, e);
                        }
                    }
                }
            }
            _ => {
                return Err(format!("unexpected sign round {}", round));
            }
        }

        // Store outbound messages using the round embedded in each tuple
        for (from, to, msg_round, ref msg_payload) in &outbound {
            self.store_outbound_message(session_id, *from, *to, *msg_round, msg_payload).await?;
        }

        Ok(outbound)
    }

    /// Initialize a Reshare session: load existing key share, create session, generate Round 1.
    async fn init_reshare_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        config: SessionConfig,
    ) -> Result<(), String> {
        // Load existing server key share — resharing requires the old share
        let key_share = self.shard_store.load_key_share(user_id).await?
            .ok_or_else(|| format!("no server shard for user {}, cannot reshare without existing share", user_id))?;

        // Determine participants from the config. If total_parties < 3, this is a
        // recovery reshare with a subset of old-share holders (e.g. [1, 2]).
        // In that case, identify which parties are participating and what the target is.
        let total = key_share.total_parties;
        let participants_count = config.total_parties as u16;

        let mut reshare = if participants_count < total {
            // Recovery mode: fewer than all parties are participating.
            // The parties field from the DB tells us which old-share holders are active.
            // We look up the session to find the actual party indices.
            let row: Option<(Vec<i16>,)> = sqlx::query_as(
                "SELECT parties FROM mpc_sessions WHERE id = $1"
            )
            .bind(session_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| format!("failed to fetch session parties: {}", e))?;

            let participant_indices: Vec<u16> = row
                .map(|(p,)| p.into_iter().map(|x| x as u16).collect())
                .unwrap_or_else(|| (0..total).collect());

            // Target party is the one NOT in the participant list (the one being recovered)
            let target_party = (0..total)
                .find(|p| !participant_indices.contains(p))
                .unwrap_or(0);

            // Adjust config to use the full total_parties (3) for polynomial evaluation
            let full_config = SessionConfig {
                session_id: config.session_id.clone(),
                threshold: config.threshold,
                total_parties: total,
                party_index: SERVER_PARTY_INDEX,
            };

            ReshareSession::new_for_recovery(full_config, key_share, participant_indices, target_party)
        } else {
            // Proactive reshare: only device (0) + server (1) participate;
            // backup (2) is offline, its new share is derived separately.
            let participants = vec![0u16, 1u16];
            let server_config = SessionConfig {
                session_id: config.session_id.clone(),
                threshold: config.threshold,
                total_parties: key_share.total_parties,
                party_index: SERVER_PARTY_INDEX,
            };
            ReshareSession::new_for_recovery(server_config, key_share, participants, SERVER_PARTY_INDEX)
        };

        // Generate server's Round 1 messages (polynomial evaluations for each party)
        let round1_msgs = reshare.generate_round1()
            .map_err(|e| format!("Reshare round 1 generation failed: {}", e))?;

        // Store server's backup contribution for the new backup shard (g_server(3))
        if let Ok(backup_contrib) = reshare.derive_backup_share() {
            if backup_contrib.len() == 32 {
                self.backup_contributions.insert(session_id, backup_contrib);
                tracing::debug!("Stored reshare backup contribution for session {}", session_id);
            }
        }

        // Store outbound messages addressed to target (Party 0 in recovery, or all in normal reshare)
        for msg in &round1_msgs {
            if msg.to == 0 || msg.to == BROADCAST_PARTY {
                self.store_outbound_message(
                    session_id,
                    SERVER_PARTY_INDEX as i16,
                    msg.to as i16,
                    1,
                    &msg.payload,
                ).await?;
            }
        }

        self.reshare_sessions.insert(session_id, reshare);
        self.session_meta.insert(session_id, ActiveSession {
            session_id,
            user_id,
            session_type: MpcSessionType::Reshare,
            phase: SessionPhase::ReshareAwaitingRound1,
            config,
            created_at: Instant::now(),
            wallet_id: None,
        });

        tracing::info!("Server Reshare session {} initialized, Round 1 sent", session_id);
        Ok(())
    }

    /// Process an inbound Reshare message from the client.
    ///
    /// Protocol flow:
    /// Round 1: Receive client's polynomial evaluations → process_round1 → finalize → store new share
    async fn process_reshare_message(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        _from_party: i16,
        round: i16,
        payload: &[u8],
    ) -> Result<Vec<(i16, i16, i16, Vec<u8>)>, String> {
        let outbound = Vec::new();

        match round {
            1 => {
                let mut reshare = self.reshare_sessions.get_mut(&session_id)
                    .ok_or("Reshare session not found")?;

                let incoming = ProtocolMessage {
                    session_id: session_id.to_string(),
                    from: 0, // client is party 0
                    to: SERVER_PARTY_INDEX,
                    round: 1,
                    payload: payload.to_vec(),
                };

                // Process client's round 1 (their polynomial evaluations for us)
                reshare.process_round1(vec![incoming])
                    .map_err(|e| format!("reshare process_round1 failed: {}", e))?;

                // Finalize to get the new key share
                let new_key_share = reshare.finalize()
                    .map_err(|e| format!("reshare finalize failed: {}", e))?;

                // Store the new key share (upsert replaces the old one)
                drop(reshare);
                self.shard_store.store_key_share(user_id, &new_key_share).await?;

                if let Some(mut meta) = self.session_meta.get_mut(&session_id) {
                    meta.phase = SessionPhase::ReshareComplete;
                }

                // Update session status in DB
                let _ = sqlx::query(
                    "UPDATE mpc_sessions SET status = 'completed', completed_at = NOW() WHERE id = $1"
                )
                .bind(session_id)
                .execute(&self.db)
                .await;

                // Clean up in-memory state
                self.reshare_sessions.remove(&session_id);

                tracing::info!("Reshare session {} COMPLETE. New server shard stored for user {}", session_id, user_id);
            }
            _ => {
                return Err(format!("unexpected reshare round {}", round));
            }
        }

        Ok(outbound)
    }

    /// Enforce the user's transaction policies before contributing a signature
    /// share. Loads enabled policies from the DB and evaluates the recomputed
    /// transaction against them. A `Deny` decision aborts signing.
    ///
    /// Note: history-dependent rules (DailyLimit, RateLimit) require Covalent
    /// aggregates not reachable from the participant; those rules are skipped
    /// (evaluate treats `history = None` as non-matching). Static rules
    /// (ExceedsAmount, WhitelistOnly, BlacklistCheck, ChainRestriction,
    /// TimeWindow, ContractInteraction) are fully enforced here.
    async fn enforce_signing_policy(
        &self,
        user_id: Uuid,
        fields: &chain_evm::transaction::Eip1559Fields,
    ) -> Result<(), String> {
        use policy_engine::{Policy, PolicyAction, Rule};

        let rows: Vec<(serde_json::Value, serde_json::Value, String, bool, i32, Uuid)> =
            sqlx::query_as(
                "SELECT rules, action, name, enabled, priority, id
                 FROM policies WHERE user_id = $1 AND enabled = true
                 ORDER BY priority DESC",
            )
            .bind(user_id)
            .fetch_all(&self.db)
            .await
            .map_err(|e| format!("failed to load policies: {}", e))?;

        // No configured policies → nothing to enforce on the signing path.
        if rows.is_empty() {
            return Ok(());
        }

        let policies: Vec<Policy> = rows
            .into_iter()
            .filter_map(|(rules_json, action_json, name, enabled, priority, id)| {
                let rules: Vec<Rule> = serde_json::from_value(rules_json).ok()?;
                let action: PolicyAction = serde_json::from_value(action_json).ok()?;
                Some(Policy {
                    id,
                    name,
                    description: String::new(),
                    rules,
                    action,
                    enabled,
                    priority: priority as u32,
                })
            })
            .collect();

        let tx_ctx = policy_engine::types::TransactionContext {
            user_id: user_id.to_string(),
            from: alloy_primitives::Address::ZERO,
            to: fields.to.unwrap_or(alloy_primitives::Address::ZERO),
            value: fields.value,
            token: None,
            chain_id: fields.chain_id,
            is_contract_interaction: !fields.data.is_empty(),
            timestamp: chrono::Utc::now(),
            history: None,
        };

        let decision = policy_engine::rules::evaluate(&tx_ctx, &policies);
        if !decision.allowed {
            tracing::warn!(
                "Policy denied signing for user={}: {}",
                user_id,
                decision.reason
            );
            return Err(format!("policy denied: {}", decision.reason));
        }
        Ok(())
    }

    /// Extract a structured signing request from the client's Round 1 payload.
    ///
    /// SECURITY: the client MUST send the full EIP-1559 transaction fields so the
    /// server can independently recompute the signing hash and enforce policy.
    /// The client-claimed `msg_hash` is accepted only for a cross-check and is
    /// never trusted as authoritative.
    ///
    /// Expected JSON shape:
    /// ```json
    /// {
    ///   "r0": "0x..",                // serialized client SignRound1Message (hex)
    ///   "msg_hash": [.. 32 bytes ..],// client-claimed digest, cross-checked only
    ///   "tx": {                      // EIP-1559 fields, authoritative
    ///     "chain_id": 1,
    ///     "nonce": 0,
    ///     "gas_limit": 21000,
    ///     "max_fee_per_gas": "0x..",
    ///     "max_priority_fee_per_gas": "0x..",
    ///     "to": "0x..",              // null for contract creation
    ///     "value": "0x..",
    ///     "data": "0x.."
    ///   }
    /// }
    /// ```
    fn extract_signing_request(payload: &[u8]) -> Result<SigningRequest, String> {
        let json: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|_| "round 1 payload must be JSON with r0 + tx fields".to_string())?;

        // r0: serialized client SignRound1Message (the MPC protocol message).
        let r0_hex = json
            .get("r0")
            .and_then(|v| v.as_str())
            .ok_or("missing r0 in signing request")?;
        let r0 = hex::decode(r0_hex.trim_start_matches("0x"))
            .map_err(|_| "r0 is not valid hex".to_string())?;

        // tx: the authoritative EIP-1559 transaction fields.
        let tx = json.get("tx").ok_or("missing tx in signing request")?;
        let fields = Self::parse_eip1559_fields(tx)?;

        // The client-claimed hash, used only for a cross-check (never trusted).
        let claimed_hash: Option<[u8; 32]> = json
            .get("msg_hash")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                let bytes: Vec<u8> = arr
                    .iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect();
                if bytes.len() == 32 {
                    let mut a = [0u8; 32];
                    a.copy_from_slice(&bytes);
                    Some(a)
                } else {
                    None
                }
            });

        Ok(SigningRequest {
            r0,
            fields,
            claimed_hash,
        })
    }

    /// Parse the EIP-1559 transaction fields object from the Round 1 JSON.
    /// Integer-valued fields accept either a JSON number or a hex/decimal string.
    fn parse_eip1559_fields(
        tx: &serde_json::Value,
    ) -> Result<chain_evm::transaction::Eip1559Fields, String> {
        use alloy_primitives::{Address, U256};

        fn parse_u64(tx: &serde_json::Value, field: &str) -> Result<u64, String> {
            match tx.get(field) {
                Some(serde_json::Value::Number(n)) => {
                    n.as_u64().ok_or_else(|| format!("{} not a u64", field))
                }
                Some(serde_json::Value::String(s)) => {
                    let s = s.trim();
                    if let Some(hex) = s.strip_prefix("0x") {
                        u64::from_str_radix(hex, 16).map_err(|_| format!("{} bad hex", field))
                    } else {
                        s.parse::<u64>().map_err(|_| format!("{} bad number", field))
                    }
                }
                _ => Err(format!("missing {}", field)),
            }
        }

        fn parse_u128(tx: &serde_json::Value, field: &str) -> Result<u128, String> {
            match tx.get(field) {
                Some(serde_json::Value::Number(n)) => {
                    n.as_u64().map(|v| v as u128).ok_or_else(|| format!("{} not a u128", field))
                }
                Some(serde_json::Value::String(s)) => {
                    let s = s.trim();
                    if let Some(hex) = s.strip_prefix("0x") {
                        u128::from_str_radix(hex, 16).map_err(|_| format!("{} bad hex", field))
                    } else {
                        s.parse::<u128>().map_err(|_| format!("{} bad number", field))
                    }
                }
                _ => Err(format!("missing {}", field)),
            }
        }

        fn parse_u256(tx: &serde_json::Value, field: &str) -> Result<U256, String> {
            match tx.get(field) {
                Some(serde_json::Value::Number(n)) => Ok(U256::from(
                    n.as_u64().ok_or_else(|| format!("{} not an integer", field))?,
                )),
                Some(serde_json::Value::String(s)) => {
                    let s = s.trim();
                    let (radix, digits) = if let Some(hex) = s.strip_prefix("0x") {
                        (16, hex)
                    } else {
                        (10, s)
                    };
                    U256::from_str_radix(digits, radix).map_err(|_| format!("{} bad U256", field))
                }
                None => Ok(U256::ZERO), // value defaults to 0
                _ => Err(format!("{} bad type", field)),
            }
        }

        let to: Option<Address> = match tx.get("to") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(s)) => {
                let bytes = hex::decode(s.trim_start_matches("0x"))
                    .map_err(|_| "to is not valid hex".to_string())?;
                if bytes.len() != 20 {
                    return Err("to must be 20 bytes".into());
                }
                Some(Address::from_slice(&bytes))
            }
            _ => return Err("to has invalid type".into()),
        };

        let data: Vec<u8> = match tx.get("data") {
            None | Some(serde_json::Value::Null) => Vec::new(),
            Some(serde_json::Value::String(s)) => hex::decode(s.trim_start_matches("0x"))
                .map_err(|_| "data is not valid hex".to_string())?,
            _ => return Err("data has invalid type".into()),
        };

        Ok(chain_evm::transaction::Eip1559Fields {
            chain_id: parse_u64(tx, "chain_id")?,
            nonce: parse_u64(tx, "nonce")?,
            gas_limit: parse_u64(tx, "gas_limit")?,
            max_fee_per_gas: parse_u128(tx, "max_fee_per_gas")?,
            max_priority_fee_per_gas: parse_u128(tx, "max_priority_fee_per_gas")?,
            to,
            value: parse_u256(tx, "value")?,
            data,
        })
    }

    /// Persist an outbound message from the server into mpc_messages so the client can poll it.
    async fn store_outbound_message(
        &self,
        session_id: Uuid,
        from_party: i16,
        to_party: i16,
        round: i16,
        payload: &[u8],
    ) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO mpc_messages (session_id, from_party, to_party, round, payload, verified)
             VALUES ($1, $2, $3, $4, $5, true)"
        )
        .bind(session_id)
        .bind(from_party)
        .bind(to_party)
        .bind(round)
        .bind(payload)
        .execute(&self.db)
        .await
        .map_err(|e| format!("failed to store outbound message: {}", e))?;

        // Update session activity
        let _ = sqlx::query(
            "UPDATE mpc_sessions SET last_activity = NOW(), current_round = GREATEST(current_round, $2)
             WHERE id = $1"
        )
        .bind(session_id)
        .bind(round as i32)
        .execute(&self.db)
        .await;

        Ok(())
    }

    /// Remove expired sessions from memory and mark as expired in DB.
    fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut expired = Vec::new();
        for entry in self.session_meta.iter() {
            if now.duration_since(entry.created_at) > SESSION_TIMEOUT {
                expired.push(*entry.key());
            }
        }

        for id in expired {
            self.session_meta.remove(&id);
            self.dkg_sessions.remove(&id);
            self.sign_sessions.remove(&id);
            self.reshare_sessions.remove(&id);
            self.reserved_presignatures.remove(&id);

            // Mark as expired in DB (async but fire-and-forget)
            let db = self.db.clone();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE mpc_sessions SET status = 'expired', completed_at = NOW()
                     WHERE id = $1 AND status NOT IN ('completed', 'failed', 'expired')"
                )
                .bind(id)
                .execute(&db)
                .await;
            });

            tracing::info!("Expired stale MPC session {} (timeout after 5 minutes)", id);
        }
    }

    /// Fetch the server's backup contribution for a completed DKG session.
    /// Returns None if not found. The contribution is removed after fetching (single-use).
    pub fn fetch_backup_contribution(&self, session_id: Uuid, requesting_user_id: Uuid) -> Option<Vec<u8>> {
        // Verify the session belongs to the requesting user
        if let Some(meta) = self.session_meta.get(&session_id) {
            if meta.user_id != requesting_user_id {
                tracing::warn!(
                    "User {} attempted to fetch backup contribution for session {} owned by user {}",
                    requesting_user_id, session_id, meta.user_id
                );
                return None;
            }
        } else {
            tracing::debug!("Session {} metadata not found (may have been cleaned up)", session_id);
            // Still try to fetch if available (session may have completed and been cleaned up)
        }

        // Remove and return the contribution (single-use fetch)
        self.backup_contributions.remove(&session_id).map(|(_, v)| v)
    }

    /// Check if the participant has an active in-memory session for the given session_id.
    /// Used by the recovery endpoint to determine if re-initialization is needed.
    pub fn has_active_session(&self, session_id: Uuid) -> bool {
        self.session_meta.contains_key(&session_id)
    }

    /// Remove all in-memory state for a session. Used during recovery to
    /// clear stale crypto state before re-initializing with fresh parameters.
    pub fn remove_session(&self, session_id: Uuid) {
        self.session_meta.remove(&session_id);
        self.dkg_sessions.remove(&session_id);
        self.sign_sessions.remove(&session_id);
        self.reshare_sessions.remove(&session_id);
        self.reserved_presignatures.remove(&session_id);
    }

    /// Compute a Feldman-style commitment for recovery verification.
    ///
    /// Returns `G * (lambda_1 * s_1)` as compressed SEC1 bytes (33 bytes).
    /// The client uses this to verify: `server_commitment + G*(lambda_2 * backup_shard) == PublicKey`.
    /// If the backup shard is wrong, the sum won't equal the public key.
    pub async fn compute_recovery_commitment(&self, user_id: Uuid) -> Result<Vec<u8>, String> {
        use k256::elliptic_curve::sec1::ToEncodedPoint;
        use k256::elliptic_curve::PrimeField;
        use k256::{AffinePoint, ProjectivePoint, Scalar};

        let key_share = self.shard_store.load_key_share(user_id).await?
            .ok_or_else(|| format!("no server shard for user {}", user_id))?;

        // Parse server's secret share scalar
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&key_share.secret_share.as_bytes()[..32]);
        let s1 = Option::<Scalar>::from(Scalar::from_repr(bytes.into()))
            .ok_or_else(|| "invalid server secret share".to_string())?;

        // Lagrange coefficient for party 1 within participants [1, 2]:
        // lambda_1 = x_2 / (x_2 - x_1) = 3 / (3 - 2) = 3
        let x1 = Scalar::from(2u64); // party_index + 1
        let x2 = Scalar::from(3u64); // party_index + 1
        let den = x2 - x1;
        let den_inv = Option::<Scalar>::from(den.invert())
            .ok_or_else(|| "Lagrange denominator zero".to_string())?;
        let lambda_1 = x2 * den_inv;

        // Compute G * (lambda_1 * s_1)
        let weighted = lambda_1 * s1;
        let point = ProjectivePoint::GENERATOR * weighted;
        let affine: AffinePoint = point.into();
        let encoded = affine.to_encoded_point(true); // compressed

        Ok(encoded.as_bytes().to_vec())
    }

    /// Graceful shutdown.
    pub fn shutdown(&self) {
        self.shutdown.notify_one();
    }
}

#[cfg(test)]
mod signing_gate_tests {
    use super::*;

    fn valid_tx_json() -> String {
        // r0 is an opaque MPC message blob; tx carries the EIP-1559 fields.
        r#"{
            "r0": "0xdeadbeef",
            "tx": {
                "chain_id": 1,
                "nonce": 0,
                "gas_limit": 21000,
                "max_fee_per_gas": "0x3b9aca00",
                "max_priority_fee_per_gas": "0x3b9aca00",
                "to": "0x1111111111111111111111111111111111111111",
                "value": "0xde0b6b3a7640000",
                "data": "0x"
            }
        }"#
        .to_string()
    }

    #[test]
    fn signing_request_rejects_non_json() {
        assert!(MpcParticipant::extract_signing_request(b"not json").is_err());
    }

    #[test]
    fn signing_request_requires_r0() {
        let payload = br#"{"tx":{"chain_id":1,"nonce":0,"gas_limit":21000,"max_fee_per_gas":1,"max_priority_fee_per_gas":1,"value":"0x0"}}"#;
        assert!(MpcParticipant::extract_signing_request(payload).is_err());
    }

    #[test]
    fn signing_request_requires_tx() {
        let payload = br#"{"r0":"0xabcd"}"#;
        assert!(MpcParticipant::extract_signing_request(payload).is_err());
    }

    #[test]
    fn signing_request_parses_full_tx() {
        let payload = valid_tx_json();
        let req = MpcParticipant::extract_signing_request(payload.as_bytes())
            .expect("should parse");
        assert_eq!(req.r0, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(req.fields.chain_id, 1);
        assert_eq!(req.fields.gas_limit, 21000);
        assert!(req.fields.to.is_some());
        // 1 ETH = 10^18 wei
        assert_eq!(
            req.fields.value,
            alloy_primitives::U256::from(1_000_000_000_000_000_000u128)
        );
    }

    #[test]
    fn recompute_hash_is_deterministic_and_field_sensitive() {
        let req = MpcParticipant::extract_signing_request(valid_tx_json().as_bytes()).unwrap();
        let h1 = chain_evm::transaction::eip1559_signing_hash(&req.fields);
        let h2 = chain_evm::transaction::eip1559_signing_hash(&req.fields);
        assert_eq!(h1, h2, "hash must be deterministic");

        // Tampering with the value changes the signing hash — the basis for
        // rejecting a client whose claimed digest doesn't match its tx.
        let mut tampered = req.fields.clone();
        tampered.value = alloy_primitives::U256::from(2_000_000_000_000_000_000u128);
        let h3 = chain_evm::transaction::eip1559_signing_hash(&tampered);
        assert_ne!(h1, h3, "different value must yield a different hash");
    }
}
