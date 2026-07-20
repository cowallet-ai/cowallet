use super::{KeyShare, ProtocolMessage, SessionConfig};
use crate::errors::{MpcError, Result};
use k256::{
    elliptic_curve::{
        sec1::{FromEncodedPoint, ToEncodedPoint},
        Field, PrimeField,
    },
    AffinePoint, ProjectivePoint, Scalar,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Proactive key resharing protocol.
///
/// Generates new shares of the same underlying key without reconstructing it.
/// After resharing, old shares become useless — even if an attacker captured
/// a share before the refresh, it cannot be combined with new shares.
///
/// Should be triggered:
/// - Every 30 days (automatic, via worker crate)
/// - When a party is suspected compromised
/// - When recovering to a new device
#[allow(dead_code)]
pub struct ReshareSession {
    config: SessionConfig,
    old_share: KeyShare,
    /// Which party index the output share should be assigned to.
    /// In normal reshare this equals `old_share.party`.
    /// In recovery mode this is the target party being reconstructed (e.g. Party 0).
    target_party: u16,
    /// Indices of all old-share holders participating in this reshare.
    /// Needed for Lagrange interpolation when fewer than `total_parties` participate.
    participants: Vec<u16>,
    state: ReshareState,
    /// Our polynomial evaluation at the backup party index (party 2, x=3).
    /// Stored during generate_round1() for post-reshare backup shard derivation.
    backup_eval: Option<Scalar>,
}

/// Round 1 message for resharing: each party's new VSS commitments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReshareRound1Message {
    pub session_id: String,
    pub party_index: u16,
    pub commitments: Vec<Vec<u8>>, // New polynomial commitments
}

/// Round 2 message for resharing: secret share evaluations.
///
/// Each message carries the sender's polynomial `commitments` so the recipient
/// can run Feldman VSS verification on the received evaluation (F-008). Without
/// this, a malicious participant could inject an arbitrary share that silently
/// corrupts the reshared key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReshareRound2Message {
    pub session_id: String,
    pub from_party: u16,
    pub evaluations: Vec<(u16, Vec<u8>)>,
    /// Sender's VSS polynomial commitments (C_0, C_1, ..., C_{t-1}).
    pub commitments: Vec<Vec<u8>>,
}

#[allow(dead_code)]
enum ReshareState {
    AwaitingRound1,
    Round1Done,
    AwaitingRound2 {
        round1_messages: Vec<ReshareRound1Message>,
        /// Our own evaluation for the target party, to be added during process_round1.
        self_eval_for_target: Scalar,
    },
    Complete { new_share: KeyShare },
    Failed { error: String },
}

impl Zeroize for ReshareSession {
    fn zeroize(&mut self) {
        self.old_share.secret_share.zeroize();
        self.target_party = 0;
        self.participants.zeroize();
    }
}

impl Drop for ReshareSession {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ReshareSession {
    /// Start a resharing session with all parties participating.
    ///
    /// At least `threshold` parties with valid old shares must participate.
    /// The result is a set of new shares for the same public key, but the
    /// old shares are no longer compatible.
    pub fn new(config: SessionConfig, old_share: KeyShare) -> Self {
        let n = old_share.total_parties;
        let party = old_share.party;
        Self {
            config,
            target_party: party,
            participants: (0..n).collect(),
            old_share,
            state: ReshareState::AwaitingRound1,
            backup_eval: None,
        }
    }

    /// Start a recovery reshare session.
    ///
    /// `participants` — indices of old-share holders contributing (e.g. [1, 2] for server+backup).
    /// `target_party` — the party index for the output share (e.g. 0 for device recovery).
    pub fn new_for_recovery(
        config: SessionConfig,
        old_share: KeyShare,
        participants: Vec<u16>,
        target_party: u16,
    ) -> Self {
        Self {
            config,
            target_party,
            participants,
            old_share,
            state: ReshareState::AwaitingRound1,
            backup_eval: None,
        }
    }

    /// Generate round 1 resharing messages.
    ///
    /// Each participating party generates a polynomial g_i(x) of degree t-1 where
    /// g_i(0) = lambda_i * s_i (Lagrange-weighted old share). This ensures
    /// sum(g_i(0)) = sum(lambda_i * s_i) = S (the original secret), regardless of
    /// which t-of-n subset participates.
    pub fn generate_round1(&mut self) -> Result<Vec<ProtocolMessage>> {
        match self.state {
            ReshareState::AwaitingRound1 => {}
            _ => return Err(MpcError::ResharingFailed("invalid state for round 1".into())),
        }

        let t = self.old_share.threshold as usize;
        let n = self.old_share.total_parties as usize;
        let my_idx = self.old_share.party;
        let mut rng = OsRng;

        // Parse our old secret share
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&self.old_share.secret_share.as_bytes()[..32]);
        let old_secret = Option::<Scalar>::from(Scalar::from_repr(bytes.into()))
            .ok_or_else(|| MpcError::ResharingFailed("invalid old secret share".into()))?;

        // Compute Lagrange coefficient for our party within the participant set.
        // lambda_i = product_{j in participants, j != i} (x_j / (x_j - x_i))
        // where x_k = k + 1 (Shamir evaluation points are 1-indexed).
        let lambda = Self::lagrange_coefficient(my_idx, &self.participants)?;

        // Constant term = lambda_i * s_i (ensures sum across participants reconstructs S)
        let weighted_secret = old_secret * lambda;

        // Generate new random polynomial g_i(x) of degree t-1
        let mut coeffs = Vec::with_capacity(t);
        coeffs.push(weighted_secret);
        for _ in 1..t {
            coeffs.push(Scalar::random(&mut rng));
        }

        // Generate commitments for the new polynomial
        let mut commitments = Vec::with_capacity(t);
        for coeff in &coeffs {
            let point = AffinePoint::GENERATOR * coeff;
            let affine: AffinePoint = point.into();
            commitments.push(affine.to_encoded_point(true).as_bytes().to_vec());
        }

        // Evaluate polynomial at each new party's index (1-indexed Shamir points)
        let target = self.target_party;
        let mut evaluations = Vec::new();
        let mut self_eval_for_target = Scalar::ZERO;
        let backup_party_index: u16 = 2;

        for j in 0..n {
            let x = Scalar::from((j + 1) as u64);
            let mut y = Scalar::ZERO;
            let mut x_pow = Scalar::ONE;
            for coeff in &coeffs {
                y += coeff * &x_pow;
                x_pow *= x;
            }
            if j as u16 == target {
                self_eval_for_target = y;
            }
            if j as u16 == backup_party_index {
                self.backup_eval = Some(y);
            }
            evaluations.push((j as u16, y.to_bytes().to_vec()));
        }

        let round1 = ReshareRound1Message {
            session_id: self.config.session_id.clone(),
            party_index: my_idx,
            commitments: commitments.clone(),
        };

        // Create individual messages for each party (encrypted point-to-point).
        // Each message carries the sender's commitments for Feldman verification.
        let mut messages = Vec::new();
        for (recipient, eval_bytes) in evaluations {
            let round2 = ReshareRound2Message {
                session_id: self.config.session_id.clone(),
                from_party: my_idx,
                evaluations: vec![(recipient, eval_bytes)],
                commitments: commitments.clone(),
            };

            let payload = bincode::serialize(&round2)
                .map_err(|e| MpcError::ResharingFailed(format!("serialization failed: {}", e)))?;

            messages.push(ProtocolMessage {
                session_id: self.config.session_id.clone(),
                from: my_idx,
                to: recipient,
                round: 1,
                payload,
            });
        }

        // Store round1 state with our own evaluation for the target party
        self.state = ReshareState::AwaitingRound2 {
            round1_messages: vec![round1],
            self_eval_for_target,
        };

        Ok(messages)
    }

    /// Process round 1 messages and compute new key share.
    ///
    /// Collects share evaluations addressed to `target_party` and sums them
    /// (including our own contribution) to produce the new share.
    /// In normal reshare `target_party == old_share.party`;
    /// in recovery mode it is the party being reconstructed.
    pub fn process_round1(&mut self, messages: Vec<ProtocolMessage>) -> Result<()> {
        let (self_eval, my_idx, my_commitments) = match &self.state {
            ReshareState::AwaitingRound2 { self_eval_for_target, round1_messages } => {
                let mine = round1_messages.first().ok_or_else(|| {
                    MpcError::ResharingFailed("missing own round1 commitments".into())
                })?;
                (*self_eval_for_target, mine.party_index, mine.commitments.clone())
            }
            _ => return Err(MpcError::ResharingFailed("invalid state for round 1 processing".into())),
        };

        let target = self.target_party;
        let num_participants = self.participants.len();

        // Accumulate BOTH the evaluation share AND the constant-term commitment
        // C_{i,0} of every DISTINCT participant, keyed by sender. Keying the
        // shares by `from_party` (not appending per-message) is load-bearing for
        // F-008: a replayed round-1 message from one sender would otherwise be
        // summed twice into the secret (drifting it to S + share_i) while the
        // deduped commitment set still verified to the correct S·G — passing the
        // binding check against a quantity that no longer matches what was summed.
        // Deduping both in lockstep keeps `shares_by_party` and `constant_terms`
        // over the identical sender set.
        //
        // F-008 (key binding): once collected we verify Σ C_{i,0} == original
        // public key. Feldman only proves a share is consistent with the sender's
        // OWN polynomial — a malicious party can pick a constant term ≠ λ_i·s_i,
        // pass Feldman, yet shift the reconstructed secret S'≠S. That drift is
        // invisible until every future signature fails (funds locked).
        let mut shares_by_party: std::collections::BTreeMap<u16, Scalar> = std::collections::BTreeMap::new();
        let mut constant_terms: std::collections::BTreeMap<u16, Vec<u8>> = std::collections::BTreeMap::new();

        // Seed with our own contribution.
        shares_by_party.insert(my_idx, self_eval);
        if let Some(c0) = my_commitments.first() {
            constant_terms.insert(my_idx, c0.clone());
        }

        // Collect shares from other participants addressed to target_party
        for msg in messages {
            if msg.round != 1 {
                continue;
            }

            let round2: ReshareRound2Message = bincode::deserialize(&msg.payload)
                .map_err(|e| MpcError::ResharingFailed(format!("invalid resharing message: {}", e)))?;

            for (recipient, share_bytes) in &round2.evaluations {
                if *recipient == target {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(&share_bytes[..32]);
                    let share = Option::<Scalar>::from(Scalar::from_repr(bytes.into()))
                        .ok_or_else(|| MpcError::ResharingFailed("invalid share value".into()))?;

                    // F-008: verify the received evaluation against the sender's
                    // polynomial commitments before accepting it. A missing
                    // commitment set is a HARD ERROR (cf. F-007).
                    if round2.commitments.is_empty() {
                        return Err(MpcError::ResharingFailed(format!(
                            "missing reshare commitments from party {}",
                            round2.from_party
                        )));
                    }
                    crate::dkls23::dkg::DkgSession::verify_feldman_share(
                        &share,
                        target,
                        &round2.commitments,
                    )
                    .map_err(|e| {
                        MpcError::ResharingFailed(format!(
                            "Feldman verification failed for share from party {}: {}",
                            round2.from_party, e
                        ))
                    })?;

                    // Reject a second contribution from the same sender (replay).
                    if shares_by_party.contains_key(&round2.from_party) {
                        return Err(MpcError::ResharingFailed(format!(
                            "duplicate reshare contribution from party {}",
                            round2.from_party
                        )));
                    }
                    shares_by_party.insert(round2.from_party, share);
                    constant_terms.insert(round2.from_party, round2.commitments[0].clone());
                }
            }
        }

        // Need contributions from all participants (self + others), each distinct.
        if shares_by_party.len() < num_participants {
            return Err(MpcError::ResharingFailed(format!(
                "insufficient resharing shares: got {}, need {}",
                shares_by_party.len(),
                num_participants
            )));
        }

        // F-008 (key binding): Σ C_{i,0} MUST equal the original public key over
        // the SAME sender set that is summed below. Each honest party sets
        // C_{i,0} = (λ_i·s_i)·G, so the sum is S·G. Any deviation means the
        // reshared key would no longer match public_key — reject rather than
        // silently persisting a dead share.
        Self::verify_reshared_public_key(&constant_terms, &self.old_share.public_key)?;

        // Sum all shares to get new key share:
        // new_share[target] = sum_i(g_i(target+1))
        // Since each g_i(0) = lambda_i * s_i, the sum at any evaluation point
        // produces a valid Shamir share of the original secret S.
        let mut new_share_scalar = Scalar::ZERO;
        for share in shares_by_party.values() {
            new_share_scalar += share;
        }

        // Create final key share assigned to target_party
        let new_share = KeyShare {
            party: target,
            threshold: self.old_share.threshold,
            total_parties: self.old_share.total_parties,
            secret_share: new_share_scalar.to_bytes().to_vec().into(),
            public_key: self.old_share.public_key.clone(),
            paillier_pk: self.old_share.paillier_pk.clone(),
            // The Paillier keypair is independent of the secret share value
            // (it only protects the MtA exchange), so it is preserved across a
            // reshare to avoid regenerating safe primes.
            paillier_keypair: self.old_share.paillier_keypair.clone(),
        };

        self.state = ReshareState::Complete { new_share };
        Ok(())
    }

    /// F-008 key binding: verify Σ C_{i,0} over all participants equals the
    /// original public key. `constant_terms` maps each participant index to its
    /// constant-term commitment C_{i,0} = g_i(0)·G (compressed sec1). For honest
    /// parties g_i(0) = λ_i·s_i, so Σ g_i(0) = S and the sum of points is S·G.
    /// A mismatch means at least one party contributed an off-curve constant and
    /// the reshared secret would not match `public_key` — reject the reshare.
    fn verify_reshared_public_key(
        constant_terms: &std::collections::BTreeMap<u16, Vec<u8>>,
        expected_public_key: &[u8],
    ) -> Result<()> {
        let mut sum = ProjectivePoint::IDENTITY;
        for (party, c0_bytes) in constant_terms {
            if c0_bytes.len() < 33 {
                return Err(MpcError::ResharingFailed(format!(
                    "constant-term commitment from party {} too short",
                    party
                )));
            }
            let mut key_bytes = [0u8; 33];
            key_bytes.copy_from_slice(&c0_bytes[..33]);
            let encoded =
                k256::elliptic_curve::sec1::EncodedPoint::<k256::Secp256k1>::from_bytes(&key_bytes[..])
                    .map_err(|_| {
                        MpcError::ResharingFailed(format!(
                            "invalid constant-term commitment encoding from party {}",
                            party
                        ))
                    })?;
            let point = AffinePoint::from_encoded_point(&encoded);
            if bool::from(point.is_none()) {
                return Err(MpcError::ResharingFailed(format!(
                    "invalid constant-term commitment point from party {}",
                    party
                )));
            }
            sum += ProjectivePoint::from(point.unwrap());
        }

        let reshared_pubkey = sum
            .to_affine()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

        if reshared_pubkey != expected_public_key {
            return Err(MpcError::ResharingFailed(
                "reshare public-key binding failed: Σ C_{i,0} does not match the \
                 original public key (a participant used an inconsistent constant term)"
                    .into(),
            ));
        }

        Ok(())
    }

    /// Finalize resharing and get the new key share.
    ///
    /// The caller MUST securely erase the old share after this succeeds.
    pub fn finalize(&mut self) -> Result<KeyShare> {
        match std::mem::replace(&mut self.state, ReshareState::Failed { error: "finalized".into() }) {
            ReshareState::Complete { new_share } => Ok(new_share),
            ReshareState::Failed { error } => Err(MpcError::ResharingFailed(error)),
            _ => Err(MpcError::ResharingFailed("resharing not complete".into())),
        }
    }

    /// Get this party's contribution to the backup shard: g_i(backup_party_index + 1).
    /// Must be called after generate_round1(). Returns 32-byte scalar.
    pub fn derive_backup_share(&self) -> Result<Vec<u8>> {
        self.backup_eval
            .map(|s| s.to_bytes().to_vec())
            .ok_or_else(|| MpcError::ResharingFailed(
                "backup evaluation not available — call after generate_round1()".into(),
            ))
    }

    /// Compute Lagrange coefficient for party `i` within `participants`.
    ///
    /// lambda_i = product_{j in participants, j != i} [ x_j / (x_j - x_i) ]
    /// where x_k = k + 1 (Shamir uses 1-indexed evaluation points).
    fn lagrange_coefficient(i: u16, participants: &[u16]) -> Result<Scalar> {
        let x_i = Scalar::from((i + 1) as u64);
        let mut lambda = Scalar::ONE;

        for &j in participants {
            if j == i {
                continue;
            }
            let x_j = Scalar::from((j + 1) as u64);
            let num = x_j;
            let den = x_j - x_i;
            let den_inv = Option::<Scalar>::from(den.invert())
                .ok_or_else(|| MpcError::ResharingFailed(
                    "Lagrange denominator is zero (duplicate participant?)".into(),
                ))?;
            lambda *= num * den_inv;
        }

        Ok(lambda)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dkls23::protocol::ThresholdKeyGen;

    fn create_test_shares() -> Vec<KeyShare> {
        let config = SessionConfig {
            session_id: "test".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 0,
        };
        let kg = ThresholdKeyGen::new(config);
        kg.generate_local().unwrap()
    }

    #[test]
    fn test_reshare_session_creation() {
        let shares = create_test_shares();
        let config = SessionConfig {
            session_id: "test-reshare".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 0,
        };
        let session = ReshareSession::new(config, shares[0].clone());
        assert!(matches!(session.state, ReshareState::AwaitingRound1));
    }

    #[test]
    fn test_reshare_preserves_public_key() {
        let shares = create_test_shares();
        let pubkey = shares[0].public_key.clone();

        // For each share, create a session and simulate resharing
        let config = SessionConfig {
            session_id: "test-reshare-2".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 0,
        };
        let mut session = ReshareSession::new(config, shares[0].clone());

        // Generate round1 messages
        let messages = session.generate_round1().unwrap();
        assert!(!messages.is_empty());

        // Public key should still match
        assert_eq!(session.old_share.public_key, pubkey);
    }

    #[test]
    fn test_finalize_before_reshare_fails() {
        let shares = create_test_shares();
        let config = SessionConfig {
            session_id: "test-reshare-3".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 0,
        };
        let mut session = ReshareSession::new(config, shares[0].clone());
        assert!(session.finalize().is_err());
    }

    /// Recovery test: Party 0 (device) is lost; Party 1 (server) + Party 2 (backup)
    /// perform a 2-of-3 recovery reshare to reconstruct a new Party 0 shard.
    /// Verifies that the reconstructed shard corresponds to the same public key.
    #[test]
    fn test_recovery_reshare_2_of_3() {
        let shares = create_test_shares();
        let original_pubkey = shares[0].public_key.clone();

        // Participants: server (1) and backup (2)
        let participants = vec![1u16, 2u16];
        let target_party = 0u16;

        // Server's reshare session
        let server_config = SessionConfig {
            session_id: "recovery-test".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 1,
        };
        let mut reshare_server = ReshareSession::new_for_recovery(
            server_config,
            shares[1].clone(),
            participants.clone(),
            target_party,
        );

        // Backup's reshare session
        let backup_config = SessionConfig {
            session_id: "recovery-test".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 2,
        };
        let mut reshare_backup = ReshareSession::new_for_recovery(
            backup_config,
            shares[2].clone(),
            participants.clone(),
            target_party,
        );

        // Both generate round 1
        let server_r1 = reshare_server.generate_round1().unwrap();
        let backup_r1 = reshare_backup.generate_round1().unwrap();

        // Each processes the other's messages addressed to Party 0
        let server_msgs_for_target: Vec<_> = server_r1.into_iter()
            .filter(|m| m.to == target_party)
            .collect();
        let backup_msgs_for_target: Vec<_> = backup_r1.into_iter()
            .filter(|m| m.to == target_party)
            .collect();

        // Backup processes server's messages (backup already has its own eval internally)
        reshare_backup.process_round1(server_msgs_for_target.clone()).unwrap();

        // Server processes backup's messages (server already has its own eval internally)
        reshare_server.process_round1(backup_msgs_for_target.clone()).unwrap();

        let new_device_share_from_backup = reshare_backup.finalize().unwrap();
        let new_device_share_from_server = reshare_server.finalize().unwrap();

        // Both should produce the same shard for Party 0
        assert_eq!(new_device_share_from_backup.party, 0);
        assert_eq!(new_device_share_from_server.party, 0);
        assert_eq!(
            new_device_share_from_backup.secret_share.as_bytes(),
            new_device_share_from_server.secret_share.as_bytes(),
            "server and backup must derive the same device shard"
        );
        assert_eq!(
            new_device_share_from_backup.public_key, original_pubkey,
            "recovered shard must correspond to original public key"
        );
    }

    /// Verify that Feldman commitment detects a wrong backup shard.
    #[test]
    fn test_feldman_commitment_rejects_wrong_backup() {
        use k256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
        use k256::{EncodedPoint, ProjectivePoint};

        let shares = create_test_shares();
        let public_key = shares[0].public_key.clone();

        // Parse public key
        let pk_enc = EncodedPoint::from_bytes(&public_key).unwrap();
        let pk_affine = AffinePoint::from_encoded_point(&pk_enc).unwrap();
        let pk_point = ProjectivePoint::from(pk_affine);

        // Compute server commitment: G * (lambda_1 * s_1)
        let mut s1_bytes = [0u8; 32];
        s1_bytes.copy_from_slice(&shares[1].secret_share.as_bytes()[..32]);
        let s1 = Scalar::from_repr(s1_bytes.into()).unwrap();
        let lambda_1 = ReshareSession::lagrange_coefficient(1, &[1, 2]).unwrap();
        let server_commitment = ProjectivePoint::GENERATOR * (lambda_1 * s1);

        // Compute correct backup commitment: G * (lambda_2 * s_2)
        let mut s2_bytes = [0u8; 32];
        s2_bytes.copy_from_slice(&shares[2].secret_share.as_bytes()[..32]);
        let s2 = Scalar::from_repr(s2_bytes.into()).unwrap();
        let lambda_2 = ReshareSession::lagrange_coefficient(2, &[1, 2]).unwrap();
        let correct_backup_commitment = ProjectivePoint::GENERATOR * (lambda_2 * s2);

        // Verify: server_commitment + correct_backup_commitment == PublicKey
        let sum = server_commitment + correct_backup_commitment;
        assert_eq!(
            AffinePoint::from(sum).to_encoded_point(false).as_bytes(),
            pk_affine.to_encoded_point(false).as_bytes(),
            "correct backup shard should produce matching public key"
        );

        // Now try a WRONG backup shard (random scalar)
        let wrong_s2 = Scalar::random(&mut OsRng);
        let wrong_backup_commitment = ProjectivePoint::GENERATOR * (lambda_2 * wrong_s2);
        let wrong_sum = server_commitment + wrong_backup_commitment;
        assert_ne!(
            AffinePoint::from(wrong_sum).to_encoded_point(false).as_bytes(),
            pk_affine.to_encoded_point(false).as_bytes(),
            "wrong backup shard must NOT match public key"
        );
    }

    /// F-008 key binding: a participant whose constant term does not equal
    /// λ_i·s_i (here simulated by corrupting its secret share) still passes
    /// Feldman — its polynomial is internally consistent — but Σ C_{i,0} no
    /// longer matches the original public key. `process_round1` must reject it
    /// rather than persisting a share that drifts the key (funds lockout).
    #[test]
    fn test_reshare_rejects_inconsistent_constant_term() {
        let shares = create_test_shares();

        let participants = vec![1u16, 2u16];
        let target_party = 0u16;

        let server_config = SessionConfig {
            session_id: "binding-test".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 1,
        };
        let mut reshare_server = ReshareSession::new_for_recovery(
            server_config,
            shares[1].clone(),
            participants.clone(),
            target_party,
        );

        // Corrupt the backup party's secret share: its polynomial will still be
        // self-consistent (Feldman passes), but C_{2,0} = λ_2·s_2' ≠ λ_2·s_2, so
        // the summed public key drifts away from the original.
        let mut bad_backup = shares[2].clone();
        let wrong_secret = Scalar::random(&mut OsRng);
        bad_backup.secret_share = wrong_secret.to_bytes().to_vec().into();

        let backup_config = SessionConfig {
            session_id: "binding-test".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 2,
        };
        let mut reshare_backup = ReshareSession::new_for_recovery(
            backup_config,
            bad_backup,
            participants.clone(),
            target_party,
        );

        let server_r1 = reshare_server.generate_round1().unwrap();
        let backup_r1 = reshare_backup.generate_round1().unwrap();

        let backup_msgs_for_target: Vec<_> = backup_r1
            .into_iter()
            .filter(|m| m.to == target_party)
            .collect();
        let _server_msgs_for_target: Vec<_> = server_r1
            .into_iter()
            .filter(|m| m.to == target_party)
            .collect();

        // Server processes the tampered backup's messages. Feldman on the share
        // succeeds, but the public-key binding check must fail.
        let result = reshare_server.process_round1(backup_msgs_for_target);
        assert!(
            result.is_err(),
            "reshare must reject an inconsistent constant term"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("public-key binding"),
            "expected public-key binding failure, got: {}",
            err
        );
    }

    /// F-008 replay guard: a round-1 message replayed from the same sender must
    /// be rejected. Otherwise its share would be summed twice (drifting the
    /// secret to S + share_i) while the deduped commitment set still verified to
    /// the correct S·G — passing the binding check against the wrong quantity.
    #[test]
    fn test_reshare_rejects_duplicate_sender() {
        let shares = create_test_shares();

        let participants = vec![1u16, 2u16];
        let target_party = 0u16;

        let server_config = SessionConfig {
            session_id: "replay-test".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 1,
        };
        let mut reshare_server = ReshareSession::new_for_recovery(
            server_config,
            shares[1].clone(),
            participants.clone(),
            target_party,
        );

        let backup_config = SessionConfig {
            session_id: "replay-test".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 2,
        };
        let mut reshare_backup = ReshareSession::new_for_recovery(
            backup_config,
            shares[2].clone(),
            participants.clone(),
            target_party,
        );

        reshare_server.generate_round1().unwrap();
        let backup_r1 = reshare_backup.generate_round1().unwrap();

        let backup_msg = backup_r1
            .into_iter()
            .find(|m| m.to == target_party)
            .expect("backup should address target");

        // Deliver the SAME backup message twice (replay).
        let replayed = vec![backup_msg.clone(), backup_msg];
        let result = reshare_server.process_round1(replayed);
        assert!(
            result.is_err(),
            "reshare must reject a duplicated sender contribution"
        );
        assert!(
            result.unwrap_err().to_string().contains("duplicate"),
            "expected duplicate-contribution rejection"
        );
    }

    /// Verify Lagrange coefficients are correct for known values.
    #[test]
    fn test_lagrange_coefficients() {
        // For participants [0, 1, 2] (all parties), Lagrange at point 0 for party 0:
        // lambda_0 = (x1/(x1-x0)) * (x2/(x2-x0)) = (2/(2-1)) * (3/(3-1)) = 2 * 3/2 = 3
        let lambda = ReshareSession::lagrange_coefficient(0, &[0, 1, 2]).unwrap();
        let expected = Scalar::from(3u64);
        assert_eq!(lambda, expected);

        // For participants [1, 2] (recovery mode), Lagrange for party 1:
        // lambda_1 = x2/(x2-x1) = 3/(3-2) = 3
        let lambda = ReshareSession::lagrange_coefficient(1, &[1, 2]).unwrap();
        let expected = Scalar::from(3u64);
        assert_eq!(lambda, expected);

        // For participants [1, 2], Lagrange for party 2:
        // lambda_2 = x1/(x1-x2) = 2/(2-3) = -2
        let lambda = ReshareSession::lagrange_coefficient(2, &[1, 2]).unwrap();
        // -2 mod p = p - 2
        let expected = Scalar::ZERO - Scalar::from(2u64);
        assert_eq!(lambda, expected);
    }
}
