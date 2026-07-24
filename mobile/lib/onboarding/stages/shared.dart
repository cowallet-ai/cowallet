import 'package:flutter/material.dart';
import 'package:cowallet/theme/typography.dart';
import 'package:cowallet/theme/colors.dart';
import '../../l10n/strings.dart';

/// Top bar with optional back button and progress dots.
Widget obTopBar(BuildContext context,
    {bool showBack = false, int? step, int total = 3, VoidCallback? onBack}) {
  return Padding(
    padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
    child: Row(
      children: [
        if (showBack)
          GestureDetector(
            onTap: onBack,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.arrow_back_ios_new,
                    size: 16, color: CwColors.ink3),
                const SizedBox(width: 4),
                Text(S.back,
                    style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 14, color: CwColors.ink3)),
              ],
            ),
          )
        else
          const SizedBox(width: 48),
        const Spacer(),
        if (step != null) obProgressDots(step, total),
        const Spacer(),
        const SizedBox(width: 48),
      ],
    ),
  );
}

Widget obProgressDots(int current, int total) {
  return Row(
    mainAxisSize: MainAxisSize.min,
    children: List.generate(total, (i) {
      final isActive = i == current;
      final isDone = i < current;
      return Container(
        width: 8,
        height: 8,
        margin: const EdgeInsets.symmetric(horizontal: 4),
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: (isActive || isDone) ? CwColors.accent : CwColors.line,
        ),
      );
    }),
  );
}

Widget obHeading(BuildContext context, String text) {
  return Text(
    text,
    style: Theme.of(context).textTheme.displayMedium,
    textAlign: TextAlign.center,
  );
}

Widget obSubtitle(BuildContext context, String text) {
  return Text(
    text,
    style: Theme.of(context).textTheme.bodyLarge?.copyWith(
          color: CwColors.ink2,
        ),
    textAlign: TextAlign.center,
  );
}

Widget obPrimaryButton(String label, VoidCallback? onPressed) {
  return SizedBox(
    width: double.infinity,
    child: FilledButton(
      onPressed: onPressed,
      child: Text(label),
    ),
  );
}

Widget obSecondaryLink(String label, VoidCallback onPressed) {
  return TextButton(
    onPressed: onPressed,
    child: Text(label, style: TextStyle(color: CwColors.ink3, fontSize: 14)),
  );
}

Widget obFeatureRow(BuildContext context, IconData icon, String title, String sub) {
  return Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Container(
        width: 40,
        height: 40,
        decoration: BoxDecoration(
          color: CwColors.accentSoft,
          borderRadius: BorderRadius.circular(10),
        ),
        child: Icon(icon, size: 20, color: CwColors.accent),
      ),
      const SizedBox(width: 14),
      Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title,
                style: Theme.of(context)
                    .textTheme
                    .titleMedium
                    ?.copyWith(color: CwColors.ink1)),
            const SizedBox(height: 2),
            Text(sub,
                style: Theme.of(context)
                    .textTheme
                    .bodySmall
                    ?.copyWith(color: CwColors.ink3)),
          ],
        ),
      ),
    ],
  );
}
