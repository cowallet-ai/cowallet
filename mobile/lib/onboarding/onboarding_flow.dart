import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import '../theme/colors.dart';
import '../utils/secure_storage.dart';
import 'controller.dart';
import 'routes.dart';
import 'scope.dart';
import 'stages/hero_stage.dart';
import 'stages/email_stage.dart';
import 'stages/otp_stage.dart';
import 'stages/creating_stage.dart';
import 'stages/backup_stage.dart';
import 'stages/bio_stage.dart';
import 'stages/name_stage.dart';
import 'stages/ready_stage.dart';
import 'stages/persona_stage.dart';

/// Hosts the onboarding stages as a child Navigator. Each stage is its own
/// route; the initial stack is rebuilt from the persisted step so a killed
/// app resumes where it left off.
class OnboardingFlow extends StatefulWidget {
  const OnboardingFlow({super.key});

  @override
  State<OnboardingFlow> createState() => _OnboardingFlowState();
}

class _OnboardingFlowState extends State<OnboardingFlow> {
  final OnboardingController _controller = OnboardingController();
  List<OnboardingStep>? _initialStack; // null until the saved step resolves

  @override
  void initState() {
    super.initState();
    _resolveInitialStack();
  }

  Future<void> _resolveInitialStack() async {
    final saved = await SecureStorage.get(SecureStorage.keyOnboardingStep);
    if (!mounted) return;
    setState(() => _initialStack = initialStackFor(saved));
  }


  Widget _stageWidget(OnboardingStep step) {
    switch (step) {
      case OnboardingStep.hero:
        return const HeroStage();
      case OnboardingStep.email:
        return const EmailStage();
      case OnboardingStep.emailOtp:
        return const OtpStage();
      case OnboardingStep.creating:
        return const CreatingStage();
      case OnboardingStep.backup:
        return const BackupStage();
      case OnboardingStep.bio:
        return const BioStage();
      case OnboardingStep.name:
        return const NameStage();
      case OnboardingStep.ready:
        return const ReadyStage();
      case OnboardingStep.persona:
        return const PersonaStage();
    }
  }

  /// Returns true for stages that support back navigation (user can correct
  /// prior input or navigate within the post-DKG returnable group).
  /// Returns false for locked stages: hero (stack bottom / no prior input),
  /// creating/backup (DKG hard boundary), bio (biometric auth — must not be
  /// re-entered), and name (post-DKG group floor; back would return to bio).
  bool _canPopStep(OnboardingStep step) {
    switch (step) {
      case OnboardingStep.email:
      case OnboardingStep.emailOtp:
      case OnboardingStep.ready:
      case OnboardingStep.persona:
        return true;
      default: // hero, creating, backup, bio, name
        return false;
    }
  }

  Route<dynamic> _routeFor(OnboardingStep step) => CupertinoPageRoute(
        settings: RouteSettings(name: step.name),
        // PopScope canPop is per-stage: input-correction and the post-DKG
        // returnable group allow back; the DKG boundary and stack floors do
        // not. NavigatorPopHandler in build() bridges system back on Android.
        builder: (_) => PopScope(
          canPop: _canPopStep(step),
          child: _stageWidget(step),
        ),
      );

  Route<dynamic> _onGenerateRoute(RouteSettings settings) {
    final step = stepFromName(settings.name) ?? OnboardingStep.hero;
    return _routeFor(step);
  }

  List<Route<dynamic>> _onGenerateInitialRoutes(
          NavigatorState _, String initialRoute) =>
      _initialStack!.map(_routeFor).toList();

  @override
  Widget build(BuildContext context) {
    if (_initialStack == null) {
      // Brief hold while the persisted step resolves; native splash still covers
      // cold start, so this is only a frame or two.
      return const Scaffold(backgroundColor: CwColors.bgPaper);
    }
    return OnboardingScope(
      controller: _controller,
      // Bridges the root navigator's system back (Android hardware/predictive
      // back) into this child Navigator, so both intra-flow back and the
      // creating/backup PopScope guards actually respond to system back.
      // Without this, system back bypasses the child Navigator entirely.
      child: NavigatorPopHandler(
        onPopWithResult: (_) => _controller.navigatorKey.currentState?.maybePop(),
        child: Navigator(
          key: _controller.navigatorKey,
          initialRoute: _initialStack!.last.name,
          onGenerateInitialRoutes: _onGenerateInitialRoutes,
          onGenerateRoute: _onGenerateRoute,
        ),
      ),
    );
  }
}
