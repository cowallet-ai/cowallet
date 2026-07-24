import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_native_splash/flutter_native_splash.dart';
import 'package:cowallet/theme/theme.dart';
import 'package:cowallet/theme/theme_controller.dart';
import 'package:cowallet/router/app_router.dart';
import 'package:cowallet/state/app_state.dart';
import 'package:cowallet/services/locator.dart';
import 'package:cowallet/services/push_service.dart';
import 'package:cowallet/api/auth_api.dart';
import 'package:cowallet/network/dio_client.dart';
import 'package:cowallet/services/version_check.dart';
import 'package:cowallet/l10n/app_localizations.dart';
import 'package:cowallet/l10n/s.dart';
import 'package:cowallet/views/settings/mandatory_backup_export_view.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  SystemChrome.setPreferredOrientations([DeviceOrientation.portraitUp]);

  // Wire the 401 interceptor to the single-flight session recoverer so a token
  // expiry mid-session self-heals (refresh → challenge-response re-login)
  // instead of dumping the user. Shares one in-flight recovery with startup.
  DioClient.sessionRecoverer = AuthApi.recoverSession;

  // Server-side upgrade gate: any protected request from a stale build returns
  // 426. Route to the blocking upgrade screen (idempotent) so a client that
  // bypassed the startup check still can't proceed.
  DioClient.onUpgradeRequired = (body) {
    _navigateToForceUpgrade(
      iosUrl: (body['ios_store_url'] as String?) ?? '',
      androidUrl: (body['android_store_url'] as String?) ?? '',
    );
  };

  // 🔥 INSTANT FIRST PAINT - Native splash shows immediately
  runApp(const CowalletApp());

  // Start initialization in background
  debugPrint('[main] Starting background initialization...');
  Services.initAll().then((_) {
    debugPrint('[main] All services initialized');
    // Remove native splash when app is ready
    FlutterNativeSplash.remove();
  });
}

/// Whether the blocking upgrade screen is already showing — prevents stacking
/// duplicate screens when multiple in-flight requests each return 426.
bool _forceUpgradeShown = false;

/// Replace the whole stack with the non-dismissible upgrade screen. Safe to call
/// from anywhere (startup check or the 426 interceptor); no-ops if already shown.
void _navigateToForceUpgrade({
  required String iosUrl,
  required String androidUrl,
}) {
  if (_forceUpgradeShown) return;
  _forceUpgradeShown = true;
  Services.navigatorKey.currentState?.pushNamedAndRemoveUntil(
    AppRouter.forceUpgrade,
    (route) => false,
    arguments: {'ios': iosUrl, 'android': androidUrl},
  );
}

class CowalletApp extends StatefulWidget {
  const CowalletApp({super.key});

  static AppState of(BuildContext context) =>
      context.findAncestorStateOfType<_CowalletAppState>()!.appState;

  static void setLocale(BuildContext context, Locale locale) {
    context.findAncestorStateOfType<_CowalletAppState>()?._setLocale(locale);
  }

  @override
  State<CowalletApp> createState() => _CowalletAppState();
}

class _CowalletAppState extends State<CowalletApp>
    with WidgetsBindingObserver {
  final appState = AppState();
  Locale? _locale;
  final String _initialRoute = AppRouter.onboarding; // Default to onboarding

  // Use shared navigator key from Services
  GlobalKey<NavigatorState> get _navigatorKey => Services.navigatorKey;

  @override
  void initState() {
    super.initState();
    // Path ② — OS light/dark switch (the sneaky one under themeMode.system).
    // Path ① (settings listener) and the first sync are wired only after
    // Services.settings is constructed in _initEssentialAndNavigate — it's a
    // `late` field that isn't assigned yet at initState time.
    WidgetsBinding.instance.addObserver(this);
    _initEssentialAndNavigate();
  }

  /// Sole writer of [ThemeController.brightness]. All three trigger paths funnel
  /// here so the value MaterialApp renders and the value CwColors reads stay one
  /// and the same — no drift under themeMode.system.
  void _syncBrightness() {
    final platform =
        WidgetsBinding.instance.platformDispatcher.platformBrightness;
    final resolved =
        ThemeController.resolve(Services.settings.themeMode, platform);
    if (ThemeController.brightness.value != resolved) {
      setState(() => ThemeController.brightness.value = resolved);
    }
  }

  // Path ② delivery: OS brightness changed while we're running.
  @override
  void didChangePlatformBrightness() => _syncBrightness();

  /// Initialize locale: check saved preference or auto-detect from system
  Future<void> _initLocale() async {
    final savedLang = Services.settings.language;
    if (savedLang == 'zh' || savedLang == 'en') {
      if (!mounted) return;
      setState(() => _locale = Locale(savedLang!));
    } else {
      // Auto-detect from system locale
      final systemLocale = WidgetsBinding.instance.platformDispatcher.locale;
      final locale = _detectLocale(systemLocale);
      if (!mounted) return;
      setState(() => _locale = locale);
      await Services.settings.setLanguage(locale.languageCode);
    }
  }

  /// Detect locale from system language code
  Locale _detectLocale(Locale systemLocale) {
    final lang = systemLocale.languageCode.toLowerCase();
    return (lang == 'zh' || lang.startsWith('zh'))
        ? const Locale('zh')
        : const Locale('en');
  }

  /// Change language and persist preference
  void _setLocale(Locale locale) {
    if (_locale?.languageCode == locale.languageCode) return;
    setState(() => _locale = locale);
    Services.settings.setLanguage(locale.languageCode);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    Services.settings.removeListener(_syncBrightness);
    Services.push.dispose();
    Services.presignPool.dispose();
    appState.dispose();
    super.dispose();
  }

  Future<void> _initEssentialAndNavigate() async {
    // Wait for essential services to be ready
    await Services.initEssential();
    // Now that Services.settings exists and has loaded persisted themeMode,
    // wire path ① and apply the saved light/dark/system choice.
    Services.settings.addListener(_syncBrightness);
    _syncBrightness();
    await _initLocale();
    _setupPushNotificationHandlers();

    // Forced-upgrade gate (client half). If this build is below the server's
    // min_build, show the blocking screen and stop — do NOT route into the
    // wallet, since signing would fail on the incompatible v1.0.1 MPC protocol.
    // Fail-open: a network/parse error returns ok, so the app still starts.
    final version = await VersionCheck.check();
    if (version.mustUpgrade) {
      _navigateToForceUpgrade(
        iosUrl: version.iosStoreUrl,
        androidUrl: version.androidStoreUrl,
      );
      return;
    }

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

      // If a key rotation left an un-exported backup (offline-file users), the
      // refreshed shard was staged durably. Force the user back to the blocking
      // backup screen — skipping it means backup+server recovery would fail.
      await _enforcePendingBackupReExport();

      // Background tasks
      _refreshSessionInBackground();
      Services.push.reregisterToken();
      _refreshBalanceInBackground(addr);
    }
    // If no wallet, we stay on onboarding (initialRoute)
  }

  /// Force the mandatory backup re-export screen if a prior key rotation left
  /// the refreshed backup shard un-exported (durably flagged). Blocks until the
  /// user completes the export. Idempotent — safe to call on every launch.
  Future<void> _enforcePendingBackupReExport() async {
    try {
      final pending = await Services.mpcWallet.isBackupReExportPending();
      if (!pending) return;
      final ctx = _navigatorKey.currentContext;
      if (ctx == null || !ctx.mounted) return;
      await Navigator.of(ctx).push(
        MaterialPageRoute(
          fullscreenDialog: true,
          builder: (_) => const MandatoryBackupExportView(),
        ),
      );
    } catch (_) {
      // Never let this block app startup; the flag persists for the next launch.
    }
  }

  Future<void> _refreshSessionInBackground() async {
    try {
      if (await AuthApi.isLoggedIn()) return;
      // Single-flight recovery shared with the 401 interceptor: refresh first,
      // then challenge-response re-login. Prevents startup and an early 401
      // from racing over the one-time-use refresh token.
      await AuthApi.recoverSession();
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
  Widget build(BuildContext context) {
    return MaterialApp(
      navigatorKey: _navigatorKey,
      title: 'CoWallet',
      debugShowCheckedModeBanner: false,
      // Option Y: we resolve brightness ourselves and hand MaterialApp the one
      // matching theme. No darkTheme/themeMode — Flutter never independently
      // picks a brightness that could disagree with CwColors' global signal.
      theme: ThemeController.isDark ? cwDarkTheme() : cwTheme(),
      initialRoute: _initialRoute,
      onGenerateRoute: AppRouter.onGenerateRoute,
      locale: _locale,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: const [Locale('zh'), Locale('en')],
      builder: (context, child) {
        // Initialize S with context for backward compatibility
        S.of(context);
        // App-wide tap-to-dismiss: tapping any non-interactive area (outside a
        // text field / button) drops focus and closes the soft keyboard. Screens
        // with text inputs (backup, contacts, recovery, search, onboarding) had
        // no other way to dismiss the keyboard once it was up.
        return GestureDetector(
          onTap: () => FocusManager.instance.primaryFocus?.unfocus(),
          // translucent so taps still hit widgets underneath; only "empty"
          // taps that nothing else consumes reach this handler.
          behavior: HitTestBehavior.translucent,
          child: child!,
        );
      },
    );
  }
}
