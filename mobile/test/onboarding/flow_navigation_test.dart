import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/controller.dart';
import 'package:cowallet/onboarding/routes.dart';
import 'package:cowallet/onboarding/scope.dart';

// Mirrors OnboardingFlow's per-stage canPop logic.
bool _canPopStep(OnboardingStep step) {
  switch (step) {
    case OnboardingStep.email:
    case OnboardingStep.emailOtp:
    case OnboardingStep.name:
    case OnboardingStep.ready:
    case OnboardingStep.persona:
      return true;
    default:
      return false;
  }
}

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

Widget _host(OnboardingController c, List<OnboardingStep> stack) {
  Route<dynamic> routeFor(OnboardingStep s) => CupertinoPageRoute(
        settings: RouteSettings(name: s.name),
        builder: (_) => PopScope(
          canPop: _canPopStep(s),
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

  testWidgets('system back works on a returnable stage (email → hero)',
      (tester) async {
    final c = OnboardingController();
    await tester.pumpWidget(
        _host(c, [OnboardingStep.hero, OnboardingStep.email]));
    // Let NavigationNotification settle so NavigatorPopHandler registers.
    await tester.pumpAndSettle();
    expect(find.text('email'), findsOneWidget);

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    // email is a returnable stage — back pops it and reveals hero.
    expect(find.text('hero'), findsOneWidget);
    expect(find.text('email'), findsNothing);
  });

  testWidgets('system back is blocked on a locked stage (bio group floor)',
      (tester) async {
    final c = OnboardingController();
    // bio is canPop:false — seeded alone to simulate it as the group floor.
    await tester.pumpWidget(_host(c, [OnboardingStep.bio]));
    await tester.pumpAndSettle();
    expect(find.text('bio'), findsOneWidget);

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    // bio is a locked stage — system back must not leave it.
    expect(find.text('bio'), findsOneWidget);
  });
}
