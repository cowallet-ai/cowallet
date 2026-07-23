import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import '../bridge/mpc_bridge.dart';
import '../l10n/strings.dart';
import '../theme/colors.dart';
import '../platform/biometrics.dart';
import '../platform/biometrics_impl.dart';
import '../platform/cloud_backup.dart';
import '../platform/secure_storage.dart';
import '../platform/secure_storage_impl.dart';
import '../api/mpc_api.dart';
import '../api/shards_api.dart';
import '../utils/secure_storage.dart' as app_storage;
import 'backup_shard_service.dart';
import 'contacts_service.dart';
import 'settings_service.dart';
import 'wallet_service.dart';
import 'chain_service.dart';
import 'balance_service.dart';
import 'tx_service.dart';
import 'intent_executor.dart';
import 'gas_service.dart';
import 'notification_service.dart';
import 'push_service.dart';
import 'tx_history_service.dart';
import 'mpc_wallet_service.dart';
import 'mpc_session_manager.dart';
import 'pending_sign_service.dart';
import 'policy_service.dart';
import 'presign_pool_service.dart';

class Services {
  static final navigatorKey = GlobalKey<NavigatorState>();
  static late BiometricService biometrics;
  static late SecureStorageService storage;
  static late WalletService wallet;
  static late MpcWalletService mpcWallet;
  static late ChainService chain;
  static late BalanceService balance;
  static late TxService tx;
  static late IntentExecutor intent;
  static late GasService gas;
  static late TxHistoryService txHistory;
  static late BackupShardService backup;
  static late ContactsService contacts;
  static late NotificationService notifications;
  static late PushService push;
  static late SettingsService settings;
  static late PolicyService policy;
  static late PresignPoolService presignPool;
  static late MpcSessionManager mpcSessionManager;
  static late PendingSignService pendingSign;

  // API clients (stateless, no initialization needed)
  static final mpcApi = MpcApi();

  static bool rustReady = false;

  /// Helper to ignore Future errors without awaiting.
  static void unawaited(Future<void>? future) {
    future?.catchError((e) {
      // Silently ignore errors
    });
  }

  /// Essential initialization - completes in <500ms for critical services.
  /// Only initializes what's needed for immediate user interaction.
  static Future<void> initEssential() async {
    storage = FlutterSecureStorageService();
    biometrics = LocalAuthBiometricService();
    settings = SettingsService();
    await settings.init();

    // Critical services - needed for first interaction
    mpcWallet = MpcWalletService();
    wallet = mpcWallet;
    chain = JsonRpcChainService();
    balance = BalanceService();
    gas = GasService(chain);
    tx = MpcTxService(
      wallet: wallet,
      chain: chain,
    );
    policy = PolicyService();
    push = PushService();
    await push.init();

    debugPrint('[Services] Essential init complete');
  }

  /// Background initialization - runs after first paint.
  /// Heavier operations (Rust FFI, cached data) go here.
  static Future<void> initBackground() async {
    try {
      await MpcBridge.ensureInitialized()
          .timeout(const Duration(seconds: 5));
      rustReady = true;
    } catch (e) {
      debugPrint('[Services] RustLib.init() failed: $e — FFI unavailable');
    }
    backup = BackupShardService(PlatformCloudBackup());
    mpcSessionManager = MpcSessionManager(mpcWallet);
    pendingSign = PendingSignService();

    debugPrint('[Services] Background init complete');
  }

  /// Deferred initialization - runs after UI is stable.
  /// Non-critical services (notifications, push, cached data) go here.
  static Future<void> initDeferred() async {
    txHistory = TxHistoryService(storage: storage, chain: chain);
    contacts = ContactsService();
    intent = IntentExecutor(
      wallet: wallet,
      balance: balance,
      tx: tx,
      gas: gas,
      txHistory: txHistory,
      chain: chain,
    );
    presignPool = PresignPoolService();

    // Retry pending backup shard hash upload if previous attempt failed
    _retryPendingBackupHash();

    // Load cached data
    unawaited(txHistory.load());
    unawaited(contacts.load());

    // Initialize notification services
    notifications = NotificationService();
    await notifications.init();

    debugPrint('[Services] Deferred init complete');
  }

  /// Run all initialization phases in order.
  /// Essential first, then background, then deferred.
  static Future<void> initAll() async {
    await initEssential();
    await initBackground();
    await initDeferred();
  }

  /// @deprecated Use initAll() for better performance
  static Future<void> init() async {
    await initAll();
  }

  static Future<void> _retryPendingBackupHash() async {
    try {
      final pendingHash = await app_storage.SecureStorage.get('pending_backup_hash');
      if (pendingHash != null && pendingHash.isNotEmpty) {
        final result = await ShardsApi.storeBackupHash(backupShardHashHex: pendingHash);
        if (result.isSuccess) {
          await app_storage.SecureStorage.delete('pending_backup_hash');
          debugPrint('[Services] Retried pending backup hash upload — success');
        }
      }
    } catch (_) {}
  }

  /// Unified authentication for sensitive, non-shard-loading operations
  /// (freeze, view keys, delete account). Goes straight to the OS: the native
  /// prompt does biometrics AND falls back to the device passcode/PIN itself
  /// (`biometricOnly:false`). There is no longer an in-app PIN.
  ///
  /// If the device has NO secure lock at all, there's nothing to authenticate
  /// against — we guide the user to set one up in system settings and deny.
  /// All sensitive operations MUST use this — never call biometrics.authenticate directly.
  static Future<bool> authenticate({required String reason}) async {
    final supported = await biometrics.isDeviceSupported();
    debugPrint('[Auth] isDeviceSupported=$supported');
    if (!supported) {
      await _promptDeviceSecuritySetup();
      return false;
    }
    final result = await biometrics.authenticate(reason: reason);
    debugPrint('[Auth] native authenticate result=$result');
    return result;
  }

  /// Shown when the device has no biometric and no system passcode. Offers to
  /// open system settings so the user can add a lock, then come back. Public so
  /// onboarding can reuse the same guidance before persisting the device shard.
  static Future<void> promptDeviceSecuritySetup() => _promptDeviceSecuritySetup();

  static Future<void> _promptDeviceSecuritySetup() async {
    final ctx = navigatorKey.currentContext;
    if (ctx == null) return;
    final open = await showDialog<bool>(
      context: ctx,
      builder: (c) => AlertDialog(
        backgroundColor: CwColors.bgCard,
        title: Text(S.deviceSecurityRequiredTitle),
        content: Text(S.deviceSecurityRequiredBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(c).pop(false),
            child: Text(S.deviceSecurityCancel),
          ),
          TextButton(
            onPressed: () => Navigator.of(c).pop(true),
            child: Text(S.deviceSecurityOpenSettings),
          ),
        ],
      ),
    );
    if (open == true) {
      // Best-effort: opens the OS Settings app. Neither platform exposes a
      // reliable deep link straight to the passcode screen, so we land the
      // user in Settings and let the dialog copy tell them what to enable.
      final uri = Uri.parse('app-settings:');
      try {
        await launchUrl(uri, mode: LaunchMode.externalApplication);
      } catch (_) {}
    }
  }

  /// Authentication for operations that immediately load the device shard
  /// (sign, reshare). The device shard is always hardware-backed now, so the
  /// keystore's OWN native prompt during shard decryption authorizes the
  /// auth-bound key — that IS the authentication. Prompting here too would ask
  /// the user twice, so we defer to the keystore as the single gate.
  ///
  /// We still verify a device lock EXISTS up front: without one the keystore
  /// key is unusable and decryption would fail with an opaque error, so we
  /// guide the user to set up device security instead.
  ///
  /// Use this ONLY for shard-loading operations. Operations that do not load
  /// the shard (freeze, view keys) must call [authenticate].
  static Future<bool> authenticateForShardOp({required String reason}) async {
    final supported = await biometrics.isDeviceSupported();
    if (!supported) {
      await _promptDeviceSecuritySetup();
      return false;
    }
    debugPrint('[Auth] shard-op: deferring to native keystore prompt');
    return true;
  }
}
