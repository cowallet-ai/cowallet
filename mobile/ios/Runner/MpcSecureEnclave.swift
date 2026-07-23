// iOS Secure Enclave Channel Handler
// Handles platform channel calls for SE operations

import Flutter
import CryptoKit
import LocalAuthentication

public class MpcSecureEnclaveHandler: NSObject, FlutterPlugin {
  public static func dummy(methodCall: FlutterMethodCall, result: @escaping FlutterResult) {
    // This method is added to work around a fatal exception when a plugin is
    // registered using generics.
  }

  public static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "com.cowallet.mpc/se",
      binaryMessenger: registrar.messenger()
    )
    let instance = MpcSecureEnclaveHandler()
    registrar.addMethodCallDelegate(instance, channel: channel)
  }

  public func dummyMethodToEnforceBundling() {
    // This method is added to work around a fatal exception when the plugin is
    // registered using generics.
  }

  public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "generateKey":
      generateKey(call, result: result)
    case "getPublicKey":
      getPublicKey(call, result: result)
    case "signWithBiometric":
      signWithBiometric(call, result: result)
    case "isAvailable":
      isAvailable(result: result)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  // MARK: - Secure Enclave Operations

  /// Generate a new P-256 private key in Secure Enclave
  private func generateKey(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let keyId = args["keyId"] as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "keyId is required", details: nil))
      return
    }

    do {
      // Secure Enclave P-256 signing key. Only the opaque, non-exportable
      // dataRepresentation blob is persisted; the raw private key never leaves
      // the SE. This blob is NOT a raw EC key — it MUST be reconstructed via
      // CryptoKit (see loadSigningKey). Storing it as kSecAttrKeyTypeECSECPrime‐
      // Random and reading it back as a SecKey ref (the previous bug) fails at
      // sign time and silently aborts challenge-response login.
      let privateKey = try SecureEnclave.P256.Signing.PrivateKey()

      let keyTag = "com.cowallet.se.\(keyId)".data(using: .utf8)!
      let addQuery: [String: Any] = [
        kSecClass as String: kSecClassKey,
        kSecAttrApplicationTag as String: keyTag,
        kSecValueData as String: privateKey.dataRepresentation,
        kSecAttrAccessible as String: kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly,
        kSecAttrSynchronizable as String: false,
      ]

      SecItemDelete(addQuery as CFDictionary)
      let status = SecItemAdd(addQuery as CFDictionary, nil)
      guard status == errSecSuccess else {
        throw NSError(domain: "Keychain", code: Int(status))
      }

      // 33-byte compressed SEC1 public key (backend: VerifyingKey::from_sec1_bytes).
      // Compress from x963 (65B) to avoid the iOS 16-only compressedRepresentation.
      let publicKeyData = compressPublicKey(privateKey.publicKey.x963Representation)
      result([
        "publicKey": publicKeyData.base64EncodedString(),
        "keyId": keyId,
      ])
    } catch {
      result(FlutterError(code: "GENERATION_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  /// Reconstruct a Secure Enclave signing key from its persisted opaque blob.
  /// The blob was stored via kSecValueData (see generateKey); CryptoKit — NOT a
  /// SecKey ref — is the only valid way to rebuild an SE key for signing.
  private func loadSigningKey(
    keyId: String,
    context: LAContext? = nil
  ) throws -> SecureEnclave.P256.Signing.PrivateKey {
    let keyTag = "com.cowallet.se.\(keyId)".data(using: .utf8)!
    let query: [String: Any] = [
      kSecClass as String: kSecClassKey,
      kSecAttrApplicationTag as String: keyTag,
      kSecReturnData as String: true,
    ]
    var out: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &out)
    guard status == errSecSuccess, let blob = out as? Data else {
      throw NSError(domain: "Keychain", code: Int(status),
                    userInfo: [NSLocalizedDescriptionKey: "SE signing key not found"])
    }
    if let context = context {
      return try SecureEnclave.P256.Signing.PrivateKey(
        dataRepresentation: blob, authenticationContext: context)
    }
    return try SecureEnclave.P256.Signing.PrivateKey(dataRepresentation: blob)
  }

  /// Get public key for a key ID
  private func getPublicKey(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let keyId = args["keyId"] as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "keyId is required", details: nil))
      return
    }

    do {
      // Reconstruct via CryptoKit (no auth context needed to read the public
      // key) and emit 33-byte compressed SEC1, matching generateKey + backend.
      let privateKey = try loadSigningKey(keyId: keyId)
      let compressed = compressPublicKey(privateKey.publicKey.x963Representation)
      result(compressed.base64EncodedString())
    } catch {
      result(FlutterError(code: "GET_KEY_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  /// Sign a message with biometric authentication
  private func signWithBiometric(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let keyId = args["keyId"] as? String,
          let messageBase64 = args["message"] as? String,
          let reason = args["reason"] as? String,
          let messageData = Data(base64Encoded: messageBase64) else {
      result(FlutterError(code: "INVALID_ARGS", message: "keyId, message, and reason are required", details: nil))
      return
    }

    // Authenticate with biometrics, falling back to the device passcode.
    //
    // MUST be .deviceOwnerAuthentication (NOT ...WithBiometrics): the
    // "WithBiometrics" policy is biometric-ONLY — when Face ID fails, its
    // fallback button returns LAError.userFallback to the app instead of
    // presenting the system passcode, so the user is stuck. The plain
    // .deviceOwnerAuthentication policy tries biometrics first and then
    // automatically presents the system PIN/passcode on failure.
    //
    // This is safe here because the SE key itself is not access-control bound
    // (created with kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly, no
    // SecAccessControl), so this evaluatePolicy IS the single auth gate.
    let context = LAContext()
    var error: NSError?

    guard context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error) else {
      let message = error?.localizedDescription ?? "Device authentication not available"
      result(FlutterError(code: "BIOMETRIC_UNAVAILABLE", message: message, details: nil))
      return
    }

    context.evaluatePolicy(
      .deviceOwnerAuthentication,
      localizedReason: reason
    ) { [weak self] success, authError in
      guard success else {
        let message = authError?.localizedDescription ?? "Authentication failed"
        result(FlutterError(code: "AUTH_FAILED", message: message, details: nil))
        return
      }

      do {
        // Reconstruct the SE key via CryptoKit (the ONLY valid way — a stored
        // SE blob is not a SecKey). Pass the already-authenticated LAContext so
        // the SE operation rides the auth we just performed.
        let privateKey = try self?.loadSigningKey(keyId: keyId, context: context)
        guard let privateKey = privateKey else {
          throw NSError(domain: "Signing", code: -1,
                        userInfo: [NSLocalizedDescriptionKey: "Handler deallocated"])
        }

        // CryptoKit hashes with SHA-256 internally, matching the backend's
        // verify_prehash(Sha256::digest(msg)). Returns a DER-encoded ECDSA
        // signature; the backend accepts DER or 64-byte compact.
        let signature = try privateKey.signature(for: messageData)
        result(signature.derRepresentation.base64EncodedString())
      } catch {
        result(FlutterError(code: "SIGNING_FAILED", message: error.localizedDescription, details: nil))
      }
    }
  }

  /// Check if Secure Enclave is available
  private func isAvailable(result: @escaping FlutterResult) {
    let context = LAContext()
    var error: NSError?

    // SE is available on iPhone 5s and later
    let hasSecureEnclave = ProcessInfo().isOperatingSystemAtLeast(OperatingSystemVersion(majorVersion: 9, minorVersion: 0, patchVersion: 0))

    // Match the signing path's policy (.deviceOwnerAuthentication): available
    // whenever ANY device auth exists — biometric OR passcode. Using the
    // biometrics-only policy here would wrongly report unavailable on a
    // passcode-only device (or when biometrics are temporarily locked out),
    // even though signWithBiometric can still authenticate via the passcode.
    result(hasSecureEnclave && context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error))
  }

  // MARK: - Helper Functions

  /// Compress a 65-byte uncompressed public key to 33 bytes
  private func compressPublicKey(_ publicKey: Data) -> Data {
    guard publicKey.count == 65 && publicKey[0] == 0x04 else {
      return publicKey
    }

    let x = publicKey.subdata(in: 1 ..< 33)
    let y = publicKey.subdata(in: 33 ..< 65)

    let isOdd = y[y.count - 1] & 1 == 1
    let prefix = isOdd ? UInt8(0x03) : UInt8(0x02)

    var compressed = Data()
    compressed.append(prefix)
    compressed.append(x)
    return compressed
  }
}
