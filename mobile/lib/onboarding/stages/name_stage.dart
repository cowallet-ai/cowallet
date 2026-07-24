import 'package:flutter/material.dart';
import '../../l10n/strings.dart';
import '../../main.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../controller.dart';
import '../routes.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 6: name entry.
class NameStage extends StatefulWidget {
  const NameStage({super.key});

  @override
  State<NameStage> createState() => _NameStageState();
}

class _NameStageState extends State<NameStage> {
  final _nameCtrl = TextEditingController();

  @override
  void dispose() {
    _nameCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = OnboardingScope.of(context);
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(child: _nameStage(context, c)),
    );
  }

  // ---- Name ----
  void _submitName(OnboardingController c) {
    final name = _nameCtrl.text.trim();
    if (name.isNotEmpty) {
      CowalletApp.of(context).setUserName(name);
    }
    c.goTo(OnboardingStep.ready);
  }

  Widget _nameStage(BuildContext context, OnboardingController c) {
    return SingleChildScrollView(
      key: const ValueKey('name'),
      child: Column(
        children: [
          obTopBar(context, showBack: false, step: 2, total: 3),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SizedBox(height: 24),
                Center(child: obHeading(context, S.nameH1)),
                const SizedBox(height: 28),
                // Text input
                Container(
                  decoration: BoxDecoration(
                    color: CwColors.bgCard,
                    borderRadius: BorderRadius.circular(14),
                    border: Border.all(color: CwColors.line),
                  ),
                  child: TextField(
                    controller: _nameCtrl,
                    textCapitalization: TextCapitalization.words,
                    style: TextStyle(
                      fontFamily: CwTypography.serifFamily,
                      fontSize: 20,
                      fontWeight: FontWeight.w500,
                      color: CwColors.ink1,
                    ),
                    textAlign: TextAlign.center,
                    decoration: InputDecoration(
                      hintText: S.namePlaceholder,
                      hintStyle: TextStyle(
                        fontFamily: CwTypography.serifFamily,
                        fontSize: 20,
                        fontWeight: FontWeight.w400,
                        color: CwColors.ink4,
                      ),
                      contentPadding: const EdgeInsets.symmetric(
                          horizontal: 16, vertical: 18),
                      border: InputBorder.none,
                    ),
                    onSubmitted: (_) => _submitName(c),
                  ),
                ),
                const SizedBox(height: 10),
                // Hint
                Center(
                  child: Text(
                    S.nameHint,
                    style: Theme.of(context)
                        .textTheme
                        .bodySmall
                        ?.copyWith(color: CwColors.ink4),
                  ),
                ),
                const SizedBox(height: 32),
                obPrimaryButton(S.continueBtn, () => _submitName(c)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
