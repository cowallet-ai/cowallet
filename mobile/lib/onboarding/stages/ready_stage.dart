import 'package:flutter/material.dart';
import '../../l10n/strings.dart';
import '../../main.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/cw_orb.dart';
import '../controller.dart';
import '../routes.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 9: ready (post-name confirmation, before persona).
class ReadyStage extends StatefulWidget {
  const ReadyStage({super.key});

  @override
  State<ReadyStage> createState() => _ReadyStageState();
}

class _ReadyStageState extends State<ReadyStage> {
  @override
  Widget build(BuildContext context) {
    final c = OnboardingScope.of(context);
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(child: _readyStage(context, c)),
    );
  }

  Widget _readyStage(BuildContext context, OnboardingController c) {
    final name = CowalletApp.of(context).userName;
    final h1 = name.isNotEmpty ? S.readyH1Named(name) : S.readyH1;

    return SingleChildScrollView(
      key: const ValueKey('ready'),
      padding: const EdgeInsets.symmetric(horizontal: 28),
      child: Column(
        children: [
          obTopBar(context, showBack: true, onBack: c.goBack),
          const SizedBox(height: 24),
          // CwOrb with checkmark badge
          SizedBox(
            width: 140,
            height: 140,
            child: Stack(
              alignment: Alignment.center,
              children: [
                const CwOrb(size: 120, breathing: true),
                Positioned(
                  right: 8,
                  bottom: 8,
                  child: Container(
                    width: 36,
                    height: 36,
                    decoration: BoxDecoration(
                      color: CwColors.success,
                      shape: BoxShape.circle,
                      border: Border.all(color: CwColors.bgPaper, width: 3),
                    ),
                    child: const Icon(Icons.check, size: 20, color: Colors.white),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 28),
          obHeading(context, h1),
          const SizedBox(height: 8),
          obSubtitle(context, S.readySub),
          const SizedBox(height: 32),
          // "What you can do next" label
          Align(
            alignment: Alignment.centerLeft,
            child: Text(
              S.readyWhat,
              style: Theme.of(context)
                  .textTheme
                  .labelLarge
                  ?.copyWith(color: CwColors.ink3),
            ),
          ),
          const SizedBox(height: 16),
          // 3 numbered next-steps
          _numberedStep(context, 1, S.ready1h, S.ready1s),
          const SizedBox(height: 12),
          _numberedStep(context, 2, S.ready2h, S.ready2s),
          const SizedBox(height: 12),
          _numberedStep(context, 3, S.ready3h, S.ready3s),
          const SizedBox(height: 36),
          obPrimaryButton(S.readyGo, () => c.goTo(OnboardingStep.persona)),
          const SizedBox(height: 24),
        ],
      ),
    );
  }

  Widget _numberedStep(BuildContext context, int n, String title, String sub) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          width: 28,
          height: 28,
          decoration: BoxDecoration(
            color: CwColors.accentSoft,
            shape: BoxShape.circle,
          ),
          alignment: Alignment.center,
          child: Text(
            '$n',
            style: TextStyle(
              fontFamily: CwTypography.monoFamily,
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: CwColors.accent,
            ),
          ),
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
}
