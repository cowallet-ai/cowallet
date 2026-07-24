import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/onboarding/controller.dart';
import 'package:cowallet/onboarding/scope.dart';

void main() {
  testWidgets('OnboardingScope.of returns the provided controller', (tester) async {
    final controller = OnboardingController();
    late OnboardingController resolved;
    await tester.pumpWidget(
      OnboardingScope(
        controller: controller,
        child: Builder(builder: (ctx) {
          resolved = OnboardingScope.of(ctx);
          return const SizedBox();
        }),
      ),
    );
    expect(identical(resolved, controller), isTrue);
  });
}
