package com.cowallet.mpc

import android.content.Context
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import androidx.annotation.RequiresApi
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.fragment.app.FragmentActivity
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.security.KeyStore
import java.util.Base64
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.spec.GCMParameterSpec

class MpcKeystoreHandler(private val context: Context) : MethodChannel.MethodCallHandler {
  companion object {
    private const val CHANNEL = "com.cowallet.mpc/keystore"
    private const val KEYSTORE_PROVIDER = "AndroidKeyStore"
    private const val CIPHER_TRANSFORMATION = "AES/GCM/NoPadding"
    private const val KEY_ALIAS = "com.cowallet.storage.master"
    private const val SHARD_KEY_ALIAS = "com.cowallet.shard.encryption"
    private const val GCM_TAG_LENGTH_BITS = 128
    private const val IV_LENGTH_BYTES = 12
    private const val SHARD_PREFS_NAME = "cowallet_shard_storage"
    private const val SHARD_PREF_KEY = "device-shard-encrypted"
    // Validity window (seconds) for the shard key after a successful auth.
    // Time-bound (not per-use/CryptoObject) to avoid the ColorOS -26 bug.
    private const val SHARD_AUTH_VALIDITY_SECONDS = 15

    fun setup(flutterEngine: FlutterEngine, context: Context) {
      val channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
      channel.setMethodCallHandler(MpcKeystoreHandler(context))
    }
  }

  private val mainHandler = Handler(Looper.getMainLooper())

  /// Encrypted, MasterKey-backed SharedPreferences for shard ciphertext.
  /// The MasterKey is itself AES256-GCM in the AndroidKeyStore, so the on-disk
  /// preference file is double-encrypted and excluded from backups via
  /// android:allowBackup="false" / data_extraction_rules.xml.
  private fun shardPrefs() =
    EncryptedSharedPreferences.create(
      context,
      SHARD_PREFS_NAME,
      MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build(),
      EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
      EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
    )

  override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
    when (call.method) {
      "storeSecret" -> {
        val key = call.argument<String>("key")
        val value = call.argument<String>("value")
        if (key != null && value != null) {
          storeSecret(key, value, result)
        } else {
          result.error("INVALID_ARGS", "key and value are required", null)
        }
      }
      "getSecret" -> {
        val key = call.argument<String>("key")
        if (key != null) {
          getSecret(key, result)
        } else {
          result.error("INVALID_ARGS", "key is required", null)
        }
      }
      "deleteSecret" -> {
        val key = call.argument<String>("key")
        if (key != null) {
          deleteSecret(key, result)
        } else {
          result.error("INVALID_ARGS", "key is required", null)
        }
      }
      "storeEncryptedShard" -> {
        // Flutter sends List<int> which arrives as List<Int> or ByteArray depending on codec
        val data: ByteArray? = when (val raw = call.argument<Any>("data")) {
          is ByteArray -> raw
          is List<*> -> ByteArray(raw.size) { i -> (raw[i] as Number).toByte() }
          else -> null
        }
        if (data != null) {
          storeEncryptedShard(data, result)
        } else {
          result.error("INVALID_ARGS", "data is required", null)
        }
      }
      "loadEncryptedShard" -> {
        loadEncryptedShard(result)
      }
      else -> result.notImplemented()
    }
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun storeSecret(key: String, value: String, result: MethodChannel.Result) {
    try {
      ensureMasterKeyExists()

      val encryptedData = encryptData(value.toByteArray(Charsets.UTF_8))

      val sharedPref = context.getSharedPreferences("cowallet_secure_storage", Context.MODE_PRIVATE)
      sharedPref.edit().putString(key, encryptedData).apply()

      result.success(null)
    } catch (e: Exception) {
      result.error("STORE_FAILED", e.message, null)
    }
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun getSecret(key: String, result: MethodChannel.Result) {
    try {
      val sharedPref = context.getSharedPreferences("cowallet_secure_storage", Context.MODE_PRIVATE)
      val encryptedData = sharedPref.getString(key, null)

      if (encryptedData == null) {
        result.success(null)
        return
      }

      val decryptedData = decryptData(encryptedData)
      val value = String(decryptedData, Charsets.UTF_8)

      result.success(value)
    } catch (e: Exception) {
      result.error("GET_FAILED", e.message, null)
    }
  }

  private fun deleteSecret(key: String, result: MethodChannel.Result) {
    try {
      val sharedPref = context.getSharedPreferences("cowallet_secure_storage", Context.MODE_PRIVATE)
      sharedPref.edit().remove(key).apply()

      result.success(null)
    } catch (e: Exception) {
      result.error("DELETE_FAILED", e.message, null)
    }
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun ensureMasterKeyExists() {
    val keyStore = KeyStore.getInstance(KEYSTORE_PROVIDER)
    keyStore.load(null)

    if (!keyStore.containsAlias(KEY_ALIAS)) {
      // Try StrongBox first, then fall back to a regular TEE-backed key. Devices
      // without a dedicated StrongBox chip throw StrongBoxUnavailableException on
      // setIsStrongBoxBacked(true), so we retry without it rather than failing.
      val strongBoxSupported = Build.VERSION.SDK_INT >= Build.VERSION_CODES.P
      try {
        generateMasterKey(useStrongBox = strongBoxSupported)
      } catch (e: android.security.keystore.StrongBoxUnavailableException) {
        if (keyStore.containsAlias(KEY_ALIAS)) keyStore.deleteEntry(KEY_ALIAS)
        generateMasterKey(useStrongBox = false)
      }
    }
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun generateMasterKey(useStrongBox: Boolean) {
    val builder = KeyGenParameterSpec.Builder(
      KEY_ALIAS,
      KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
    )
      .setKeySize(256)
      .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
      .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)

    if (useStrongBox && Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
      builder.setIsStrongBoxBacked(true)
    }

    val keyGenerator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE_PROVIDER)
    keyGenerator.init(builder.build())
    keyGenerator.generateKey()
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun encryptData(plaintext: ByteArray): String {
    val keyStore = KeyStore.getInstance(KEYSTORE_PROVIDER)
    keyStore.load(null)

    val secretKey = keyStore.getKey(KEY_ALIAS, null)
      ?: throw Exception("Master key not found")

    val cipher = Cipher.getInstance(CIPHER_TRANSFORMATION)
    cipher.init(Cipher.ENCRYPT_MODE, secretKey)

    val iv = cipher.iv
    val ciphertext = cipher.doFinal(plaintext)

    val combined = ByteArray(iv.size + ciphertext.size)
    System.arraycopy(iv, 0, combined, 0, iv.size)
    System.arraycopy(ciphertext, 0, combined, iv.size, ciphertext.size)

    return Base64.getEncoder().encodeToString(combined)
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun decryptData(encryptedData: String): ByteArray {
    val keyStore = KeyStore.getInstance(KEYSTORE_PROVIDER)
    keyStore.load(null)

    val secretKey = keyStore.getKey(KEY_ALIAS, null)
      ?: throw Exception("Master key not found")

    val combined = Base64.getDecoder().decode(encryptedData)

    val iv = ByteArray(IV_LENGTH_BYTES)
    val ciphertext = ByteArray(combined.size - IV_LENGTH_BYTES)

    System.arraycopy(combined, 0, iv, 0, IV_LENGTH_BYTES)
    System.arraycopy(combined, IV_LENGTH_BYTES, ciphertext, 0, ciphertext.size)

    val cipher = Cipher.getInstance(CIPHER_TRANSFORMATION)
    cipher.init(Cipher.DECRYPT_MODE, secretKey, GCMParameterSpec(GCM_TAG_LENGTH_BITS, iv))

    return cipher.doFinal(ciphertext)
  }

  // MARK: - Hardware-Backed Shard Encryption

  @RequiresApi(Build.VERSION_CODES.M)
  private fun storeEncryptedShard(shardData: ByteArray, result: MethodChannel.Result) {
    try {
      // Ensure hardware-backed, auth-bound encryption key exists
      ensureShardEncryptionKeyExists()

      // Authenticate the user (plain BiometricPrompt, NO CryptoObject). The key
      // is time-bound: within SHARD_AUTH_VALIDITY_SECONDS of this auth the
      // Cipher can be initialized and used. This avoids the CryptoObject-bound
      // per-use path that fails with -26 on some ROMs (ColorOS/StrongBox).
      authenticateUser(
        title = "Protect wallet backup",
        subtitle = "Authenticate to encrypt your device shard",
        onSuccess = {
          try {
            val cipher = Cipher.getInstance(CIPHER_TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, getShardKey())
            val iv = cipher.iv
            val ciphertext = cipher.doFinal(shardData)
            val combined = ByteArray(iv.size + ciphertext.size)
            System.arraycopy(iv, 0, combined, 0, iv.size)
            System.arraycopy(ciphertext, 0, combined, iv.size, ciphertext.size)
            val encoded = Base64.getEncoder().encodeToString(combined)

            shardPrefs().edit().putString(SHARD_PREF_KEY, encoded).apply()
            result.success(null)
          } catch (e: KeyPermanentlyInvalidatedException) {
            android.util.Log.e("BioNative", "storeEncryptedShard: key invalidated", e)
            result.error("KEY_INVALIDATED", e.message, null)
          } catch (e: Exception) {
            android.util.Log.e("BioNative", "storeEncryptedShard: encrypt failed", e)
            result.error("ENCRYPTION_FAILED", e.message, null)
          }
        },
        onError = { code, msg ->
          android.util.Log.e("BioNative", "storeEncryptedShard: auth error $code $msg")
          result.error(code, msg, null)
        },
      )
    } catch (e: Exception) {
      android.util.Log.e("BioNative", "storeEncryptedShard: failed", e)
      result.error("ENCRYPTION_FAILED", e.message, null)
    }
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun loadEncryptedShard(result: MethodChannel.Result) {
    try {
      val encryptedData = shardPrefs().getString(SHARD_PREF_KEY, null)
      if (encryptedData == null) {
        result.success(null) // No shard stored
        return
      }

      val combined = Base64.getDecoder().decode(encryptedData)
      val iv = ByteArray(IV_LENGTH_BYTES)
      val ciphertext = ByteArray(combined.size - IV_LENGTH_BYTES)
      System.arraycopy(combined, 0, iv, 0, IV_LENGTH_BYTES)
      System.arraycopy(combined, IV_LENGTH_BYTES, ciphertext, 0, ciphertext.size)

      // Authenticate first (no CryptoObject), then decrypt within the key's
      // validity window. See storeEncryptedShard for why we avoid CryptoObject.
      authenticateUser(
        title = "Unlock wallet backup",
        subtitle = "Authenticate to decrypt your device shard",
        onSuccess = {
          try {
            val cipher = Cipher.getInstance(CIPHER_TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, getShardKey(), GCMParameterSpec(GCM_TAG_LENGTH_BITS, iv))
            val decrypted = cipher.doFinal(ciphertext)
            result.success(decrypted)
          } catch (e: KeyPermanentlyInvalidatedException) {
            result.error("KEY_INVALIDATED", e.message, null)
          } catch (e: Exception) {
            result.error("DECRYPTION_FAILED", e.message, null)
          }
        },
        onError = { code, msg -> result.error(code, msg, null) },
      )
    } catch (e: Exception) {
      result.error("DECRYPTION_FAILED", e.message, null)
    }
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun getShardKey(): javax.crypto.SecretKey {
    val keyStore = KeyStore.getInstance(KEYSTORE_PROVIDER)
    keyStore.load(null)
    return keyStore.getKey(SHARD_KEY_ALIAS, null) as? javax.crypto.SecretKey
      ?: throw Exception("Shard encryption key not found")
  }

  /// Authorize an auth-bound Cipher via BiometricPrompt and run [onSuccess] on
  /// the main thread once the user authenticates. The Cipher is wrapped in a
  /// CryptoObject so the resulting key use is cryptographically tied to this
  /// specific authentication. Requires the host to be a FragmentActivity
  /// (MainActivity extends FlutterFragmentActivity), so no Dart-side change is
  /// needed — the native prompt is shown automatically on store/load.
  private fun authenticateUser(
    title: String,
    subtitle: String,
    onSuccess: () -> Unit,
    onError: (String, String) -> Unit,
  ) {
    val activity = context as? FragmentActivity
    if (activity == null) {
      android.util.Log.e("BioNative", "authenticateUser: context is not a FragmentActivity")
      onError("NO_ACTIVITY", "Biometric prompt requires a FragmentActivity host")
      return
    }

    mainHandler.post {
      val executor = androidx.core.content.ContextCompat.getMainExecutor(context)
      val prompt = BiometricPrompt(
        activity,
        executor,
        object : BiometricPrompt.AuthenticationCallback() {
          override fun onAuthenticationSucceeded(authResult: BiometricPrompt.AuthenticationResult) {
            // No CryptoObject: the time-bound key is now usable for the next
            // SHARD_AUTH_VALIDITY_SECONDS. Caller inits & runs the cipher.
            onSuccess()
          }

          override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
            android.util.Log.e("BioNative", "authenticateUser: onAuthenticationError $errorCode $errString")
            onError("AUTH_FAILED", errString.toString())
          }
        },
      )

      // No CryptoObject here — this is a plain user-presence check that opens
      // the key's time-bound validity window. Both biometric and device
      // credential are accepted; when DEVICE_CREDENTIAL is allowed, no negative
      // button may be set.
      val promptInfo = BiometricPrompt.PromptInfo.Builder()
        .setTitle(title)
        .setSubtitle(subtitle)
        .setAllowedAuthenticators(
          BiometricManager.Authenticators.BIOMETRIC_STRONG or
            BiometricManager.Authenticators.DEVICE_CREDENTIAL,
        )
        .build()

      try {
        prompt.authenticate(promptInfo)
      } catch (e: Exception) {
        onError("AUTH_FAILED", e.message ?: "Biometric authentication failed")
      }
    }
  }

  @RequiresApi(Build.VERSION_CODES.M)
  private fun ensureShardEncryptionKeyExists() {
    val keyStore = KeyStore.getInstance(KEYSTORE_PROVIDER)
    keyStore.load(null)

    // Self-heal: a shard key left over from an earlier run may have been created
    // with incompatible auth parameters (e.g. an older build that allowed
    // DEVICE_CREDENTIAL, which cannot authorize a CryptoObject and yields
    // KEY_USER_NOT_AUTHENTICATED). If no shard has actually been stored yet, it
    // is safe to drop such a key and regenerate it with the current parameters.
    if (keyStore.containsAlias(SHARD_KEY_ALIAS)) {
      val hasStoredShard = shardPrefs().getString(SHARD_PREF_KEY, null) != null
      if (!hasStoredShard) {
        android.util.Log.i("BioNative", "recreating stale shard key with current auth params")
        keyStore.deleteEntry(SHARD_KEY_ALIAS)
      }
    }

    if (!keyStore.containsAlias(SHARD_KEY_ALIAS)) {
      // The shard key is bound to user authentication with a short validity
      // window (see below). The ciphertext cannot be decrypted without a recent
      // live authentication, even with full filesystem access.
      //
      // NOTE: We deliberately do NOT use a per-use (timeout=0) CryptoObject-bound
      // key. On some ROMs (e.g. ColorOS) the Keystore2 implementation rejects the
      // crypto op with KEY_USER_NOT_AUTHENTICATED (-26) even after a successful
      // BiometricPrompt, because the device-credential auth token isn't bound to
      // the specific operation. A time-bound key authorized by a plain
      // BiometricPrompt (no CryptoObject) works across all ROMs.
      val builder = KeyGenParameterSpec.Builder(
        SHARD_KEY_ALIAS,
        KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
      )
        .setKeySize(256)
        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        .setRandomizedEncryptionRequired(true)
        .setUserAuthenticationRequired(true)

      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        builder.setIsStrongBoxBacked(true) // Use StrongBox if available
      }

      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        // 15s validity window after any strong auth (biometric OR device
        // credential). No CryptoObject needed — avoids the ColorOS -26 bug.
        builder.setUserAuthenticationParameters(
          SHARD_AUTH_VALIDITY_SECONDS,
          KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
        )
      } else {
        @Suppress("DEPRECATION")
        builder.setUserAuthenticationValidityDurationSeconds(SHARD_AUTH_VALIDITY_SECONDS)
      }

      val keyGenerator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE_PROVIDER)
      try {
        keyGenerator.init(builder.build())
        keyGenerator.generateKey()
        android.util.Log.i("BioNative", "shard key generated (StrongBox path)")
      } catch (e: Exception) {
        // StrongBox may be unavailable on this device; retry without it.
        android.util.Log.w("BioNative", "StrongBox key gen failed, retrying without: ${e.message}")
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
          builder.setIsStrongBoxBacked(false)
          keyGenerator.init(builder.build())
          keyGenerator.generateKey()
          android.util.Log.i("BioNative", "shard key generated (TEE fallback path)")
        } else {
          throw e
        }
      }
    }
  }

}
