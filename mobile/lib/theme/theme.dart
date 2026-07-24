import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'colors.dart';
import 'typography.dart';

ThemeData cwTheme() => _cwTheme(Brightness.light);

ThemeData cwDarkTheme() => _cwTheme(Brightness.dark);

ThemeData _cwTheme(Brightness brightness) {
  final isDark = brightness == Brightness.dark;
  final colorScheme = isDark
      ? ColorScheme.dark(
          primary: CwColors.accent,
          onPrimary: Colors.white,
          secondary: CwColors.ink2,
          onSecondary: CwColors.bgPaper,
          surface: CwColors.bgCard,
          onSurface: CwColors.ink1,
          error: CwColors.danger,
          outline: CwColors.line,
          outlineVariant: CwColors.lineStrong,
        )
      : ColorScheme.light(
          primary: CwColors.accent,
          onPrimary: Colors.white,
          secondary: CwColors.ink2,
          onSecondary: CwColors.bgPaper,
          surface: CwColors.bgCard,
          onSurface: CwColors.ink1,
          error: CwColors.danger,
          outline: CwColors.line,
          outlineVariant: CwColors.lineStrong,
        );
  return ThemeData(
    useMaterial3: true,
    brightness: brightness,
    fontFamily: CwTypography.serifFamily,
    scaffoldBackgroundColor: CwColors.bgPaper,
    colorScheme: colorScheme,
    textTheme: CwTypography.textTheme,
    appBarTheme: AppBarTheme(
      backgroundColor: CwColors.bgPaper,
      foregroundColor: CwColors.ink1,
      elevation: 0,
      scrolledUnderElevation: 0,
      systemOverlayStyle:
          isDark ? SystemUiOverlayStyle.light : SystemUiOverlayStyle.dark,
    ),
    cardTheme: CardThemeData(
      color: CwColors.bgCard,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: CwColors.line),
      ),
    ),
    bottomNavigationBarTheme: BottomNavigationBarThemeData(
      backgroundColor: CwColors.bgPaper,
      selectedItemColor: CwColors.accent,
      unselectedItemColor: CwColors.ink4,
      type: BottomNavigationBarType.fixed,
      elevation: 0,
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: CwColors.accent,
        foregroundColor: Colors.white,
        minimumSize: const Size.fromHeight(52),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
        textStyle: TextStyle(
          fontFamily: CwTypography.serifFamily,
          fontSize: 15,
          fontWeight: FontWeight.w600,
        ),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: CwColors.ink1,
        minimumSize: const Size.fromHeight(52),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
        side: BorderSide(color: CwColors.lineStrong),
      ),
    ),
    dividerTheme: DividerThemeData(
      color: CwColors.line,
      thickness: 1,
      space: 0,
    ),
  );
}
