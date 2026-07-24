import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/controller.dart';
import 'package:cowallet/onboarding/routes.dart';
import 'package:cowallet/onboarding/scope.dart';

void main() {
  testWidgets('creating route blocks system back (canPop:false)',
      (tester) async {
    final c = OnboardingController();
    var popAttempted = false;
    Route<dynamic> routeFor(OnboardingStep s) => CupertinoPageRoute(
          settings: RouteSettings(name: s.name),
          builder: (_) => PopScope(
            canPop: false,
            onPopInvokedWithResult: (didPop, _) => popAttempted = true,
            child: const Scaffold(body: Center(child: Text('creating'))),
          ),
        );
    // Wrap in NavigatorPopHandler exactly as OnboardingFlow does, so the test
    // exercises the real system-back -> child Navigator -> PopScope path.
    await tester.pumpWidget(MaterialApp(
      home: OnboardingScope(
        controller: c,
        child: NavigatorPopHandler(
          onPopWithResult: (_) => c.navigatorKey.currentState?.maybePop(),
          child: Navigator(
            key: c.navigatorKey,
            initialRoute: OnboardingStep.creating.name,
            onGenerateInitialRoutes: (_, initial) =>
                [routeFor(OnboardingStep.creating)],
            onGenerateRoute: (_) => routeFor(OnboardingStep.creating),
          ),
        ),
      ),
    ));
    expect(find.text('creating'), findsOneWidget);
    // Let the NavigationNotification from the child PopScope propagate up to
    // NavigatorPopHandler so it registers to intercept system back.
    await tester.pumpAndSettle();

    // Simulate a system back gesture/button. With NavigatorPopHandler wiring,
    // this now reaches the child Navigator's PopScope.
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    // The guard fired (proving system back reached it) AND the route did not
    // leave (canPop:false consumed the pop).
    expect(popAttempted, isTrue,
        reason: 'system back must reach the child PopScope guard');
    expect(find.text('creating'), findsOneWidget,
        reason: 'canPop:false must keep the creating route on screen');
  });
}
