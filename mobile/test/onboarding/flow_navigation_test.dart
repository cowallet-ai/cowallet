import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/controller.dart';
import 'package:cowallet/onboarding/routes.dart';
import 'package:cowallet/onboarding/scope.dart';

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
    case OnboardingStep.email:
      return Scaffold(
        appBar: AppBar(leading: BackButton(onPressed: c.goBack)),
        body: const Center(child: Text('email')),
      );
    default:
      return Scaffold(body: Center(child: Text(step.name)));
  }
}

Widget _host(OnboardingController c, List<OnboardingStep> stack) {
  Route<dynamic> routeFor(OnboardingStep s) => CupertinoPageRoute(
        settings: RouteSettings(name: s.name),
        builder: (_) => _fakeStage(c, s),
      );
  return MaterialApp(
    home: OnboardingScope(
      controller: c,
      child: Navigator(
        key: c.navigatorKey,
        initialRoute: stack.last.name,
        onGenerateInitialRoutes: (_, initial) =>
            stack.map(routeFor).toList(),
        onGenerateRoute: (s) =>
            routeFor(stepFromName(s.name) ?? OnboardingStep.hero),
      ),
    ),
  );
}

void main() {
  testWidgets('forward push then back returns to previous stage',
      (tester) async {
    final c = OnboardingController();
    await tester.pumpWidget(_host(c, [OnboardingStep.hero]));
    expect(find.text('hero'), findsOneWidget);

    await tester.tap(find.byKey(const Key('to-email')));
    await tester.pumpAndSettle();
    expect(find.text('email'), findsOneWidget);
    expect(find.text('hero'), findsNothing);

    c.goBack();
    await tester.pumpAndSettle();
    expect(find.text('hero'), findsOneWidget);
  });
}
