use crate::errors::{MpcError, Result};
use snow::{HandshakeState, TransportState};
use zeroize::Zeroize;

/// Noise_XX encrypted channel for MPC message transport.
///
/// Provides authenticated encryption with forward secrecy between
/// two MPC parties. Uses the `snow` crate (Noise Protocol Framework).
///
/// Handshake pattern: XX (mutual authentication)
/// Cipher: ChaChaPoly
/// DH: X25519
/// Hash: SHA256
///
/// # Identity pinning
///
/// Each party uses a LONG-TERM static X25519 keypair as its identity, and
/// pins the EXPECTED peer static public key. After the handshake completes,
/// the negotiated remote static key is compared against the pinned value and
/// the channel is rejected on mismatch. This prevents man-in-the-middle
/// attacks: without pinning, Noise_XX authenticates "some peer" but not "the
/// peer we intended to talk to" (F-006).
///
/// The static identity keys MUST be provisioned out-of-band by the caller
/// (e.g. the backend / mobile app), not generated per session.
pub struct NoiseChannel {
    state: ChannelState,
    peer_public_key: Option<[u8; 32]>,
    /// The peer static public key we expect to negotiate (identity pinning).
    expected_peer_public_key: [u8; 32],
}

enum ChannelState {
    HandshakeInitiator(HandshakeState),
    HandshakeResponder(HandshakeState),
    Transport(TransportState),
    Temporary,
}

impl Zeroize for ChannelState {
    fn zeroize(&mut self) {
        // HandshakeState and TransportState handle their own zeroization
        // via snow's internal zero-on-drop guarantees
    }
}

impl Drop for NoiseChannel {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Zeroize for NoiseChannel {
    fn zeroize(&mut self) {
        self.peer_public_key.zeroize();
        self.expected_peer_public_key.zeroize();
        // Replace state with a dummy to clear it and trigger drop of inner state
        self.state = ChannelState::Temporary;
    }
}

/// Noise protocol parameters: XX pattern with 25519, ChaChaPoly, SHA256
const NOISE_PARAMS: &str = "Noise_XX_25519_ChaChaPoly_SHA256";

impl NoiseChannel {
    /// Create a new Noise channel builder.
    fn builder() -> snow::Builder<'static> {
        snow::Builder::new(NOISE_PARAMS.parse().expect("valid Noise parameters"))
    }

    /// Initiate a Noise_XX handshake as the initiator.
    ///
    /// - `local_static_private`: this party's LONG-TERM X25519 static private key
    ///   (32 bytes), provisioned out-of-band — NOT generated per session.
    /// - `expected_peer_public`: the responder's pinned static public key (32 bytes).
    ///   After the handshake the negotiated remote static is compared to this and
    ///   rejected on mismatch.
    ///
    /// Returns (channel, first_handshake_message) to send to the responder.
    pub fn initiate(
        local_static_private: &[u8; 32],
        expected_peer_public: &[u8; 32],
    ) -> Result<(Self, Vec<u8>)> {
        let mut handshake = Self::builder()
            .local_private_key(local_static_private)
            .build_initiator()
            .map_err(|e| MpcError::Transport(format!("failed to build initiator: {}", e)))?;

        // First handshake message: -> e
        let mut buf = vec![0u8; 65535];
        let len = handshake
            .write_message(&[], &mut buf)
            .map_err(|e| MpcError::Transport(format!("handshake write failed: {}", e)))?;
        buf.truncate(len);

        Ok((
            Self {
                state: ChannelState::HandshakeInitiator(handshake),
                peer_public_key: None,
                expected_peer_public_key: *expected_peer_public,
            },
            buf,
        ))
    }

    /// Respond to a Noise_XX handshake as the responder.
    ///
    /// - `local_static_private`: this party's LONG-TERM X25519 static private key
    ///   (32 bytes), provisioned out-of-band — NOT generated per session.
    /// - `expected_peer_public`: the initiator's pinned static public key (32 bytes).
    ///
    /// Receives the initiator's first message, returns (channel, response_message).
    pub fn respond(
        initiator_msg: &[u8],
        local_static_private: &[u8; 32],
        expected_peer_public: &[u8; 32],
    ) -> Result<(Self, Vec<u8>)> {
        let mut handshake = Self::builder()
            .local_private_key(local_static_private)
            .build_responder()
            .map_err(|e| MpcError::Transport(format!("failed to build responder: {}", e)))?;

        // Process initiator's first message
        handshake
            .read_message(initiator_msg, &mut [])
            .map_err(|e| MpcError::Transport(format!("failed to read initiator message: {}", e)))?;

        // Send response: <- e, ee, s, es
        let mut buf = vec![0u8; 65535];
        let len = handshake
            .write_message(&[], &mut buf)
            .map_err(|e| MpcError::Transport(format!("handshake write failed: {}", e)))?;
        buf.truncate(len);

        Ok((
            Self {
                state: ChannelState::HandshakeResponder(handshake),
                peer_public_key: None,
                expected_peer_public_key: *expected_peer_public,
            },
            buf,
        ))
    }

    /// Complete the handshake as initiator after receiving responder's message.
    ///
    /// This processes the responder's message and generates the final initiator message.
    /// Returns the final message to send to the responder.
    /// After calling this, the channel is ready for transport encryption.
    pub fn complete_handshake_initiator(&mut self, responder_msg: &[u8]) -> Result<Vec<u8>> {
        // First process the responder's message
        let (peer_pub, final_msg) = {
            let handshake = match &mut self.state {
                ChannelState::HandshakeInitiator(h) => h,
                _ => {
                    return Err(MpcError::Transport(
                        "not in initiator handshake state".into(),
                    ))
                }
            };

            // Process responder's message: <- e, ee, s, es
            handshake
                .read_message(responder_msg, &mut [])
                .map_err(|e| {
                    MpcError::Transport(format!("failed to read responder message: {}", e))
                })?;

            // Get peer's static public key from handshake
            let pubkey = handshake
                .get_remote_static()
                .ok_or_else(|| MpcError::Transport("no remote public key after handshake".into()))?
                .to_vec();

            // Write the final initiator message (-> s, se)
            let mut buf = vec![0u8; 65535];
            let len = handshake
                .write_message(&[], &mut buf)
                .map_err(|e| MpcError::Transport(format!("handshake write failed: {}", e)))?;
            buf.truncate(len);

            (pubkey, buf)
        };

        let mut key = [0u8; 32];
        key.copy_from_slice(&peer_pub);

        // Identity pinning: reject if the negotiated remote static does not
        // match the expected peer key (F-006).
        self.verify_pinned_peer(&key)?;
        self.peer_public_key = Some(key);

        // Now transition to transport mode
        let handshake = match std::mem::replace(&mut self.state, ChannelState::Temporary) {
            ChannelState::HandshakeInitiator(h) => h,
            _ => {
                return Err(MpcError::Transport(
                    "not in initiator handshake state".into(),
                ))
            }
        };

        let transport = handshake
            .into_transport_mode()
            .map_err(|e| MpcError::Transport(format!("failed to enter transport mode: {}", e)))?;

        self.state = ChannelState::Transport(transport);
        Ok(final_msg)
    }

    /// Complete the handshake as responder after receiving initiator's final message.
    ///
    /// After calling this, the channel is ready for transport encryption.
    pub fn complete_handshake_responder(&mut self, initiator_final_msg: &[u8]) -> Result<()> {
        // First process the message while we still have the handshake state
        let peer_pub = {
            let handshake = match &mut self.state {
                ChannelState::HandshakeResponder(h) => h,
                _ => {
                    return Err(MpcError::Transport(
                        "not in responder handshake state".into(),
                    ))
                }
            };

            // Process initiator's final message: -> s, se
            handshake
                .read_message(initiator_final_msg, &mut [])
                .map_err(|e| MpcError::Transport(format!("failed to read final message: {}", e)))?;

            // Get peer's static public key from handshake
            handshake
                .get_remote_static()
                .ok_or_else(|| MpcError::Transport("no remote public key after handshake".into()))?
                .to_vec()
        };

        let mut key = [0u8; 32];
        key.copy_from_slice(&peer_pub);

        // Identity pinning: reject if the negotiated remote static does not
        // match the expected peer key (F-006).
        self.verify_pinned_peer(&key)?;
        self.peer_public_key = Some(key);

        // Now transition to transport mode
        let handshake = match std::mem::replace(&mut self.state, ChannelState::Temporary) {
            ChannelState::HandshakeResponder(h) => h,
            _ => {
                return Err(MpcError::Transport(
                    "not in responder handshake state".into(),
                ))
            }
        };

        let transport = handshake
            .into_transport_mode()
            .map_err(|e| MpcError::Transport(format!("failed to enter transport mode: {}", e)))?;

        self.state = ChannelState::Transport(transport);
        Ok(())
    }

    /// Check if the handshake is complete and channel is ready for transport.
    pub fn is_ready(&self) -> bool {
        matches!(self.state, ChannelState::Transport(_))
    }

    /// Get the peer's public key if handshake is complete.
    pub fn peer_public_key(&self) -> Option<&[u8; 32]> {
        self.peer_public_key.as_ref()
    }

    /// Compare the negotiated remote static key against the pinned expected key
    /// in constant time. Returns an error on mismatch (F-006).
    fn verify_pinned_peer(&self, negotiated: &[u8; 32]) -> Result<()> {
        use k256::elliptic_curve::subtle::ConstantTimeEq;
        if bool::from(negotiated.ct_eq(&self.expected_peer_public_key)) {
            Ok(())
        } else {
            Err(MpcError::Transport(
                "peer static key does not match pinned identity".into(),
            ))
        }
    }

    /// Encrypt a message for the peer.
    ///
    /// In transport mode, each message is encrypted with an incrementing nonce.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let transport = match &mut self.state {
            ChannelState::Transport(t) => t,
            _ => return Err(MpcError::Transport("handshake not complete".into())),
        };

        // snow::TransportState::write_message returns ciphertext with MAC appended
        let mut buf = vec![0u8; plaintext.len() + 16]; // +16 for ChaChaPoly tag
        let len = transport
            .write_message(plaintext, &mut buf)
            .map_err(|e| MpcError::Transport(format!("encryption failed: {}", e)))?;
        buf.truncate(len);
        Ok(buf)
    }

    /// Decrypt a message from the peer.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let transport = match &mut self.state {
            ChannelState::Transport(t) => t,
            _ => return Err(MpcError::Transport("handshake not complete".into())),
        };

        let mut buf = vec![0u8; ciphertext.len()];
        let len = transport
            .read_message(ciphertext, &mut buf)
            .map_err(|e| MpcError::Transport(format!("decryption failed: {}", e)))?;
        buf.truncate(len);
        Ok(buf)
    }
}

/// Generate a fresh X25519 static keypair for use as a long-term identity.
///
/// Returns (private, public). In production these are provisioned and persisted
/// out-of-band; this helper exists for key setup and tests.
pub fn generate_static_keypair() -> Result<([u8; 32], [u8; 32])> {
    let kp = NoiseChannel::builder()
        .generate_keypair()
        .map_err(|e| MpcError::Transport(format!("failed to generate keypair: {}", e)))?;
    let mut private = [0u8; 32];
    let mut public = [0u8; 32];
    private.copy_from_slice(&kp.private);
    public.copy_from_slice(&kp.public);
    Ok((private, public))
}

/// Convenience function: perform a full Noise_XX handshake between two parties
/// using the supplied long-term static keypairs, with mutual identity pinning.
///
/// - `initiator_private` / `initiator_public`: initiator's static identity.
/// - `responder_private` / `responder_public`: responder's static identity.
///
/// Each side pins the other's static public key and the handshake fails if the
/// negotiated remote static does not match.
///
/// Returns (initiator_channel, responder_channel) ready for transport.
pub fn handshake_pair(
    initiator_private: &[u8; 32],
    initiator_public: &[u8; 32],
    responder_private: &[u8; 32],
    responder_public: &[u8; 32],
) -> Result<(NoiseChannel, NoiseChannel)> {
    // 1. Initiator sends first message (e), pinning the responder's identity.
    let (mut initiator, msg1) = NoiseChannel::initiate(initiator_private, responder_public)?;

    // 2. Responder receives msg1 and sends response (e, ee, s, es),
    //    pinning the initiator's identity.
    let (mut responder, msg2) = NoiseChannel::respond(&msg1, responder_private, initiator_public)?;

    // 3. Initiator receives responder's message and sends final message (s, se)
    let msg3 = initiator.complete_handshake_initiator(&msg2)?;

    // 4. Responder receives final message
    responder.complete_handshake_responder(&msg3)?;

    assert!(initiator.is_ready());
    assert!(responder.is_ready());

    Ok((initiator, responder))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroize;

    /// Set up two pinned long-term identities and complete a handshake.
    fn pinned_handshake_pair() -> (NoiseChannel, NoiseChannel) {
        let (a_priv, a_pub) = generate_static_keypair().unwrap();
        let (b_priv, b_pub) = generate_static_keypair().unwrap();
        handshake_pair(&a_priv, &a_pub, &b_priv, &b_pub).unwrap()
    }

    #[test]
    fn test_handshake_completes() {
        let (mut alice, mut bob) = pinned_handshake_pair();
        assert!(alice.is_ready());
        assert!(bob.is_ready());
    }

    #[test]
    fn test_handshake_rejects_unpinned_peer() {
        // Alice expects Bob, but the actual responder uses a DIFFERENT identity
        // than the one Alice pinned. The handshake must fail.
        let (a_priv, a_pub) = generate_static_keypair().unwrap();
        let (b_priv, _b_pub) = generate_static_keypair().unwrap();
        let (_imposter_priv, imposter_pub) = generate_static_keypair().unwrap();

        // Alice pins the imposter key but Bob presents his real key.
        let (mut alice, msg1) = NoiseChannel::initiate(&a_priv, &imposter_pub).unwrap();
        let (_bob, msg2) = NoiseChannel::respond(&msg1, &b_priv, &a_pub).unwrap();

        // Alice processes Bob's message and should reject on pin mismatch.
        let result = alice.complete_handshake_initiator(&msg2);
        assert!(result.is_err(), "alice must reject unpinned peer");
        assert!(!alice.is_ready());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (mut alice, mut bob) = pinned_handshake_pair();

        // Alice -> Bob
        let plaintext = b"Hello, MPC world!";
        let ciphertext = alice.encrypt(plaintext).unwrap();
        let decrypted = bob.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);

        // Bob -> Alice
        let response = b"Response from Bob";
        let ciphertext2 = bob.encrypt(response).unwrap();
        let decrypted2 = alice.decrypt(&ciphertext2).unwrap();
        assert_eq!(decrypted2, response);
    }

    #[test]
    fn test_multiple_messages() {
        let (mut alice, mut bob) = pinned_handshake_pair();

        for i in 0..10 {
            let msg = format!("Message {}", i);
            let ciphertext = alice.encrypt(msg.as_bytes()).unwrap();
            let decrypted = bob.decrypt(&ciphertext).unwrap();
            assert_eq!(decrypted, msg.as_bytes());
        }
    }

    #[test]
    fn test_large_message() {
        let (mut alice, mut bob) = pinned_handshake_pair();

        // 4KB message
        let large_msg = vec![0x42u8; 4096];
        let ciphertext = alice.encrypt(&large_msg).unwrap();
        let decrypted = bob.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, large_msg);
    }

    #[test]
    fn test_decrypt_fails_with_wrong_data() {
        let (mut alice, mut bob) = pinned_handshake_pair();

        let plaintext = b"Hello";
        let mut ciphertext = alice.encrypt(plaintext).unwrap();

        // Corrupt the ciphertext
        ciphertext[0] ^= 0xFF;

        let result = bob.decrypt(&ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_fails_before_handshake() {
        let (a_priv, _a_pub) = generate_static_keypair().unwrap();
        let (_b_priv, b_pub) = generate_static_keypair().unwrap();
        let (mut alice, _) = NoiseChannel::initiate(&a_priv, &b_pub).unwrap();
        assert!(!alice.is_ready());

        let result = alice.encrypt(b"too early");
        assert!(result.is_err());
    }

    #[test]
    fn test_zeroize() {
        let (mut alice, mut bob) = pinned_handshake_pair();

        // Use the channel
        let ciphertext = alice.encrypt(b"test").unwrap();
        let _ = bob.decrypt(&ciphertext).unwrap();

        // Zeroize
        alice.zeroize();
        bob.zeroize();

        // Channel should no longer be usable
        assert!(!alice.is_ready());
        assert!(!bob.is_ready());
    }
}
