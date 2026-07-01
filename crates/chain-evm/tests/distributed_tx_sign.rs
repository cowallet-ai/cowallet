//! End-to-end test: sign a real EIP-1559 transaction through the *distributed*
//! DKLS23 MPC path (generate_round1 / process_round2 / Paillier MtA) — never
//! `SignSession::new_local`, which reconstructs the full private key and is
//! test/demo only. Confirms the resulting (r, s, v) recovers to the same
//! Ethereum address the DKG public key derives, i.e. the distributed signature
//! is a valid Ethereum signature over the transaction's signing hash.
//!
//! This mirrors `mpc_core::dkls23::sign::tests::test_distributed_sign_2_parties`
//! but drives it from a concrete `TxEip1559` signing hash and asserts ecrecover.
//!
//! Uses Paillier cryptography (slow, ~60-120s), so it is `#[ignore]` by default.
//! Run with: cargo test -p chain-evm --test distributed_tx_sign -- --ignored

use alloy_consensus::{SignableTransaction, TxEip1559};
use alloy_primitives::{Address, Bytes, Signature, TxKind, B256, U256};
use mpc_core::dkls23::{
    dkg::DkgSession,
    sign::SignSession,
    KeyShare, ProtocolMessage, SessionConfig,
};

fn make_config(session_id: &str, party: u16) -> SessionConfig {
    SessionConfig {
        session_id: session_id.into(),
        threshold: 2,
        total_parties: 3,
        party_index: party,
    }
}

/// Full 3-party DKG with real round message exchange, returning the shares.
fn run_dkg(session_id: &str) -> Vec<KeyShare> {
    let mut dkg0 = DkgSession::new(make_config(session_id, 0));
    let mut dkg1 = DkgSession::new(make_config(session_id, 1));
    let mut dkg2 = DkgSession::new(make_config(session_id, 2));

    let r1_0 = dkg0.generate_round1().unwrap();
    let r1_1 = dkg1.generate_round1().unwrap();
    let r1_2 = dkg2.generate_round1().unwrap();
    dkg0.process_round1(vec![r1_1.clone(), r1_2.clone()]).unwrap();
    dkg1.process_round1(vec![r1_0.clone(), r1_2.clone()]).unwrap();
    dkg2.process_round1(vec![r1_0.clone(), r1_1.clone()]).unwrap();

    let r2_0 = dkg0.generate_round2().unwrap();
    let r2_1 = dkg1.generate_round2().unwrap();
    let r2_2 = dkg2.generate_round2().unwrap();

    let msgs_0: Vec<_> = vec![&r2_1, &r2_2]
        .into_iter()
        .flat_map(|m| m.iter().filter(|x| x.to == 0))
        .cloned()
        .collect();
    let msgs_1: Vec<_> = vec![&r2_0, &r2_2]
        .into_iter()
        .flat_map(|m| m.iter().filter(|x| x.to == 1))
        .cloned()
        .collect();
    let msgs_2: Vec<_> = vec![&r2_0, &r2_1]
        .into_iter()
        .flat_map(|m| m.iter().filter(|x| x.to == 2))
        .cloned()
        .collect();

    let s0 = dkg0.process_round2(msgs_0).unwrap();
    let s1 = dkg1.process_round2(msgs_1).unwrap();
    let s2 = dkg2.process_round2(msgs_2).unwrap();

    assert_eq!(s0.public_key, s1.public_key);
    assert_eq!(s1.public_key, s2.public_key);
    vec![s0, s1, s2]
}

/// Sign `msg_hash` with parties 0 (device) and 1 (server) via the distributed
/// DKLS23 path. Returns the raw 65-byte (r || s || v) signature.
fn distributed_sign(
    session_id: &str,
    device_share: &KeyShare,
    server_share: &KeyShare,
    msg_hash: [u8; 32],
) -> [u8; 65] {
    let mut sign_device =
        SignSession::new_distributed(make_config(session_id, 0), device_share.clone(), msg_hash);
    let mut sign_server =
        SignSession::new_distributed(make_config(session_id, 1), server_share.clone(), msg_hash);

    // Round 1: exchange ephemeral public keys
    let r1_device = sign_device.generate_round1().unwrap();
    let r1_server = sign_server.generate_round1().unwrap();
    sign_device.process_round1(vec![r1_server]).unwrap();
    sign_server.process_round1(vec![r1_device]).unwrap();

    // Round 2: device sends Paillier MtA request; server computes Enc(s)
    let r2_device = sign_device.generate_round2().unwrap();
    let _ = sign_server.process_round2(vec![r2_device]).unwrap();

    let server_response = sign_server
        .get_server_response()
        .expect("server should have produced a ServerSignature");

    let server_response_msg = ProtocolMessage {
        session_id: session_id.into(),
        from: 1,
        to: 0,
        round: 2,
        payload: server_response,
    };

    // Device decrypts Enc(s) into the final signature.
    let sig = sign_device.process_round2(vec![server_response_msg]).unwrap();
    sig.to_bytes()
}

/// Return the y-parity under which `sig_bytes` recovers to `expected`, if any.
/// A valid (r, s) recovers to a *different* public key for each recovery id, so
/// the test is: does SOME recovery id yield the wallet's address? This mirrors
/// `MpcSigner::sign_hash_inner`, which searches recovery ids for a match.
fn recover_parity_for(sig_bytes: &[u8; 65], msg_hash: [u8; 32], expected: Address) -> Option<bool> {
    let r = B256::from_slice(&sig_bytes[0..32]);
    let s = B256::from_slice(&sig_bytes[32..64]);
    let prehash = B256::from(msg_hash);

    for y_parity in [false, true] {
        let sig = Signature::from_scalars_and_parity(r, s, y_parity);
        if let Ok(addr) = sig.recover_address_from_prehash(&prehash) {
            if addr == expected {
                return Some(y_parity);
            }
        }
    }
    None
}

#[test]
#[ignore = "uses slow Paillier MtA (~60-120s); run with --ignored"]
fn distributed_sign_eip1559_recovers_correct_address() {
    // 1. DKG → shares + the canonical wallet address.
    let shares = run_dkg("dist-tx-sign-eip1559");
    let expected_addr = Address::from_slice(&shares[0].eth_address());

    // 2. Build a concrete EIP-1559 transfer and take its signing hash.
    let recipient: Address = "0x1234567890123456789012345678901234567890"
        .parse()
        .unwrap();
    let tx = TxEip1559 {
        chain_id: 8453, // Base mainnet
        nonce: 3,
        gas_limit: 21_000,
        max_fee_per_gas: 2_000_000_000,
        max_priority_fee_per_gas: 1_000_000_000,
        to: TxKind::Call(recipient),
        value: U256::from(500_000_000_000_000_000u128), // 0.5 ETH
        access_list: Default::default(),
        input: Bytes::new(),
    };
    let sig_hash = tx.signature_hash();
    let msg_hash: [u8; 32] = sig_hash.0;

    // 3. Sign it through the real distributed MPC path (no new_local).
    let sig_bytes = distributed_sign("dist-tx-sign-eip1559", &shares[0], &shares[1], msg_hash);

    // r and s must be non-zero.
    assert_ne!(&sig_bytes[0..32], &[0u8; 32], "r must be non-zero");
    assert_ne!(&sig_bytes[32..64], &[0u8; 32], "s must be non-zero");

    // 4. The signature must recover to the wallet's Ethereum address under one
    //    of the two y-parities — this is what makes it a valid Ethereum
    //    signature for this key.
    let y_parity = recover_parity_for(&sig_bytes, msg_hash, expected_addr)
        .expect("distributed EIP-1559 signature must ecrecover to the DKG-derived address");

    // 5. And the signed transaction must actually encode/broadcast-shape cleanly.
    let alloy_sig = Signature::from_scalars_and_parity(
        B256::from_slice(&sig_bytes[0..32]),
        B256::from_slice(&sig_bytes[32..64]),
        y_parity,
    );
    let signed = tx.into_signed(alloy_sig);
    let mut encoded = Vec::new();
    signed.eip2718_encode(&mut encoded);
    assert_eq!(encoded[0], 0x02, "must encode as an EIP-1559 (type 0x02) tx");
}
