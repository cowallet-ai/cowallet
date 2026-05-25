import 'package:flutter/widgets.dart';
import '../bridge/frb_generated/frb_generated.dart';
import '../widgets/pin_verify_dialog.dart';
import '../platform/biometrics.dart';
import '../platform/biometrics_impl.dart';
import '../platform/cloud_backup.dart';
import '../platform/secure_storage.dart';
import '../platform/secure_storage_impl.dart';
import '../api/mpc_api.dart';
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

    print('[Services] Essential init complete');
  }

  /// Background initialization - runs after first paint.
  /// Heavier operations (Rust FFI, cached data) go here.
  static Future<void> initBackground() async {
    try {
      await RustLib.init(forceSameCodegenVersion: false)
          .timeout(const Duration(seconds: 5));
      rustReady = true;
    } catch (e) {
      print('[Services] RustLib.init() failed: $e — FFI unavailable');
    }
    backup = BackupShardService(PlatformCloudBackup());
    mpcSessionManager = MpcSessionManager(mpcWallet);
    pendingSign = PendingSignService();

    print('[Services] Background init complete');
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

    // Load cached data
    unawaited(txHistory.load());
    unawaited(contacts.load());

    // Initialize notification services
    notifications = NotificationService();
    await notifications.init();
    push = PushService();
    await push.init();

    print('[Services] Deferred init complete');
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

  /// Unified authentication: biometric if user enabled it, otherwise app PIN.
  /// All sensitive operations MUST use this — never call biometrics.authenticate directly.
  static Future<bool> authenticate({required String reason}) async {
    final biometricEnabled = await biometrics.isEnabled();
    print('[Auth] biometricEnabled=$biometricEnabled');
    if (biometricEnabled) {
      final hasEnrolled = await biometrics.hasEnrolledBiometrics();
      print('[Auth] hasEnrolled=$hasEnrolled');
      if (hasEnrolled) {
        final result = await biometrics.authenticate(reason: reason);
        print('[Auth] biometric authenticate result=$result');
        return result;
      }
      print('[Auth] biometric enabled but not enrolled, falling through to PIN');
    }
    final ctx = navigatorKey.currentContext;
    print('[Auth] navigatorKey.currentContext=${ctx != null ? "available" : "NULL"}');
    if (ctx == null) return false;
    final pinResult = await PinVerifyDialog.show(ctx, reason: reason);
    print('[Auth] PIN verify result=$pinResult');
    return pinResult;
  }
}
