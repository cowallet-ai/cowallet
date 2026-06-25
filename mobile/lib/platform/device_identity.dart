import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:convert/convert.dart';

import 'se_manager.dart';
import 'sb_manager.dart';

/// Abstraction over the platform hardware-backed device identity key used for
/// challenge-response login.
///
/// - iOS: Secure Enclave **P-256** key (`alg == "p256"`), public key exported
///   as 33-byte compressed SEC1.
/// - Android: StrongBox/Keystore **RSA-2048** key (`alg == "rsa"`), public key
///   exported as X.509 SubjectPublicKeyInfo DER.
///
/// Both are created during onboarding via `initializeWallet`. The hardware
/// hashes the message with SHA-256 internally before signing, which the
/// backend mirrors when verifying.
class DeviceIdentity {
  /// Algorithm tag for the current platform's hardware key.
  /// Returns null on unsupported platforms.
  static String? get algorithm {
    if (Platform.isIOS) return 'p256';
    if (Platform.isAndroid) return 'rsa';
    return null;
  }

  /// The hardware device public key as hex (no 0x prefix), or null if the
  /// device key has not been initialized / platform unsupported.
  static Future<String?> publicKeyHex() async {
    try {
      final List<int> raw;
      if (Platform.isIOS) {
        raw = await SecureEnclaveManager().getDeviceShardPublicKey();
      } else if (Platform.isAndroid) {
        raw = await StrongBoxManager().getDeviceShardPublicKey();
      } else {
        return null;
      }
      if (raw.isEmpty) return null;
      return hex.encode(raw);
    } catch (_) {
      return null;
    }
  }

  /// Sign a login challenge with the hardware key, returning the signature as
  /// hex (no 0x prefix). The hardware applies SHA-256 internally.
  static Future<String> signChallenge(
    List<int> challenge,
    String reason,
  ) async {
    final hashB64 = base64Encode(Uint8List.fromList(challenge));
    final String sigB64;
    if (Platform.isIOS) {
      sigB64 = await SecureEnclaveManager().signHashWithBiometric(hashB64, reason);
    } else if (Platform.isAndroid) {
      sigB64 = await StrongBoxManager().signHashWithBiometric(hashB64, reason);
    } else {
      throw UnsupportedError('Device identity signing not supported on this platform');
    }
    return hex.encode(base64Decode(sigB64));
  }
}
