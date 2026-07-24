import 'package:flutter/material.dart';

/// Single source of truth for the app's effective brightness.
///
/// [CwColors] getters read [brightness] synchronously (no BuildContext), and
/// the top-level app writes it whenever the resolved brightness changes. Keep
/// this the ONLY place that decides light vs dark — MaterialApp renders the
/// matching theme from the same value, so the two can never drift.
abstract final class ThemeController {
  static final ValueNotifier<Brightness> brightness =
      ValueNotifier(Brightness.light);

  static bool get isDark => brightness.value == Brightness.dark;

  /// Collapse a [ThemeMode] plus the OS-reported [platform] brightness into the
  /// single brightness we actually render. `system` defers to the OS.
  static Brightness resolve(ThemeMode mode, Brightness platform) =>
      switch (mode) {
        ThemeMode.light => Brightness.light,
        ThemeMode.dark => Brightness.dark,
        ThemeMode.system => platform,
      };
}
