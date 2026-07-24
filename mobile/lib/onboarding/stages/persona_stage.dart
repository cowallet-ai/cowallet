import 'package:flutter/material.dart';
import '../../l10n/strings.dart';
import '../../main.dart';
import '../../theme/colors.dart';
import '../controller.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 8: persona selection (final stage; leads to app home).
class PersonaStage extends StatefulWidget {
  const PersonaStage({super.key});

  @override
  State<PersonaStage> createState() => _PersonaStageState();
}

class _PersonaStageState extends State<PersonaStage> {
  String? _selectedPersona;

  @override
  Widget build(BuildContext context) {
    final c = OnboardingScope.of(context);
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(child: _personaStage(context, c)),
    );
  }

  // ---- Persona ----
  void _pickPersona(String id) {
    setState(() => _selectedPersona = id);
    CowalletApp.of(context).setPersona(id);
    _finish();
  }

  void _skipPersona() => _finish();

  // ---- Finish ----
  Future<void> _finish() async {
    final app = CowalletApp.of(context);
    app.completeOnboarding();
    final c = OnboardingScope.of(context);
    await c.finish(context, app.walletAddress);
  }

  Widget _personaStage(BuildContext context, OnboardingController c) {
    return SingleChildScrollView(
      key: const ValueKey('persona'),
      child: Column(
        children: [
          obTopBar(context, showBack: false),
          const SizedBox(height: 24),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                obHeading(context, S.personaH1),
                const SizedBox(height: 8),
                obSubtitle(context, S.personaSub),
                const SizedBox(height: 28),
                _personaCard(
                  id: 'daily',
                  icon: Icons.wb_sunny_outlined,
                  title: S.personaDaily,
                  desc: S.personaDailyDesc,
                  tag: S.personaDailyTag,
                ),
                const SizedBox(height: 12),
                _personaCard(
                  id: 'trader',
                  icon: Icons.candlestick_chart,
                  title: S.personaTrader,
                  desc: S.personaTraderDesc,
                ),
                const SizedBox(height: 12),
                _personaCard(
                  id: 'family',
                  icon: Icons.people_outline,
                  title: S.personaFamily,
                  desc: S.personaFamilyDesc,
                  tag: S.personaFamilyTag,
                ),
                const SizedBox(height: 12),
                _personaCard(
                  id: 'builder',
                  icon: Icons.terminal,
                  title: S.personaBuilder,
                  desc: S.personaBuilderDesc,
                ),
                const SizedBox(height: 24),
                obSecondaryLink(S.personaSkip, _skipPersona),
                const SizedBox(height: 24),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _personaCard({
    required String id,
    required IconData icon,
    required String title,
    required String desc,
    String? tag,
  }) {
    final selected = _selectedPersona == id;
    return GestureDetector(
      onTap: () => _pickPersona(id),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        width: double.infinity,
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: selected ? CwColors.accentSoft : CwColors.bgCard,
          borderRadius: BorderRadius.circular(16),
          border: Border.all(
            color: selected ? CwColors.accent : CwColors.line,
            width: selected ? 2 : 1,
          ),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: selected
                    ? CwColors.accent.withValues(alpha: 0.15)
                    : CwColors.accentSoft,
                borderRadius: BorderRadius.circular(10),
              ),
              child: Icon(icon, size: 20, color: CwColors.accent),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(title,
                            style: Theme.of(context)
                                .textTheme
                                .titleMedium
                                ?.copyWith(
                                    color: CwColors.ink1,
                                    fontWeight: FontWeight.w600)),
                      ),
                      if (tag != null)
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 2),
                          decoration: BoxDecoration(
                            color: CwColors.accentSoft,
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Text(tag,
                              style: TextStyle(
                                  fontSize: 11,
                                  color: CwColors.accent,
                                  fontWeight: FontWeight.w600)),
                        ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(desc,
                      style: Theme.of(context)
                          .textTheme
                          .bodyMedium
                          ?.copyWith(color: CwColors.ink3)),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
