import '../platform/se_manager.dart';
import '../platform/sb_manager.dart';
import '../platform/cloud_backup.dart';
import '../api/mpc_api.dart';
import '../utils/secure_storage.dart';
import 'backup_shard_service.dart';

enum KeyStatus { ok, warning, error, unknown }

class KeyHealth {
  final KeyStatus status;
  final DateTime? lastUsed;
  final DateTime? lastChecked;
  final String? error;

  KeyHealth({
    required this.status,
    this.lastUsed,
    this.lastChecked,
    this.error,
  });
}

class KeyHealthService {
  static const verifyExpiryDays = 30;
  final _backupService = BackupShardService(PlatformCloudBackup());
  static const _lastUsedPhonePrefix = 'key_phone_last_used_';
  static const _lastUsedServerPrefix = 'key_server_last_used_';
  static const _lastCheckedBackupPrefix = 'key_backup_last_checked_';

  Future<String> _getWalletSuffix() async {
    final addr = await SecureStorage.get('mpc_address');
    if (addr != null && addr.length >= 10) return addr.toLowerCase().substring(0, 10);
    return 'unknown';
  }

  Future<String> _getBackupCheckedKey() async => '$_lastCheckedBackupPrefix${await _getWalletSuffix()}';

  /// Check key 1: phone (Secure Enclave / StrongBox)
  Future<KeyHealth> checkPhoneKey() async {
    try {
      final se = SecureEnclaveManager();
      final sb = StrongBoxManager();

      bool available = false;
      if (await se.isAvailable()) {
        available = true;
      } else if (await sb.isAvailable()) {
        available = true;
      }

      final suffix = await _getWalletSuffix();
      final lastUsedStr = await SecureStorage.get('$_lastUsedPhonePrefix$suffix');
      final lastUsed = lastUsedStr != null ? DateTime.tryParse(lastUsedStr) : null;

      return KeyHealth(
        status: available ? KeyStatus.ok : KeyStatus.error,
        lastUsed: lastUsed,
        lastChecked: DateTime.now(),
        error: available ? null : 'Hardware security not available',
      );
    } catch (e) {
      return KeyHealth(status: KeyStatus.error, error: e.toString());
    }
  }

  /// Check key 2: server heartbeat
  Future<KeyHealth> checkServerKey() async {
    try {
      final result = await MpcApi.getServerShardStatus();
      final suffix = await _getWalletSuffix();
      final lastUsedStr = await SecureStorage.get('$_lastUsedServerPrefix$suffix');
      final lastUsed = lastUsedStr != null ? DateTime.tryParse(lastUsedStr) : null;

      if (result.isSuccess) {
        return KeyHealth(
          status: KeyStatus.ok,
          lastUsed: lastUsed,
          lastChecked: DateTime.now(),
        );
      } else {
        return KeyHealth(
          status: KeyStatus.warning,
          lastUsed: lastUsed,
          lastChecked: DateTime.now(),
          error: result.errorMessage,
        );
      }
    } catch (e) {
      return KeyHealth(
        status: KeyStatus.error,
        lastChecked: DateTime.now(),
        error: e.toString(),
      );
    }
  }

  /// Get the backup method used during setup.
  Future<BackupMethod?> getBackupMethod() async {
    return await _backupService.getBackupMethod();
  }

  /// Check key 3: backup (cloud or file)
  Future<KeyHealth> checkBackupKey() async {
    try {
      final backupCheckedKey = await _getBackupCheckedKey();
      final lastCheckedStr = await SecureStorage.get(backupCheckedKey);
      final lastChecked = lastCheckedStr != null ? DateTime.tryParse(lastCheckedStr) : null;

      final method = await _backupService.getBackupMethod();

      // Local file backup cannot be auto-verified
      if (method == BackupMethod.file) {
        if (lastChecked != null) {
          return KeyHealth(
            status: KeyStatus.ok,
            lastChecked: lastChecked,
          );
        }
        return KeyHealth(
          status: KeyStatus.warning,
          lastChecked: null,
          error: 'file_not_verified',
        );
      }

      // Cloud backup check
      if (lastChecked != null) {
        return KeyHealth(
          status: KeyStatus.ok,
          lastChecked: lastChecked,
        );
      }

      final hasBackup = await _backupService.hasCloudBackup();
      return KeyHealth(
        status: hasBackup ? KeyStatus.warning : KeyStatus.unknown,
        lastChecked: null,
        error: hasBackup ? 'cloud_not_verified' : 'cloud_not_found',
      );
    } catch (e) {
      return KeyHealth(
        status: KeyStatus.error,
        error: e.toString(),
      );
    }
  }


  /// Verify the cloud backup is recoverable. Requires the user's backup
  /// [password]: a successful Argon2id + AES-256-GCM decryption (which also
  /// validates the shard is a well-formed secp256k1 scalar) proves the backup
  /// is intact. The plaintext shard never crosses the FFI boundary into Dart.
  Future<bool> testBackupKey(String password) async {
    try {
      final encrypted = await _backupService.retrieveFromCloud();
      if (encrypted == null || encrypted.isEmpty) {
        print('[KeyHealth] cloud backup not available');
        return false;
      }

      final valid = await _backupService.importEncrypted(encrypted, password);
      print('[KeyHealth] cloud backup decrypt+validate result: $valid');
      if (!valid) return false;

      await SecureStorage.save(await _getBackupCheckedKey(), DateTime.now().toIso8601String());
      return true;
    } catch (e) {
      print('[KeyHealth] testBackupKey (cloud) error: $e');
      return false;
    }
  }

  /// Verify a backup file is recoverable using the user's backup [password].
  Future<bool> testBackupKeyWithFile(String fileContent, String password) async {
    try {
      final encrypted = _backupService.parseBackupFile(fileContent);
      if (encrypted == null || encrypted.isEmpty) {
        print('[KeyHealth] parseBackupFile failed: empty content');
        return false;
      }

      final valid = await _backupService.importEncrypted(encrypted, password);
      print('[KeyHealth] file backup decrypt+validate result: $valid');
      if (!valid) return false;

      await SecureStorage.save(await _getBackupCheckedKey(), DateTime.now().toIso8601String());
      return true;
    } catch (e) {
      print('[KeyHealth] testBackupKeyWithFile error: $e');
      return false;
    }
  }

  Future<void> recordPhoneKeyUsage() async {
    final suffix = await _getWalletSuffix();
    await SecureStorage.save('$_lastUsedPhonePrefix$suffix', DateTime.now().toIso8601String());
  }

  Future<void> recordServerKeyUsage() async {
    final suffix = await _getWalletSuffix();
    await SecureStorage.save('$_lastUsedServerPrefix$suffix', DateTime.now().toIso8601String());
  }
}
