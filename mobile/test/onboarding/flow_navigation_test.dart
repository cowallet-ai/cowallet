import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/controller.dart';
import 'package:cowallet/onboarding/routes.dart';
import 'package:cowallet/onboarding/scope.dart';

// Fake stages: hero has a forward button; every other stage is plain text.
// No stage renders a back affordance — onboarding is forward-only.
Widget _fakeStage(OnboardingController c, OnboardingStep step) {
  switch (step) {
    case OnboardingStep.hero:
      return Scaffold(
        body: Center(
          child: TextButton(
            key: const Key('to-email'),
            onPressed: () => c.goTo(OnboardingStep.email),
            child: const Text('hero'),
          ),
        ),
      );
    default:
      return Scaffold(body: Center(child: Text(step.name)));
  }
}

// Mirrors OnboardingFlow's host: every route is wrapped in
// PopScope(canPop:false) and the child Navigator is bridged to system back
// via NavigatorPopHandler.
Widget _host(OnboardingController c, List<OnboardingStep> stack) {
  Route<dynamic> routeFor(OnboardingStep s) => CupertinoPageRoute(
        settings: RouteSettings(name: s.name),
        builder: (_) => PopScope(
          canPop: false,
          child: _fakeStage(c, s),
        ),
      );
  return MaterialApp(
    home: OnboardingScope(
      controller: c,
      child: NavigatorPopHandler(
        onPopWithResult: (_) => c.navigatorKey.currentState?.maybePop(),
        child: Navigator(
          key: c.navigatorKey,
          initialRoute: stack.last.name,
          onGenerateInitialRoutes: (_, initial) =>
              stack.map(routeFor).toList(),
          onGenerateRoute: (s) =>
              routeFor(stepFromName(s.name) ?? OnboardingStep.hero),
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('forward push advances to the next stage', (tester) async {
    final c = OnboardingController();
    await tester.pumpWidget(_host(c, [OnboardingStep.hero]));
    expect(find.text('hero'), findsOneWidget);

    await tester.tap(find.byKey(const Key('to-email')));
    await tester.pumpAndSettle();
    expect(find.text('email'), findsOneWidget);
    expect(find.text('hero'), findsNothing);
  });

  testWidgets('system back does not leave a non-first stage (forward-only)',
      (tester) async {
    final c = OnboardingController();
    // Seed a two-deep stack so a pop *could* return to hero if unguarded.
    await tester.pumpWidget(_host(c, [OnboardingStep.hero, OnboardingStep.email]));
    await tester.pumpAndSettle();
    expect(find.text('email'), findsOneWidget);

    // Simulate Android system back — must be consumed by the per-route guard,
    // never popping back to hero.
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.text('email'), findsOneWidget);
    expect(find.text('hero'), findsNothing);
  });
}
