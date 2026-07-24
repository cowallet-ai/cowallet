import 'package:flutter/material.dart';
import 'theme_controller.dart';

/// Claude "paper / ink" palette. Each token resolves at runtime from
/// [ThemeController], so `CwColors.ink1` returns the light or dark value for
/// the app's current brightness with no BuildContext needed.
///
/// Dark values are reverse-engineered from the light palette (warm near-black
/// browns rather than cold pure black, to keep the paper aesthetic). They are
/// a first pass — TODO(design): confirm dark color values.
abstract final class CwColors {
  // ── Paper / ink backgrounds ────────────────────────────────────────────
  static const _lightBgPaper = Color(0xFFFAF9F5);
  static const _darkBgPaper = Color(0xFF1A1712);
  static Color get bgPaper => ThemeController.isDark ? _darkBgPaper : _lightBgPaper;

  static const _lightBgSubtle = Color(0xFFF1EAD9);
  static const _darkBgSubtle = Color(0xFF24201A);
  static Color get bgSubtle => ThemeController.isDark ? _darkBgSubtle : _lightBgSubtle;

  static const _lightBgCard = Color(0xFFFFFFFF);
  static const _darkBgCard = Color(0xFF211D17);
  static Color get bgCard => ThemeController.isDark ? _darkBgCard : _lightBgCard;

  static const _lightBgHover = Color(0xFFEFE7D3);
  static const _darkBgHover = Color(0xFF2E2921);
  static Color get bgHover => ThemeController.isDark ? _darkBgHover : _lightBgHover;

  // ── Ink (text) ─────────────────────────────────────────────────────────
  static const _lightInk1 = Color(0xFF141008);
  static const _darkInk1 = Color(0xFFF5F1E8);
  static Color get ink1 => ThemeController.isDark ? _darkInk1 : _lightInk1;

  static const _lightInk2 = Color(0xFF4A3F32);
  static const _darkInk2 = Color(0xFFD6CDBD);
  static Color get ink2 => ThemeController.isDark ? _darkInk2 : _lightInk2;

  static const _lightInk3 = Color(0xFF8A7A6C);
  static const _darkInk3 = Color(0xFFA99C8A);
  static Color get ink3 => ThemeController.isDark ? _darkInk3 : _lightInk3;

  static const _lightInk4 = Color(0xFFB8A898);
  static const _darkInk4 = Color(0xFF786E60);
  static Color get ink4 => ThemeController.isDark ? _darkInk4 : _lightInk4;

  // ── Lines ──────────────────────────────────────────────────────────────
  static const _lightLine = Color(0xFFE7DFCD);
  static const _darkLine = Color(0xFF332E26);
  static Color get line => ThemeController.isDark ? _darkLine : _lightLine;

  static const _lightLineStrong = Color(0xFFD5C9B0);
  static const _darkLineStrong = Color(0xFF453E33);
  static Color get lineStrong => ThemeController.isDark ? _darkLineStrong : _lightLineStrong;

  // ── Accent: Claude orange (brightened slightly for dark legibility) ──────
  static const _lightAccent = Color(0xFFD97757);
  static const _darkAccent = Color(0xFFE08768);
  static Color get accent => ThemeController.isDark ? _darkAccent : _lightAccent;

  static const _lightAccentHover = Color(0xFFC96744);
  static const _darkAccentHover = Color(0xFFEB9878);
  static Color get accentHover => ThemeController.isDark ? _darkAccentHover : _lightAccentHover;

  static const _lightAccentSoft = Color(0xFFF7E3D8);
  static const _darkAccentSoft = Color(0xFF3A2A22);
  static Color get accentSoft => ThemeController.isDark ? _darkAccentSoft : _lightAccentSoft;

  static const _lightAccentSoft2 = Color(0xFFFBEFE4);
  static const _darkAccentSoft2 = Color(0xFF2F2620);
  static Color get accentSoft2 => ThemeController.isDark ? _darkAccentSoft2 : _lightAccentSoft2;

  // ── Signals (hues brightened, soft variants darkened for dark bg) ────────
  static const _lightDanger = Color(0xFFC0392B);
  static const _darkDanger = Color(0xFFE5654F);
  static Color get danger => ThemeController.isDark ? _darkDanger : _lightDanger;

  static const _lightDangerSoft = Color(0xFFF7DCD8);
  static const _darkDangerSoft = Color(0xFF3A211D);
  static Color get dangerSoft => ThemeController.isDark ? _darkDangerSoft : _lightDangerSoft;

  static const _lightSuccess = Color(0xFF5A7A4E);
  static const _darkSuccess = Color(0xFF7B9A6D);
  static Color get success => ThemeController.isDark ? _darkSuccess : _lightSuccess;

  static const _lightSuccessSoft = Color(0xFFE1ECD9);
  static const _darkSuccessSoft = Color(0xFF232E1F);
  static Color get successSoft => ThemeController.isDark ? _darkSuccessSoft : _lightSuccessSoft;

  static const _lightWarn = Color(0xFFB8832B);
  static const _darkWarn = Color(0xFFD0A050);
  static Color get warn => ThemeController.isDark ? _darkWarn : _lightWarn;

  static const _lightWarnSoft = Color(0xFFF4E8CD);
  static const _darkWarnSoft = Color(0xFF322A1A);
  static Color get warnSoft => ThemeController.isDark ? _darkWarnSoft : _lightWarnSoft;

  static const _lightGold = Color(0xFFA88A4A);
  static const _darkGold = Color(0xFFC6A860);
  static Color get gold => ThemeController.isDark ? _darkGold : _lightGold;

  static const _lightGoldSoft = Color(0xFFF4ECD5);
  static const _darkGoldSoft = Color(0xFF2E2818);
  static Color get goldSoft => ThemeController.isDark ? _darkGoldSoft : _lightGoldSoft;

  static const _lightInfo = Color(0xFF3D6B8C);
  static const _darkInfo = Color(0xFF6296BB);
  static Color get info => ThemeController.isDark ? _darkInfo : _lightInfo;

  static const _lightInfoSoft = Color(0xFFDCE8F0);
  static const _darkInfoSoft = Color(0xFF1C2933);
  static Color get infoSoft => ThemeController.isDark ? _darkInfoSoft : _lightInfoSoft;
}
