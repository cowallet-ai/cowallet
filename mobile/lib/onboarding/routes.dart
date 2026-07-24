/// The onboarding stages, in true runtime order. Each is a Navigator route.
enum OnboardingStep { hero, email, emailOtp, creating, backup, bio, name, ready, persona }

/// Route-name constants for the onboarding child Navigator. Values equal each
/// [OnboardingStep]'s `.name` so persisted steps map 1:1.
class OnboardingRoutes {
  static const String hero = 'hero';
  static const String email = 'email';
  static const String emailOtp = 'emailOtp';
  static const String creating = 'creating';
  static const String backup = 'backup';
  static const String bio = 'bio';
  static const String name = 'name';
  static const String ready = 'ready';
  static const String persona = 'persona';
}

OnboardingStep? stepFromName(String? name) {
  if (name == null || name.isEmpty) return null;
  for (final s in OnboardingStep.values) {
    if (s.name == name) return s;
  }
  return null;
}

/// Rebuild the child Navigator's initial route stack from a persisted step.
///
/// DKG is a hard boundary: `creating` and `backup` restore as standalone
/// roots (no legal back target), while pre-DKG and the post-DKG returnable
/// group (bio→name→ready→persona) rebuild their full back stack so the swipe
/// gesture steps backwards correctly.
List<OnboardingStep> initialStackFor(String? savedStep) {
  final step = stepFromName(savedStep);
  switch (step) {
    case null:
    case OnboardingStep.hero:
      return const [OnboardingStep.hero];
    case OnboardingStep.email:
      return const [OnboardingStep.hero, OnboardingStep.email];
    case OnboardingStep.emailOtp:
      return const [OnboardingStep.hero, OnboardingStep.email, OnboardingStep.emailOtp];
    case OnboardingStep.creating:
      return const [OnboardingStep.creating];
    case OnboardingStep.backup:
      return const [OnboardingStep.backup];
    case OnboardingStep.bio:
      return const [OnboardingStep.bio];
    case OnboardingStep.name:
      return const [OnboardingStep.bio, OnboardingStep.name];
    case OnboardingStep.ready:
      return const [OnboardingStep.bio, OnboardingStep.name, OnboardingStep.ready];
    case OnboardingStep.persona:
      return const [
        OnboardingStep.bio,
        OnboardingStep.name,
        OnboardingStep.ready,
        OnboardingStep.persona,
      ];
  }
}
