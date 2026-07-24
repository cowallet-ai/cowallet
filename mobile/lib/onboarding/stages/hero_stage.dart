import 'package:flutter/material.dart';
import '../../l10n/strings.dart';
import '../../theme/colors.dart';
import '../../widgets/cw_orb.dart';
import '../controller.dart';
import '../routes.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 1+2: hero + intro (PageView).
class HeroStage extends StatefulWidget {
  const HeroStage({super.key});

  @override
  State<HeroStage> createState() => _HeroStageState();
}

class _HeroStageState extends State<HeroStage> {
  final PageController _pageCtrl = PageController();
  int _guidePage = 0;

  @override
  void dispose() {
    _pageCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = OnboardingScope.of(context);
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(child: _heroStage(context, c)),
    );
  }

  Widget _heroStage(BuildContext context, OnboardingController c) {
    return Column(
      key: const ValueKey('hero'),
      children: [
        Expanded(
          child: PageView(
            controller: _pageCtrl,
            onPageChanged: (i) => setState(() => _guidePage = i),
            children: [
              _heroPage(context),
              _introPageContent(context),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(bottom: 32, left: 28, right: 28),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Page indicator dots
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: List.generate(2, (i) {
                  return AnimatedContainer(
                    duration: const Duration(milliseconds: 200),
                    width: i == _guidePage ? 20 : 8,
                    height: 8,
                    margin: const EdgeInsets.symmetric(horizontal: 4),
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(4),
                      color: i == _guidePage ? CwColors.accent : CwColors.line,
                    ),
                  );
                }),
              ),
              const SizedBox(height: 24),
              // CTA button
              obPrimaryButton(
                _guidePage == 0 ? S.getStarted : S.introStart,
                () {
                  if (_guidePage == 0) {
                    _pageCtrl.animateToPage(1,
                        duration: const Duration(milliseconds: 300),
                        curve: Curves.easeInOut);
                  } else {
                    c.goTo(OnboardingStep.email);
                  }
                },
              ),
              const SizedBox(height: 12),
              TextButton(
                onPressed: () => Navigator.of(context, rootNavigator: true)
                    .pushNamed('/recovery'),
                child: Text(
                  S.recoverWallet,
                  style: TextStyle(color: CwColors.ink3, fontSize: 14),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                S.heroLegal,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: CwColors.ink4,
                      fontSize: 11,
                    ),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _heroPage(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 28),
      child: Column(
        children: [
          const SizedBox(height: 48),
          const CwOrb(size: 140, breathing: true),
          const SizedBox(height: 28),
          Text(
            S.heroKicker,
            style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: CwColors.ink3,
                  letterSpacing: 1.2,
                ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 12),
          RichText(
            textAlign: TextAlign.center,
            text: TextSpan(
              style: Theme.of(context).textTheme.displayLarge,
              children: [
                TextSpan(text: S.heroH1a),
                if (S.heroH1b.isNotEmpty)
                  TextSpan(
                    text: ' ${S.heroH1b} ',
                    style: Theme.of(context).textTheme.displayLarge,
                  ),
                TextSpan(
                  text: S.heroH1em,
                  style: Theme.of(context).textTheme.displayLarge?.copyWith(
                        fontStyle: FontStyle.italic,
                        color: CwColors.accent,
                      ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          Text(
            S.heroExplain,
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: CwColors.ink2,
                ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 32),
          obFeatureRow(context, Icons.touch_app_outlined, S.heroFeat1h, S.heroFeat1s),
          const SizedBox(height: 16),
          obFeatureRow(context, Icons.public, S.heroFeat2h, S.heroFeat2s),
          const SizedBox(height: 16),
          obFeatureRow(context, Icons.auto_awesome, S.heroFeat3h, S.heroFeat3s),
          const SizedBox(height: 24),
        ],
      ),
    );
  }

  Widget _introPageContent(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 28),
      child: Column(
        children: [
          const SizedBox(height: 48),
          Icon(Icons.lock_outline, size: 64, color: CwColors.accent),
          const SizedBox(height: 24),
          obHeading(context, S.introH1),
          const SizedBox(height: 12),
          obSubtitle(context, S.introSub),
          const SizedBox(height: 32),
          obFeatureRow(context, Icons.call_split, S.introBullet1h, S.introBullet1s),
          const SizedBox(height: 16),
          obFeatureRow(context, Icons.verified_user_outlined, S.introBullet2h, S.introBullet2s),
          const SizedBox(height: 16),
          obFeatureRow(context, Icons.hide_source_outlined, S.introBullet3h, S.introBullet3s),
          const SizedBox(height: 24),
        ],
      ),
    );
  }
}
