// iOS Secure Storage Channel Handler
// Handles platform channel calls for encrypted storage

import Flutter
import CryptoKit
import Security

public class MpcSecureStorageHandler: NSObject, FlutterPlugin {
  public static func dummy(methodCall: FlutterMethodCall, result: @escaping FlutterResult) {
    // This method is added to work around a fatal exception when a plugin is
    // registered using generics.
  }

  public static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "com.cowallet.mpc/storage",
      binaryMessenger: registrar.messenger()
    )
    let instance = MpcSecureStorageHandler()
    registrar.addMethodCallDelegate(instance, channel: channel)
  }

  public func dummyMethodToEnforceBundling() {
    // This method is added to work around a fatal exception when the plugin is
    // registered using generics.
  }

  public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "storeSecret":
      storeSecret(call, result: result)
    case "getSecret":
      getSecret(call, result: result)
    case "deleteSecret":
      deleteSecret(call, result: result)
    case "storeEncryptedShard":
      storeEncryptedShard(call, result: result)
    case "loadEncryptedShard":
      loadEncryptedShard(call, result: result)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  // MARK: - Secure Storage

  /// Store encrypted data in Keychain
  private func storeSecret(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let key = args["key"] as? String,
          let value = args["value"] as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "key and value are required", details: nil))
      return
    }

    do {
      let data = value.data(using: .utf8) ?? Data()

      // Prepare keychain query
      let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrAccount as String: key,
        kSecAttrService as String: "com.cowallet.secure_storage",
        kSecValueData as String: data,
        kSecAttrAccessible as String: kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly,
        kSecAttrSynchronizable as String: false,
      ]

      // Delete existing value if present
      SecItemDelete(query as CFDictionary)

      // Add new value
      let status = SecItemAdd(query as CFDictionary, nil)

      guard status == errSecSuccess else {
        throw NSError(domain: "Keychain", code: Int(status))
      }

      result(nil) // Success, no return value needed
    } catch {
      result(FlutterError(code: "STORE_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  /// Retrieve encrypted data from Keychain
  private func getSecret(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let key = args["key"] as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "key is required", details: nil))
      return
    }

    do {
      let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrAccount as String: key,
        kSecAttrService as String: "com.cowallet.secure_storage",
        kSecReturnData as String: true,
      ]

      var retrievedData: CFTypeRef?
      let status = SecItemCopyMatching(query as CFDictionary, &retrievedData)

      guard status == errSecSuccess else {
        // Key not found is not an error, return nil
        if status == errSecItemNotFound {
          result(nil)
        } else {
          throw NSError(domain: "Keychain", code: Int(status))
        }
        return
      }

      guard let data = retrievedData as? Data,
            let value = String(data: data, encoding: .utf8) else {
        result(FlutterError(code: "DECODE_FAILED", message: "Failed to decode value", details: nil))
        return
      }

      result(value)
    } catch {
      result(FlutterError(code: "GET_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  /// Delete encrypted data from Keychain
  private func deleteSecret(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let key = args["key"] as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "key is required", details: nil))
      return
    }

    do {
      let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrAccount as String: key,
        kSecAttrService as String: "com.cowallet.secure_storage",
      ]

      let status = SecItemDelete(query as CFDictionary)

      guard status == errSecSuccess || status == errSecItemNotFound else {
        throw NSError(domain: "Keychain", code: Int(status))
      }

      result(nil) // Success
    } catch {
      result(FlutterError(code: "DELETE_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  // MARK: - Hardware-Backed Shard Encryption

  /// Store device shard encrypted with Secure Enclave key
  private func storeEncryptedShard(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any] else {
      result(FlutterError(code: "INVALID_ARGS", message: "data is required", details: nil))
      return
    }

    let shardData: Data
    if let typedData = args["data"] as? FlutterStandardTypedData {
      shardData = typedData.data
    } else if let byteArray = args["data"] as? [UInt8] {
      shardData = Data(byteArray)
    } else if let intArray = args["data"] as? [Int] {
      shardData = Data(intArray.map { UInt8($0 & 0xFF) })
    } else {
      result(FlutterError(code: "INVALID_ARGS", message: "data is required", details: nil))
      return
    }

    do {

      // Get or create the Secure Enclave key-agreement private key
      let sePrivateKey = try getOrCreateSecureEnclaveKey()

      // Derive a fresh per-operation symmetric key via ECDH against an
      // ephemeral key, and prepend the ephemeral public key to the blob so
      // decryption can reproduce the same shared secret.
      let ephemeral = P256.KeyAgreement.PrivateKey()
      let symmetricKey = try deriveSymmetricKey(
        sePrivateKey: sePrivateKey,
        peerPublicKey: ephemeral.publicKey
      )

      let sealedBox = try ChaChaPoly.seal(shardData, using: symmetricKey)

      // Store ephemeralPublicKey || (nonce + ciphertext + tag) in Keychain.
      let ephemeralPubBytes = ephemeral.publicKey.rawRepresentation
      var encryptedData = Data()
      encryptedData.append(UInt8(ephemeralPubBytes.count))
      encryptedData.append(ephemeralPubBytes)
      encryptedData.append(sealedBox.combined)

      let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrAccount as String: "device-shard-encrypted",
        kSecAttrService as String: "com.cowallet.secure_storage",
        kSecValueData as String: encryptedData,
        kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
        kSecAttrSynchronizable as String: false,
      ]

      // Delete existing value if present
      SecItemDelete(query as CFDictionary)

      // Add new encrypted value
      let status = SecItemAdd(query as CFDictionary, nil)

      guard status == errSecSuccess else {
        throw NSError(domain: "Keychain", code: Int(status))
      }

      result(nil) // Success
    } catch {
      result(FlutterError(code: "ENCRYPTION_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  /// Load and decrypt device shard using Secure Enclave key
  private func loadEncryptedShard(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    do {
      let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrAccount as String: "device-shard-encrypted",
        kSecAttrService as String: "com.cowallet.secure_storage",
        kSecReturnData as String: true,
      ]

      var retrievedData: CFTypeRef?
      let status = SecItemCopyMatching(query as CFDictionary, &retrievedData)

      guard status == errSecSuccess else {
        if status == errSecItemNotFound {
          result(nil) // No shard stored
        } else {
          throw NSError(domain: "Keychain", code: Int(status))
        }
        return
      }

      guard let encryptedData = retrievedData as? Data, encryptedData.count > 1 else {
        result(FlutterError(code: "DECODE_FAILED", message: "Failed to decode encrypted data", details: nil))
        return
      }

      // Parse: ephemeralPubLen (1 byte) || ephemeralPublicKey || sealedBox
      let ephemeralPubLen = Int(encryptedData[encryptedData.startIndex])
      guard encryptedData.count > 1 + ephemeralPubLen else {
        result(FlutterError(code: "DECODE_FAILED", message: "Malformed encrypted shard", details: nil))
        return
      }
      let pubStart = encryptedData.index(encryptedData.startIndex, offsetBy: 1)
      let sealedStart = encryptedData.index(pubStart, offsetBy: ephemeralPubLen)
      let ephemeralPubBytes = encryptedData[pubStart..<sealedStart]
      let sealedData = encryptedData[sealedStart...]

      // Reconstruct the ephemeral public key and re-derive the symmetric key
      // via ECDH against the Secure Enclave private key.
      let sePrivateKey = try getOrCreateSecureEnclaveKey()
      let ephemeralPublicKey = try P256.KeyAgreement.PublicKey(rawRepresentation: ephemeralPubBytes)
      let symmetricKey = try deriveSymmetricKey(
        sePrivateKey: sePrivateKey,
        peerPublicKey: ephemeralPublicKey
      )

      let sealedBox = try ChaChaPoly.SealedBox(combined: sealedData)
      let decryptedData = try ChaChaPoly.open(sealedBox, using: symmetricKey)

      // Return as byte array
      let byteArray = Array(decryptedData)
      result(byteArray)
    } catch {
      result(FlutterError(code: "DECRYPTION_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  // MARK: - Encryption Key Management (Secure Enclave + ECDH)

  /// Get or create a non-exportable Secure Enclave P-256 key-agreement private
  /// key. Only the opaque `dataRepresentation` (an encrypted key blob that is
  /// useless off-device) is persisted in the Keychain — the raw private key
  /// never leaves the Secure Enclave, so it cannot be exfiltrated. Use is gated
  /// by an access control requiring the device be unlocked and the user be
  /// present (passcode / biometry).
  private func getOrCreateSecureEnclaveKey() throws -> SecureEnclave.P256.KeyAgreement.PrivateKey {
    let keyTag = "com.cowallet.shard.se.key".data(using: .utf8)!

    // Try to retrieve the existing SE key blob
    let query: [String: Any] = [
      kSecClass as String: kSecClassKey,
      kSecAttrApplicationTag as String: keyTag,
      kSecReturnData as String: true,
    ]

    var keyData: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &keyData)

    if status == errSecSuccess, let existingBlob = keyData as? Data {
      return try SecureEnclave.P256.KeyAgreement.PrivateKey(dataRepresentation: existingBlob)
    }

    // Build an access control requiring private-key usage and user presence.
    var acError: Unmanaged<CFError>?
    guard let accessControl = SecAccessControlCreateWithFlags(
      kCFAllocatorDefault,
      kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly,
      [.privateKeyUsage, .userPresence],
      &acError
    ) else {
      throw acError?.takeRetainedValue() as Error?
        ?? NSError(domain: "SecureEnclave", code: -1,
                   userInfo: [NSLocalizedDescriptionKey: "Failed to create access control"])
    }

    // Generate the private key inside the Secure Enclave.
    let sePrivateKey = try SecureEnclave.P256.KeyAgreement.PrivateKey(accessControl: accessControl)

    // Persist only the non-exportable SE key blob (device-only, not synced).
    let addQuery: [String: Any] = [
      kSecClass as String: kSecClassKey,
      kSecAttrApplicationTag as String: keyTag,
      kSecValueData as String: sePrivateKey.dataRepresentation,
      kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
      kSecAttrSynchronizable as String: false,
    ]

    SecItemDelete(addQuery as CFDictionary)
    let addStatus = SecItemAdd(addQuery as CFDictionary, nil)

    guard addStatus == errSecSuccess else {
      throw NSError(domain: "Keychain", code: Int(addStatus),
                    userInfo: [NSLocalizedDescriptionKey: "Failed to store Secure Enclave key blob"])
    }

    return sePrivateKey
  }

  /// Derive a per-operation 256-bit ChaCha20-Poly1305 symmetric key by
  /// performing ECDH between the Secure Enclave private key and an ephemeral
  /// peer public key, then running HKDF over the shared secret. The symmetric
  /// key is never persisted or returned to the channel — it lives only for the
  /// duration of a single seal/open.
  private func deriveSymmetricKey(
    sePrivateKey: SecureEnclave.P256.KeyAgreement.PrivateKey,
    peerPublicKey: P256.KeyAgreement.PublicKey
  ) throws -> SymmetricKey {
    let sharedSecret = try sePrivateKey.sharedSecretFromKeyAgreement(with: peerPublicKey)
    return sharedSecret.hkdfDerivedSymmetricKey(
      using: SHA256.self,
      salt: Data(),
      sharedInfo: "com.cowallet.shard.encryption".data(using: .utf8)!,
      outputByteCount: 32
    )
  }
}
