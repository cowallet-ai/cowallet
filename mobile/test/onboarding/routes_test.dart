import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/routes.dart';

void main() {
  group('initialStackFor', () {
    test('null or unknown → [hero]', () {
      expect(initialStackFor(null), [OnboardingStep.hero]);
      expect(initialStackFor(''), [OnboardingStep.hero]);
      expect(initialStackFor('bogus'), [OnboardingStep.hero]);
    });

    test('pre-DKG steps rebuild the full back stack', () {
      expect(initialStackFor('email'), [OnboardingStep.hero, OnboardingStep.email]);
      expect(initialStackFor('emailOtp'),
          [OnboardingStep.hero, OnboardingStep.email, OnboardingStep.emailOtp]);
    });

    test('creating restores alone (guarded, auto-resumes)', () {
      expect(initialStackFor('creating'), [OnboardingStep.creating]);
    });

    test('backup is a standalone floor', () {
      expect(initialStackFor('backup'), [OnboardingStep.backup]);
    });

    test('post-DKG returnable group rebuilds from bio', () {
      expect(initialStackFor('bio'), [OnboardingStep.bio]);
      expect(initialStackFor('name'), [OnboardingStep.bio, OnboardingStep.name]);
      expect(initialStackFor('ready'),
          [OnboardingStep.bio, OnboardingStep.name, OnboardingStep.ready]);
      expect(initialStackFor('persona'), [
        OnboardingStep.bio,
        OnboardingStep.name,
        OnboardingStep.ready,
        OnboardingStep.persona,
      ]);
    });

    test('route name constants equal enum .name', () {
      expect(OnboardingRoutes.email, OnboardingStep.email.name);
      expect(OnboardingRoutes.emailOtp, OnboardingStep.emailOtp.name);
      expect(OnboardingRoutes.creating, OnboardingStep.creating.name);
    });
  });
}
