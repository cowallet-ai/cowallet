import 'package:flutter/widgets.dart';
import 'controller.dart';

/// Provides the [OnboardingController] to the onboarding stage subtree.
class OnboardingScope extends InheritedWidget {
  const OnboardingScope({
    super.key,
    required this.controller,
    required super.child,
  });

  final OnboardingController controller;

  static OnboardingController of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<OnboardingScope>();
    assert(scope != null, 'OnboardingScope.of() called with no scope in tree');
    return scope!.controller;
  }

  @override
  bool updateShouldNotify(OnboardingScope oldWidget) =>
      !identical(oldWidget.controller, controller);
}
