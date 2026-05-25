import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_native_splash/flutter_native_splash.dart';
import 'package:cowallet/theme/theme.dart';
import 'package:cowallet/router/app_router.dart';
import 'package:cowallet/state/app_state.dart';
import 'package:cowallet/services/locator.dart';
import 'package:cowallet/services/push_service.dart';
import 'package:cowallet/api/auth_api.dart';
import 'package:cowallet/utils/secure_storage.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  SystemChrome.setPreferredOrientations([
    DeviceOrientation.portraitUp,
  ]);

  // 🔥 INSTANT FIRST PAINT - Native splash shows immediately
  runApp(const CowalletApp());

  // Start initialization in background
  print('[main] Starting background initialization...');
  Services.initAll().then((_) {
    print('[main] All services initialized');
    // Remove native splash when app is ready
    FlutterNativeSplash.remove();
  });
}

class CowalletApp extends StatefulWidget {
  const CowalletApp({super.key});

  static AppState of(BuildContext context) =>
      context.findAncestorStateOfType<_CowalletAppState>()!.appState;

  @override
  State<CowalletApp> createState() => _CowalletAppState();
}

class _CowalletAppState extends State<CowalletApp> {
  final appState = AppState();
  final String _initialRoute = AppRouter.onboarding;  // Default to onboarding

  // Use shared navigator key from Services
  GlobalKey<NavigatorState> get _navigatorKey => Services.navigatorKey;

  @override
  void initState() {
    super.initState();
    _initEssentialAndNavigate();
  }

  Future<void> _initEssentialAndNavigate() async {
    // Wait for essential services to be ready
    await Services.initEssential();
    _setupPushNotificationHandlers();

    // Check wallet status and navigate accordingly
    _checkWalletState();
  }

  void _setupPushNotificationHandlers() {
    Services.push.onNotificationTap = _handlePushNotificationTap;
  }

  Future<void> _checkWalletState() async {
    final hasWallet = await Services.wallet.hasWallet();

    if (!mounted) return;

    if (hasWallet) {
      // Wallet exists, navigate to home
      _navigatorKey.currentState?.pushNamedAndRemoveUntil(
        AppRouter.home,
        (route) => false,
      );

      // Load wallet address in background
      final addr = await Services.wallet.getAddress();
      appState.setWalletAddress(addr);
      appState.completeOnboarding();

      // Background tasks
      _refreshSessionInBackground();
      Services.push.reregisterToken();
      _refreshBalanceInBackground(addr);
    }
    // If no wallet, we stay on onboarding (initialRoute)
  }

  Future<void> _refreshSessionInBackground() async {
    try {
      final tokenValid = await AuthApi.isLoggedIn();
      if (tokenValid) return;

      final refreshed = await AuthApi.refreshToken();
      if (!refreshed) {
        await AuthApi.login(deviceId: (await SecureStorage.getDeviceId()) ?? '');
      }
    } catch (_) {}
  }

  Future<void> _refreshBalanceInBackground(String address) async {
    try {
      await Services.balance.refresh(address);
    } catch (_) {}
  }

  void _handlePushNotificationTap(Map<String, dynamic> data) {
    final type = data['type'] as String?;
    final context = _navigatorKey.currentContext;
    if (context == null) return;

    switch (type) {
      case PushType.txConfirmed:
      case PushType.txFailed:
        final txHash = data['tx_hash'] as String?;
        if (txHash != null) {
          _navigatorKey.currentState?.pushNamedAndRemoveUntil(
            AppRouter.home,
            (route) => false,
          );
        }
        break;
      case PushType.securityAlert:
        _navigatorKey.currentState?.pushNamedAndRemoveUntil(
          AppRouter.home,
          (route) => false,
        );
        break;
      case PushType.mpcSignRequest:
        _navigatorKey.currentState?.pushNamedAndRemoveUntil(
          AppRouter.home,
          (route) => false,
        );
        break;
    }
  }

  @override
  void dispose() {
    Services.push.dispose();
    Services.presignPool.dispose();
    appState.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: appState,
      builder: (context, _) => MaterialApp(
        navigatorKey: _navigatorKey,
        title: 'CoWallet',
        debugShowCheckedModeBanner: false,
        theme: cwTheme(),
        initialRoute: _initialRoute,
        onGenerateRoute: AppRouter.onGenerateRoute,
      ),
    );
  }
}