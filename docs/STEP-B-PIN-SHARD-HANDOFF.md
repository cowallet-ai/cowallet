# Step B Handoff — PIN-only device-shard encryption (Dart wiring)

The Rust side is done and unit-tested (commit `1bcc69a`):
- `export_device_shard_encrypted(pin) -> String` (base64 blob)
- `import_device_shard_encrypted(blob, pin, publicKey) -> bool`

App-layer Argon2id + AES-256-GCM over the FULL device shard
(`secret_share || paillier_keypair`). No Secure Enclave / StrongBox → **no
biometric prompt**. Round-trip + wrong-PIN + short-PIN tests pass.

What remains is **Dart-only** but needs the FFI bindings regenerated, which
must run in your local Flutter/Rust toolchain (not available in the agent env).

---

## 1. Regenerate FFI bindings

```bash
cd /Users/mac/cat/cowallet
flutter_rust_bridge_codegen generate      # regenerates mobile/lib/bridge/frb_generated/
cd mobile && flutter pub get
```

Confirm the generated API now exposes `exportDeviceShardEncrypted` and
`importDeviceShardEncrypted` (camelCase) in `mobile/lib/bridge/frb_generated/`.

---

## 2. mobile/lib/bridge/mpc_bridge.dart — add wrappers

Next to the existing `exportDeviceShard` / `importDeviceShard`:

```dart
  /// PIN-encrypt the device shard (app-layer, no hardware key). Returns base64.
  static Future<String> exportDeviceShardEncrypted(String pin) async {
    try {
      return await frb.exportDeviceShardEncrypted(pin: pin);
    } catch (e) {
      throw MpcException('Failed to PIN-encrypt device shard: $e');
    }
  }

  /// Decrypt a PIN-encrypted device shard and load it as Party 0.
  static Future<bool> importDeviceShardEncrypted({
    required String encryptedData,
    required String pin,
    required List<int> publicKey,
  }) async {
    try {
      return await frb.importDeviceShardEncrypted(
        encryptedData: encryptedData,
        pin: pin,
        publicKey: publicKey,
      );
    } catch (e) {
      throw MpcException('Failed to load PIN-encrypted device shard: $e');
    }
  }
```

---

## 3. mobile/lib/utils/secure_storage.dart — add key

```dart
  /// PIN-encrypted device shard blob (base64). Present only when the user chose
  /// PIN protection instead of biometric/hardware.
  static const String keyPinEncryptedShard = "pin_encrypted_device_shard";
```

---

## 4. mobile/lib/services/mpc_wallet_service.dart

### 4a. Add a PIN persistence method (next to `persistDeviceShard`)

```dart
  /// Persist the device shard encrypted with the user's PIN (app-layer, no
  /// hardware key → no biometric). Used when the user picks PIN-only auth.
  Future<void> persistDeviceShardWithPin(String pin) async {
    final blob = await MpcBridge.exportDeviceShardEncrypted(pin);
    await SecureStorage.save(SecureStorage.keyPinEncryptedShard, blob);
  }
```

### 4b. Make `ensureShardLoaded` prefer the PIN blob when present

Replace the body of `ensureShardLoaded()` so it loads from the PIN blob if the
user chose PIN, else falls back to hardware:

```dart
  Future<void> ensureShardLoaded() async {
    final pubKeyHex = await SecureStorage.get('mpc_public_key');
    if (pubKeyHex == null || pubKeyHex.isEmpty) {
      throw MpcException('Public key not found');
    }
    final publicKey = List<int>.generate(
      pubKeyHex.length ~/ 2,
      (i) => int.parse(pubKeyHex.substring(i * 2, i * 2 + 2), radix: 16),
    );

    // PIN-only path: decrypt the app-layer blob with the user's PIN.
    final pinBlob = await SecureStorage.get(SecureStorage.keyPinEncryptedShard);
    if (pinBlob != null && pinBlob.isNotEmpty) {
      final pin = await _promptPinForSigning(); // your PIN entry UI
      await MpcBridge.importDeviceShardEncrypted(
        encryptedData: pinBlob,
        pin: pin,
        publicKey: publicKey,
      );
      return;
    }

    // Hardware path (biometric / device credential).
    final shardBytes = await SecureHardware.loadDeviceShard();
    if (shardBytes == null || shardBytes.isEmpty) {
      throw MpcException('Device shard not found in secure hardware');
    }
    await MpcBridge.importDeviceShard(
      shardBytes: shardBytes.toList(),
      publicKey: publicKey,
    );
  }
```

`_promptPinForSigning()` = a small dialog that returns the 6-digit PIN
(you already collect one at onboarding; verify against `wallet_pin` or just
let a wrong PIN fail the Rust decrypt).

---

## 5. mobile/lib/onboarding/onboarding_flow.dart — wire the PIN choice

Step A already routes: `creating(DKG, no store) → bio → (skip) → pin`.
Currently `_skipBio()` calls `_persistShardThen(_Stage.pin)` which persists via
the **hardware** key. For a true PIN-only path, change skip to NOT persist via
hardware, and persist with the PIN after the user sets it in `_onPinComplete`:

```dart
  void _skipBio() {
    Services.biometrics.setEnabled(false);
    _goTo(_Stage.pin);          // do NOT persist via hardware here
  }
```

Then in `_onPinComplete`, after the PIN is confirmed and saved:

```dart
      if (pin == _pinFirst) {
        await SecureStorage.save('wallet_pin', pin);
        try {
          final walletService = Services.wallet as MpcWalletService;
          await walletService.persistDeviceShardWithPin(pin);   // <-- PIN-encrypt shard
        } catch (e) {
          setState(() => _pinMismatch = true); // surface a real error UI
          return;
        }
        setState(() { _pinDone = true; _pinFirst = null; _pinInput = ''; });
        Future.delayed(const Duration(milliseconds: 600), () {
          if (mounted) _goTo(_Stage.name);
        });
      }
```

Result:
- **Enable biometrics** (bio stage) → `persistDeviceShard()` → hardware key → biometric prompt (as in step A).
- **Skip → set PIN** → `persistDeviceShardWithPin(pin)` → app-layer AES, **no biometric ever**.

---

## 6. Device test checklist (must run on real hardware)

1. **Biometric path**: create wallet → enable biometrics → confirm prompt fires
   only once, at the bio screen (not mid-DKG). Sign a tx → biometric prompt on load.
2. **PIN path**: create wallet → skip biometrics → set PIN → confirm **no biometric
   prompt** anywhere. Sign a tx → PIN dialog appears, correct PIN signs, wrong PIN fails.
3. **Restart app** in each mode → sign again → shard loads correctly.
4. **Wrong PIN** at signing → decrypt fails gracefully (no crash, retry possible).
5. Verify the signed tx `ecrecover`s to the wallet address in both modes.

---

## Notes / decisions baked in

- PIN blob format matches the backup path (version||salt||nonce||ciphertext),
  Argon2id m=64MiB/t=3/p=4 — same as the (reverted-from-backup) hardening.
- Security tradeoff (accepted): PIN-only shard has **no hardware binding** — its
  security rests on the PIN's entropy + Argon2 cost. A 6-digit PIN is brute-forceable
  offline if the encrypted blob leaks. Consider a longer PIN or rate-limited unlock
  if that threat matters. Biometric path remains the stronger option.
