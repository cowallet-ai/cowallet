import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';
import 'colors.dart';

abstract final class CwTypography {
  static final String serifFamily = GoogleFonts.notoSerifSc().fontFamily!;
  static final String serifEnFamily = GoogleFonts.fraunces().fontFamily!;
  static final String sansFamily = GoogleFonts.inter().fontFamily!;
  static final String monoFamily = GoogleFonts.jetBrainsMono().fontFamily!;

  static TextTheme get textTheme => GoogleFonts.notoSerifScTextTheme().copyWith(
    displayLarge: GoogleFonts.notoSerifSc(
      fontSize: 32,
      fontWeight: FontWeight.w600,
      color: CwColors.ink1,
      height: 1.3,
    ),
    displayMedium: GoogleFonts.notoSerifSc(
      fontSize: 26,
      fontWeight: FontWeight.w600,
      color: CwColors.ink1,
      height: 1.3,
    ),
    displaySmall: GoogleFonts.notoSerifSc(
      fontSize: 22,
      fontWeight: FontWeight.w600,
      color: CwColors.ink1,
      height: 1.4,
    ),
    headlineLarge: GoogleFonts.notoSerifSc(
      fontSize: 22,
      fontWeight: FontWeight.w600,
      color: CwColors.ink1,
      height: 1.4,
    ),
    headlineMedium: GoogleFonts.notoSerifSc(
      fontSize: 20,
      fontWeight: FontWeight.w500,
      color: CwColors.ink1,
      height: 1.4,
    ),
    headlineSmall: GoogleFonts.notoSerifSc(
      fontSize: 18,
      fontWeight: FontWeight.w500,
      color: CwColors.ink1,
      height: 1.4,
    ),
    titleLarge: GoogleFonts.notoSerifSc(
      fontSize: 17,
      fontWeight: FontWeight.w600,
      color: CwColors.ink1,
    ),
    titleMedium: GoogleFonts.notoSerifSc(
      fontSize: 15,
      fontWeight: FontWeight.w500,
      color: CwColors.ink2,
    ),
    titleSmall: GoogleFonts.notoSerifSc(
      fontSize: 13.5,
      fontWeight: FontWeight.w500,
      color: CwColors.ink2,
    ),
    bodyLarge: GoogleFonts.notoSerifSc(
      fontSize: 15,
      fontWeight: FontWeight.w400,
      color: CwColors.ink1,
      height: 1.6,
    ),
    bodyMedium: GoogleFonts.notoSerifSc(
      fontSize: 13,
      fontWeight: FontWeight.w400,
      color: CwColors.ink2,
      height: 1.5,
    ),
    bodySmall: GoogleFonts.notoSerifSc(
      fontSize: 11,
      fontWeight: FontWeight.w400,
      color: CwColors.ink3,
    ),
    labelLarge: GoogleFonts.jetBrainsMono(
      fontSize: 13,
      fontWeight: FontWeight.w500,
      color: CwColors.ink2,
      letterSpacing: 0.5,
    ),
    labelMedium: GoogleFonts.jetBrainsMono(
      fontSize: 11,
      fontWeight: FontWeight.w400,
      color: CwColors.ink3,
    ),
    labelSmall: GoogleFonts.jetBrainsMono(
      fontSize: 10,
      fontWeight: FontWeight.w500,
      color: CwColors.ink3,
      letterSpacing: 0.5,
    ),
  );

  // --- Semantic styles for common patterns ---

  // Mono: amounts, addresses, hashes, numeric data
  static TextStyle mono({
    double fontSize = 13,
    FontWeight fontWeight = FontWeight.w500,
    Color? color,
    double? letterSpacing,
    double? height,
  }) => TextStyle(
    fontFamily: monoFamily,
    fontSize: fontSize,
    fontWeight: fontWeight,
    color: color ?? CwColors.ink2,
    letterSpacing: letterSpacing ?? 0.3,
    height: height,
  );

  // Serif: Chinese display text, headings
  static TextStyle serif({
    double fontSize = 16,
    FontWeight fontWeight = FontWeight.w600,
    Color? color,
    double? height,
  }) => TextStyle(
    fontFamily: serifFamily,
    fontSize: fontSize,
    fontWeight: fontWeight,
    color: color ?? CwColors.ink1,
    height: height,
  );

  // SerifEn: English display/brand text
  static TextStyle serifEn({
    double fontSize = 18,
    FontWeight fontWeight = FontWeight.w500,
    Color? color,
    double? height,
  }) => TextStyle(
    fontFamily: serifEnFamily,
    fontSize: fontSize,
    fontWeight: fontWeight,
    color: color ?? CwColors.ink1,
    height: height,
  );

  // Sans: general UI text (default, usually not needed explicitly)
  static TextStyle sans({
    double fontSize = 14,
    FontWeight fontWeight = FontWeight.w400,
    Color? color,
    double? height,
  }) => TextStyle(
    fontFamily: sansFamily,
    fontSize: fontSize,
    fontWeight: fontWeight,
    color: color ?? CwColors.ink1,
    height: height,
  );
}
