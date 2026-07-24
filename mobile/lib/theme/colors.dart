import 'package:flutter/material.dart';
import 'theme_controller.dart';

/// Claude "paper / ink" palette. Each token resolves at runtime from
/// [ThemeController], so `CwColors.ink1` returns the light or dark value for
/// the app's current brightness with no BuildContext needed.
///
/// Dark values are reverse-engineered from the light palette (warm near-black
/// browns rather than cold pure black, to keep the paper aesthetic).
abstract final class CwColors {
  /// Pick a token's light or dark value based on the current brightness. The
  /// single place the brightness branch lives, so adding a token is one line
  /// and the isDark check can't be miswired per-token.
  static Color _r({required Color light, required Color dark}) =>
      ThemeController.isDark ? dark : light;

  // ── Paper / ink backgrounds ────────────────────────────────────────────
  static Color get bgPaper => _r(light: const Color(0xFFFAF9F5), dark: const Color(0xFF1A1712));
  static Color get bgSubtle => _r(light: const Color(0xFFF1EAD9), dark: const Color(0xFF24201A));
  static Color get bgCard => _r(light: const Color(0xFFFFFFFF), dark: const Color(0xFF211D17));
  static Color get bgHover => _r(light: const Color(0xFFEFE7D3), dark: const Color(0xFF2E2921));

  // ── Ink (text) ─────────────────────────────────────────────────────────
  static Color get ink1 => _r(light: const Color(0xFF141008), dark: const Color(0xFFF5F1E8));
  static Color get ink2 => _r(light: const Color(0xFF4A3F32), dark: const Color(0xFFD6CDBD));
  static Color get ink3 => _r(light: const Color(0xFF8A7A6C), dark: const Color(0xFFA99C8A));
  static Color get ink4 => _r(light: const Color(0xFFB8A898), dark: const Color(0xFF786E60));

  // ── Lines ──────────────────────────────────────────────────────────────
  static Color get line => _r(light: const Color(0xFFE7DFCD), dark: const Color(0xFF332E26));
  static Color get lineStrong => _r(light: const Color(0xFFD5C9B0), dark: const Color(0xFF453E33));

  // ── Accent: Claude orange (brightened slightly for dark legibility) ──────
  static Color get accent => _r(light: const Color(0xFFD97757), dark: const Color(0xFFE08768));
  static Color get accentHover => _r(light: const Color(0xFFC96744), dark: const Color(0xFFEB9878));
  static Color get accentSoft => _r(light: const Color(0xFFF7E3D8), dark: const Color(0xFF3A2A22));
  static Color get accentSoft2 => _r(light: const Color(0xFFFBEFE4), dark: const Color(0xFF2F2620));

  // ── Signals (hues brightened, soft variants darkened for dark bg) ────────
  static Color get danger => _r(light: const Color(0xFFC0392B), dark: const Color(0xFFE5654F));
  static Color get dangerSoft => _r(light: const Color(0xFFF7DCD8), dark: const Color(0xFF3A211D));
  static Color get success => _r(light: const Color(0xFF5A7A4E), dark: const Color(0xFF7B9A6D));
  static Color get successSoft => _r(light: const Color(0xFFE1ECD9), dark: const Color(0xFF232E1F));
  static Color get warn => _r(light: const Color(0xFFB8832B), dark: const Color(0xFFD0A050));
  static Color get warnSoft => _r(light: const Color(0xFFF4E8CD), dark: const Color(0xFF322A1A));
  static Color get gold => _r(light: const Color(0xFFA88A4A), dark: const Color(0xFFC6A860));
  static Color get goldSoft => _r(light: const Color(0xFFF4ECD5), dark: const Color(0xFF2E2818));
  static Color get info => _r(light: const Color(0xFF3D6B8C), dark: const Color(0xFF6296BB));
  static Color get infoSoft => _r(light: const Color(0xFFDCE8F0), dark: const Color(0xFF1C2933));
}
