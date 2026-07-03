class SignResult {
  final List<int> signature;
  final String? sessionId;

  SignResult({required this.signature, this.sessionId});
}

/// Structured EIP-1559 transaction fields accompanying an MPC sign request.
///
/// The server uses these to independently recompute the signing hash and run
/// the policy engine before contributing its signature share, so they MUST be
/// the exact fields the device hashed. Field names/encodings mirror the
/// backend `Eip1559Fields`.
class SignTxFields {
  final int chainId;
  final int nonce;
  final int gasLimit;
  final BigInt maxFeePerGas;
  final BigInt maxPriorityFeePerGas;

  /// Recipient as 0x-prefixed hex; null for contract creation.
  final String? to;
  final BigInt value;

  /// Call data as 0x-prefixed hex (or "0x" when empty).
  final String data;

  SignTxFields({
    required this.chainId,
    required this.nonce,
    required this.gasLimit,
    required this.maxFeePerGas,
    required this.maxPriorityFeePerGas,
    required this.to,
    required this.value,
    required this.data,
  });

  /// JSON object matching the backend `tx` field in the sign Round 1 payload.
  Map<String, dynamic> toJson() => {
        'chain_id': chainId,
        'nonce': nonce,
        'gas_limit': gasLimit,
        'max_fee_per_gas': '0x${maxFeePerGas.toRadixString(16)}',
        'max_priority_fee_per_gas': '0x${maxPriorityFeePerGas.toRadixString(16)}',
        'to': to,
        'value': '0x${value.toRadixString(16)}',
        'data': data,
      };
}

abstract class WalletService {
  Future<String> getAddress();
  Future<bool> hasWallet();
  Future<void> deleteWallet();

  /// MPC distributed signature: returns 65 bytes (r[32] || s[32] || v[1]).
  ///
  /// [txFields] carries the structured transaction the server re-derives the
  /// signing hash from. It is required for real transaction signing; callers
  /// that sign a bare hash (legacy/testing) may omit it, but the server will
  /// reject such requests on the protected signing path.
  ///
  /// [walletId] identifies which wallet's key share to sign with. It MUST be
  /// passed for multi-wallet signing: the server binds each stored shard to
  /// its owning user+wallet (AAD), so omitting it makes the server decrypt
  /// with the wrong AAD and fail with an aead error.
  Future<List<int>> sign(List<int> msgHash, {SignTxFields? txFields, String? walletId});

  /// MPC distributed signature with session tracking.
  Future<SignResult> signWithSession(List<int> msgHash, {SignTxFields? txFields, String? walletId});
}

