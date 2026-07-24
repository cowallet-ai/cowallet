import 'package:flutter/widgets.dart';
import 'package:flutter/cupertino.dart';
import '../services/locator.dart';
import '../services/mpc_wallet_service.dart';
import '../services/mpc_session_manager.dart';
import '../utils/secure_storage.dart';
import 'routes.dart';

/// Owns all cross-stage onboarding state, persistence, side-effects, and
/// child-Navigator transitions. BuildContext-free so it can be driven from
/// tests with a fake subclass. Stages read it via [OnboardingScope].
class OnboardingController {
  OnboardingController({GlobalKey<NavigatorState>? navigatorKey})
      : navigatorKey = navigatorKey ?? GlobalKey<NavigatorState>();

  final GlobalKey<NavigatorState> navigatorKey;

  // ---- Cross-stage shared state ----
  String email = '';
  bool forceRegister = false;
  String? backupShardHash;
  bool backupSkipped = false;
  bool backupDone = false;

  NavigatorState? get nav => navigatorKey.currentState;

  MpcSessionManager? _sessionManager;
  MpcSessionManager get sessionManager =>
      _sessionManager ??= MpcSessionManager(Services.mpcWallet);

  MpcWalletService get walletService => Services.wallet as MpcWalletService;

  // ---- Persistence ----
  Future<void> persistStep(OnboardingStep step) =>
      SecureStorage.save(SecureStorage.keyOnboardingStep, step.name);

  Future<void> clearStep() =>
      SecureStorage.delete(SecureStorage.keyOnboardingStep);

  // ---- Transitions ----

  /// Push a stage as the next route and persist it. Used for freely-returnable
  /// forward moves (hero→email→otp, bio→name→ready→persona).
  void goTo(OnboardingStep step) {
    persistStep(step);
    nav?.pushNamed(step.name);
  }

  /// Pop to the previous stage in the child Navigator.
  void goBack() => nav?.maybePop();

  /// DKG completed: clear the pre-DKG stack and land on backup as the new root.
  void onDkgSuccess() {
    persistStep(OnboardingStep.backup);
    nav?.pushNamedAndRemoveUntil(OnboardingRoutes.backup, (r) => false);
  }

  /// backup→bio: replace so bio becomes the returnable-group floor.
  void goToBioFromBackup() {
    persistStep(OnboardingStep.bio);
    nav?.pushReplacementNamed(OnboardingRoutes.bio);
  }

  /// Leave onboarding for the app home via the ROOT navigator.
  Future<void> finish(BuildContext rootContext, String walletAddress) async {
    await clearStep();
    await SecureStorage.save('onboarding_completed_at', DateTime.now().toIso8601String());
    await SecureStorage.save(
        'backup_status', backupSkipped ? 'skipped' : (backupDone ? 'saved' : 'pending'));
    await SecureStorage.save('mpc_address', walletAddress);
    if (walletAddress.isNotEmpty) {
      Services.balance.refresh(walletAddress);
    }
    // ignore: use_build_context_synchronously
    Navigator.of(rootContext, rootNavigator: true).pushReplacementNamed('/');
  }
}
