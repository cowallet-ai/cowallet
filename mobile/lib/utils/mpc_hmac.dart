import 'dart:typed_data';
import 'package:pointycastle/api.dart' show KeyParameter;
import 'package:pointycastle/macs/hmac.dart';
import 'package:pointycastle/digests/sha256.dart';

/// Computes the per-session HMAC that authenticates server-bound MPC messages
/// (F-004). Must match the server exactly:
///   HMAC-SHA256(sessionHmacKey, session_id_utf8 ‖ round_i16_le ‖ payload)
/// returned as lowercase hex.
///
/// [sessionHmacKeyHex] is the `hmac_key` returned (hex) from create_session.
/// [round] is the protocol round (server treats it as i16, little-endian).
class MpcHmac {
  static String compute({
    required String sessionHmacKeyHex,
    required String sessionId,
    required int round,
    required List<int> payload,
  }) {
    final key = _hexDecode(sessionHmacKeyHex);
    final hmac = HMac(SHA256Digest(), 64)..init(KeyParameter(key));

    // session_id as UTF-8 bytes
    hmac.update(_utf8(sessionId), 0, sessionId.length);
    // round as 2-byte little-endian (Rust i16::to_le_bytes)
    final roundBytes = Uint8List(2);
    roundBytes[0] = round & 0xff;
    roundBytes[1] = (round >> 8) & 0xff;
    hmac.update(roundBytes, 0, roundBytes.length);
    // payload bytes
    final payloadBytes = Uint8List.fromList(payload);
    hmac.update(payloadBytes, 0, payloadBytes.length);

    final out = Uint8List(hmac.macSize);
    hmac.doFinal(out, 0);
    return _hexEncode(out);
  }

  static Uint8List _utf8(String s) => Uint8List.fromList(s.codeUnits);

  static Uint8List _hexDecode(String hex) {
    final clean = hex.startsWith('0x') ? hex.substring(2) : hex;
    final out = Uint8List(clean.length ~/ 2);
    for (var i = 0; i < out.length; i++) {
      out[i] = int.parse(clean.substring(i * 2, i * 2 + 2), radix: 16);
    }
    return out;
  }

  static String _hexEncode(Uint8List bytes) {
    final sb = StringBuffer();
    for (final b in bytes) {
      sb.write(b.toRadixString(16).padLeft(2, '0'));
    }
    return sb.toString();
  }
}
