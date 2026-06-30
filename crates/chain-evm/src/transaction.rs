use alloy_consensus::SignableTransaction;
use alloy_consensus::TxEip1559;
use alloy_primitives::{Address, B256, Bytes, TxKind, U256};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::signer::MpcSigner;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub to: Address,
    pub value: U256,
    pub data: Vec<u8>,
    pub chain_id: u64,
    pub gas_limit: Option<u64>,
    pub nonce: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
    pub l1_data_fee: Option<u128>,
    pub estimated_cost_wei: U256,
    pub estimated_cost_usd: Option<f64>,
}

/// Build an unsigned EIP-1559 transaction from request + gas params.
pub fn build_unsigned_eip1559(tx: &TransactionRequest, gas: &GasEstimate, nonce: u64) -> TxEip1559 {
    TxEip1559 {
        chain_id: tx.chain_id,
        nonce,
        gas_limit: gas.gas_limit,
        max_fee_per_gas: gas.max_fee_per_gas,
        max_priority_fee_per_gas: gas.max_priority_fee_per_gas,
        to: TxKind::Call(tx.to),
        value: tx.value,
        access_list: Default::default(),
        input: Bytes::copy_from_slice(&tx.data),
    }
}

/// Fully-specified EIP-1559 transaction fields, as supplied by a client that
/// wants the MPC server to co-sign. Every field that affects the signing hash
/// is explicit so the server can reconstruct the transaction byte-for-byte and
/// recompute the signature hash independently — never trusting a client-supplied
/// digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eip1559Fields {
    pub chain_id: u64,
    pub nonce: u64,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
    /// Recipient. `None` means contract creation.
    pub to: Option<Address>,
    pub value: U256,
    #[serde(default)]
    pub data: Vec<u8>,
}

/// Recompute the EIP-1559 signature hash from fully-specified transaction
/// fields. This is the digest the client device must have signed; the MPC
/// server recomputes it server-side to enforce that it only ever contributes
/// a signature share for a transaction whose contents it has actually seen.
pub fn eip1559_signing_hash(fields: &Eip1559Fields) -> B256 {
    let tx = TxEip1559 {
        chain_id: fields.chain_id,
        nonce: fields.nonce,
        gas_limit: fields.gas_limit,
        max_fee_per_gas: fields.max_fee_per_gas,
        max_priority_fee_per_gas: fields.max_priority_fee_per_gas,
        to: match fields.to {
            Some(addr) => TxKind::Call(addr),
            None => TxKind::Create,
        },
        value: fields.value,
        access_list: Default::default(),
        input: Bytes::copy_from_slice(&fields.data),
    };
    tx.signature_hash()
}

/// Authoritative `(to, value, chain_id)` decoded from a signed, EIP-2718
/// raw transaction. The recipient and value are taken from the bytes that
/// were actually signed and broadcast — not from any caller-supplied side
/// channel — so callers (e.g. audit/history recording) cannot be fed values
/// that disagree with what hit the chain. For an ERC-20 transfer `to` is the
/// token contract and `value` is 0; decoding the inner transfer is the
/// caller's concern.
#[derive(Debug, Clone)]
pub struct DecodedRawTx {
    pub to: Option<Address>,
    pub value: U256,
    pub chain_id: Option<u64>,
}

/// Decode a signed raw transaction (hex with or without `0x`) into its
/// authoritative fields. Returns `None` if the input is not a well-formed
/// EIP-2718 transaction envelope.
pub fn decode_raw_tx(raw_tx: &str) -> Option<DecodedRawTx> {
    use alloy_consensus::TxEnvelope;
    use alloy_consensus::private::alloy_eips::eip2718::Decodable2718;
    use alloy_consensus::Transaction as _;

    let bytes = hex::decode(raw_tx.strip_prefix("0x").unwrap_or(raw_tx)).ok()?;
    let envelope = TxEnvelope::decode_2718(&mut bytes.as_slice()).ok()?;

    let to = match envelope.kind() {
        TxKind::Call(addr) => Some(addr),
        TxKind::Create => None,
    };
    Some(DecodedRawTx {
        to,
        value: envelope.value(),
        chain_id: envelope.chain_id(),
    })
}

/// Build and sign an EIP-1559 transaction, returning the RLP-encoded bytes.
pub fn sign_eip1559_tx(
    tx: &TransactionRequest,
    gas: &GasEstimate,
    nonce: u64,
    signer: &MpcSigner,
) -> Result<(Vec<u8>, B256), TransactionError> {
    let unsigned = build_unsigned_eip1559(tx, gas, nonce);
    let sig_hash = unsigned.signature_hash();

    let alloy_sig = signer
        .sign_hash_inner(&sig_hash)
        .map_err(|e| TransactionError::SigningFailed(e.to_string()))?;

    let signed = unsigned.into_signed(alloy_sig);
    let tx_hash = *signed.hash();

    let mut encoded = Vec::new();
    signed.eip2718_encode(&mut encoded);

    Ok((encoded, tx_hash))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub return_data: Vec<u8>,
    pub gas_used: u64,
    pub state_changes: Vec<StateChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub address: Address,
    pub token: Option<String>,
    pub balance_change: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("gas estimation failed: {0}")]
    GasEstimation(String),

    #[error("signing failed: {0}")]
    SigningFailed(String),

    #[error("simulation failed: {0}")]
    SimulationFailed(String),

    #[error("nonce too low")]
    NonceTooLow,

    #[error("insufficient funds")]
    InsufficientFunds,

    #[error("RPC error: {0}")]
    Rpc(String),
}

/// Query the next nonce for an address via eth_getTransactionCount.
pub async fn get_nonce(
    client: &Client,
    rpc_url: &str,
    address: Address,
) -> Result<u64, TransactionError> {
    let addr_hex = format!("0x{}", hex::encode(address.as_slice()));

    let rpc_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionCount",
        "params": [addr_hex, "latest"],
        "id": 1
    });

    let resp = client
        .post(rpc_url)
        .json(&rpc_body)
        .send()
        .await
        .map_err(|e| TransactionError::Rpc(e.to_string()))?;

    let rpc_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| TransactionError::Rpc(e.to_string()))?;

    if let Some(err) = rpc_resp.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown RPC error");
        return Err(TransactionError::Rpc(msg.to_string()));
    }

    let nonce_hex = rpc_resp
        .get("result")
        .and_then(|r| r.as_str())
        .ok_or_else(|| TransactionError::Rpc("no result in response".into()))?;

    let nonce = u64::from_str_radix(nonce_hex.strip_prefix("0x").unwrap_or(nonce_hex), 16)
        .map_err(|e| TransactionError::Rpc(format!("failed to parse nonce: {}", e)))?;

    Ok(nonce)
}

/// Broadcast a signed transaction via eth_sendRawTransaction.
pub async fn broadcast_tx(
    client: &Client,
    rpc_url: &str,
    signed_tx: &[u8],
) -> Result<B256, TransactionError> {
    let tx_hex = format!("0x{}", hex::encode(signed_tx));

    let rpc_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_sendRawTransaction",
        "params": [tx_hex],
        "id": 1
    });

    let resp = client
        .post(rpc_url)
        .json(&rpc_body)
        .send()
        .await
        .map_err(|e| TransactionError::Rpc(e.to_string()))?;

    let rpc_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| TransactionError::Rpc(e.to_string()))?;

    if let Some(err) = rpc_resp.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown RPC error");
        return Err(TransactionError::Rpc(msg.to_string()));
    }

    let tx_hash_hex = rpc_resp
        .get("result")
        .and_then(|r| r.as_str())
        .ok_or_else(|| TransactionError::Rpc("no result in response".into()))?;

    let tx_hash_bytes = hex::decode(tx_hash_hex.strip_prefix("0x").unwrap_or(tx_hash_hex))
        .map_err(|e| TransactionError::Rpc(format!("failed to parse tx hash: {}", e)))?;

    let tx_hash = B256::from_slice(&tx_hash_bytes);

    Ok(tx_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signer::MpcSigner;
    use mpc_core::dkls23::{SessionConfig, dkg::DkgSession};

    fn test_signer() -> MpcSigner {
        let config = SessionConfig {
            session_id: "tx-test".into(),
            threshold: 2,
            total_parties: 3,
            party_index: 0,
        };
        let mut dkg = DkgSession::new(config);
        let shares = dkg.run_local().unwrap();
        let eth_addr = shares[0].eth_address();

        MpcSigner::from_shares(
            Address::from_slice(&eth_addr),
            84532,
            vec![0, 1],
            vec![shares[0].clone(), shares[1].clone()],
        )
    }

    #[test]
    fn test_build_unsigned_eip1559() {
        let tx_req = TransactionRequest {
            to: Address::ZERO,
            value: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
            data: vec![],
            chain_id: 84532,
            gas_limit: None,
            nonce: None,
        };
        let gas = GasEstimate {
            gas_limit: 21000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 100_000_000,
            l1_data_fee: None,
            estimated_cost_wei: U256::ZERO,
            estimated_cost_usd: None,
        };
        let unsigned = build_unsigned_eip1559(&tx_req, &gas, 0);
        assert_eq!(unsigned.chain_id, 84532);
        assert_eq!(unsigned.gas_limit, 21000);
        assert_eq!(unsigned.nonce, 0);
    }

    #[test]
    fn test_sign_eip1559_tx() {
        let signer = test_signer();
        let tx_req = TransactionRequest {
            to: Address::ZERO,
            value: U256::from(1_000_000_000_000_000_000u128),
            data: vec![],
            chain_id: 84532,
            gas_limit: None,
            nonce: None,
        };
        let gas = GasEstimate {
            gas_limit: 21000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 100_000_000,
            l1_data_fee: None,
            estimated_cost_wei: U256::ZERO,
            estimated_cost_usd: None,
        };
        let (encoded, tx_hash) = sign_eip1559_tx(&tx_req, &gas, 0, &signer).unwrap();

        // EIP-1559 type prefix
        assert_eq!(encoded[0], 0x02);
        assert!(encoded.len() > 1);
        assert_ne!(tx_hash, B256::ZERO);
    }

    #[test]
    fn decode_raw_tx_recovers_authoritative_fields() {
        let signer = test_signer();
        let recipient: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();
        let tx_req = TransactionRequest {
            to: recipient,
            value: U256::from(7_000_000_000_000_000_000u128), // 7 ETH
            data: vec![],
            chain_id: 84532,
            gas_limit: Some(21000),
            nonce: Some(5),
        };
        let gas = GasEstimate {
            gas_limit: 21000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 100_000_000,
            l1_data_fee: None,
            estimated_cost_wei: U256::ZERO,
            estimated_cost_usd: None,
        };
        let (encoded, _) = sign_eip1559_tx(&tx_req, &gas, 5, &signer).unwrap();
        let raw_hex = format!("0x{}", hex::encode(&encoded));

        let decoded = decode_raw_tx(&raw_hex).expect("should decode signed tx");
        assert_eq!(decoded.to, Some(recipient));
        assert_eq!(decoded.value, U256::from(7_000_000_000_000_000_000u128));
        assert_eq!(decoded.chain_id, Some(84532));
    }

    #[test]
    fn decode_raw_tx_rejects_garbage() {
        assert!(decode_raw_tx("0xdeadbeef").is_none());
        assert!(decode_raw_tx("not hex").is_none());
        assert!(decode_raw_tx("").is_none());
    }

    #[test]
    fn test_build_unsigned_eip1559_with_different_params() {
        let tx_req = TransactionRequest {
            to: "0x1234567890123456789012345678901234567890"
                .parse()
                .unwrap(),
            value: U256::from(5_000_000_000_000_000_000u128), // 5 ETH
            data: vec![0xde, 0xad, 0xbe, 0xef],
            chain_id: 1,
            gas_limit: Some(50000),
            nonce: Some(42),
        };
        let gas = GasEstimate {
            gas_limit: 50000,
            max_fee_per_gas: 2_000_000_000,
            max_priority_fee_per_gas: 500_000_000,
            l1_data_fee: None,
            estimated_cost_wei: U256::ZERO,
            estimated_cost_usd: None,
        };
        let unsigned = build_unsigned_eip1559(&tx_req, &gas, 42);

        assert_eq!(unsigned.chain_id, 1);
        assert_eq!(unsigned.nonce, 42);
        assert_eq!(unsigned.gas_limit, 50000);
        assert_eq!(unsigned.value, U256::from(5_000_000_000_000_000_000u128));
        assert_eq!(unsigned.input.len(), 4);
    }

    #[test]
    fn test_build_unsigned_eip1559_with_zero_value() {
        let tx_req = TransactionRequest {
            to: Address::ZERO,
            value: U256::ZERO,
            data: vec![],
            chain_id: 8453,
            gas_limit: None,
            nonce: None,
        };
        let gas = GasEstimate {
            gas_limit: 21000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 100_000_000,
            l1_data_fee: None,
            estimated_cost_wei: U256::ZERO,
            estimated_cost_usd: None,
        };
        let unsigned = build_unsigned_eip1559(&tx_req, &gas, 0);

        assert_eq!(unsigned.value, U256::ZERO);
        assert!(unsigned.input.is_empty());
    }

    #[test]
    fn test_build_unsigned_eip1559_gas_estimates() {
        let tx_req = TransactionRequest {
            to: Address::ZERO,
            value: U256::from(1_000_000_000_000_000_000u128),
            data: vec![],
            chain_id: 42161,
            gas_limit: None,
            nonce: None,
        };
        let gas = GasEstimate {
            gas_limit: 100000,
            max_fee_per_gas: 5_000_000_000,
            max_priority_fee_per_gas: 1_000_000_000,
            l1_data_fee: Some(50_000_000_000_000u128),
            estimated_cost_wei: U256::from(550_000_000_000_000u128),
            estimated_cost_usd: Some(1.2),
        };
        let unsigned = build_unsigned_eip1559(&tx_req, &gas, 5);

        assert_eq!(unsigned.max_fee_per_gas, 5_000_000_000);
        assert_eq!(unsigned.max_priority_fee_per_gas, 1_000_000_000);
        assert_eq!(unsigned.gas_limit, 100000);
    }

    #[test]
    fn test_transaction_request_with_contract_call() {
        let tx_req = TransactionRequest {
            to: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                .parse()
                .unwrap(), // USDC
            value: U256::ZERO,
            data: vec![
                0xa9, 0x05, 0x9c, 0xbb, // transfer(address,uint256)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x12, 0x34, 0x56, 0x78,
                0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34,
                0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90,
            ],
            chain_id: 1,
            gas_limit: Some(65000),
            nonce: Some(10),
        };

        assert_eq!(tx_req.data.len(), 36);
        assert_eq!(&tx_req.data[0..4], &[0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn test_sign_eip1559_tx_different_chains() {
        let signer_base = test_signer();

        for chain_id in [1u64, 8453, 42161, 10, 56] {
            let tx_req = TransactionRequest {
                to: Address::ZERO,
                value: U256::from(100_000_000_000_000_000u128), // 0.1 ETH
                data: vec![],
                chain_id,
                gas_limit: None,
                nonce: None,
            };
            let gas = GasEstimate {
                gas_limit: 21000,
                max_fee_per_gas: 1_000_000_000,
                max_priority_fee_per_gas: 100_000_000,
                l1_data_fee: None,
                estimated_cost_wei: U256::ZERO,
                estimated_cost_usd: None,
            };

            let result = sign_eip1559_tx(&tx_req, &gas, 0, &signer_base);
            assert!(result.is_ok(), "signing should work for chain {}", chain_id);

            let (encoded, tx_hash) = result.unwrap();
            assert_eq!(encoded[0], 0x02, "should be EIP-1559 type for chain {}", chain_id);
            assert_ne!(tx_hash, B256::ZERO);
        }
    }

    /// The signing gate hinges on the server's `eip1559_signing_hash` producing
    /// the exact digest the mobile device hashes (keccak256 of
    /// `0x02 || rlp([chain_id, nonce, max_priority_fee, max_fee, gas, to, value,
    /// data, access_list])`). This pins that contract two ways:
    ///   1. it equals alloy's own `signature_hash()` for identical fields
    ///      (the path the rest of signing already uses), and
    ///   2. it equals a fixed known-answer constant, so any field-order or
    ///      encoding regression on either side is caught.
    #[test]
    fn signing_hash_matches_alloy_and_known_answer() {
        let to: Address = "0x1111111111111111111111111111111111111111".parse().unwrap();
        let fields = Eip1559Fields {
            chain_id: 8453,
            nonce: 7,
            gas_limit: 21000,
            max_fee_per_gas: 2_000_000_000,
            max_priority_fee_per_gas: 1_000_000_000,
            to: Some(to),
            value: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
            data: vec![],
        };

        // (1) Consistency with the existing signing path.
        let tx_req = TransactionRequest {
            to,
            value: U256::from(1_000_000_000_000_000_000u128),
            data: vec![],
            chain_id: 8453,
            gas_limit: None,
            nonce: None,
        };
        let gas = GasEstimate {
            gas_limit: 21000,
            max_fee_per_gas: 2_000_000_000,
            max_priority_fee_per_gas: 1_000_000_000,
            l1_data_fee: None,
            estimated_cost_wei: U256::ZERO,
            estimated_cost_usd: None,
        };
        let via_alloy = build_unsigned_eip1559(&tx_req, &gas, 7).signature_hash();
        let via_helper = eip1559_signing_hash(&fields);
        assert_eq!(
            via_helper, via_alloy,
            "eip1559_signing_hash must match alloy signature_hash for identical fields"
        );

        // (2) Known-answer: locks the exact bytes the mobile client must keccak.
        // If this constant ever needs updating, the client RLP layout changed
        // and BOTH sides must be re-verified together.
        let expected = "0x11474923bf3b50ea56ba7e0429020ff237adf46e4edb9215e6cc6a9e98fad55d";
        assert_eq!(format!("{:?}", via_helper), expected);
    }
}
