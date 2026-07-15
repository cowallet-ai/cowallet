import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../l10n/s.dart';

/// Public privacy policy that describes third-party AI data processing.
const String kPrivacyPolicyUrl = 'https://cowallet.ai/privacy';

/// Blocking bottom sheet that discloses exactly what conversation data is sent
/// to the third-party AI providers and requires explicit opt-in before the AI
/// assistant can be used.
///
/// Returns `true` when the user taps "Agree & continue", `false` (or null)
/// otherwise. It does NOT persist anything itself — the caller stores the
/// result via SettingsService so the sheet is shown only once.
class AiConsentSheet extends StatelessWidget {
  const AiConsentSheet({super.key});

  /// Shows the sheet and resolves to the user's decision (true = consented).
  static Future<bool> show(BuildContext context) async {
    final result = await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      isDismissible: false,
      enableDrag: false,
      backgroundColor: Colors.transparent,
      builder: (_) => const AiConsentSheet(),
    );
    return result ?? false;
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(12),
      constraints: BoxConstraints(
        maxHeight: MediaQuery.of(context).size.height * 0.85,
      ),
      padding: const EdgeInsets.fromLTRB(24, 24, 24, 20),
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(22),
      ),
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Center(
              child: Icon(Icons.auto_awesome_outlined, size: 40, color: CwColors.accent),
            ),
            const SizedBox(height: 16),
            Center(
              child: Text(
                S.aiConsentTitle,
                textAlign: TextAlign.center,
                style: TextStyle(
                  fontFamily: CwTypography.serifFamily,
                  fontSize: 19,
                  fontWeight: FontWeight.w600,
                  color: CwColors.ink1,
                ),
              ),
            ),
            const SizedBox(height: 16),
            Text(
              S.aiConsentIntro,
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 14, height: 1.5, color: CwColors.ink2),
            ),
            const SizedBox(height: 14),
            _bullet(S.aiConsentItemMessage),
            _bullet(S.aiConsentItemWallet),
            _bullet(S.aiConsentItemPortfolio),
            _bullet(S.aiConsentItemContacts),
            _bullet(S.aiConsentItemLocale),
            const SizedBox(height: 14),
            Text(
              S.aiConsentFooter,
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 12.5, height: 1.5, color: CwColors.ink3),
            ),
            const SizedBox(height: 12),
            Center(
              child: TextButton.icon(
                onPressed: _openPrivacy,
                icon: Icon(Icons.open_in_new, size: 15, color: CwColors.accent),
                label: Text(
                  S.aiConsentPrivacyLink,
                  style: TextStyle(fontSize: 13, color: CwColors.accent),
                ),
              ),
            ),
            const SizedBox(height: 8),
            SizedBox(
              width: double.infinity,
              child: FilledButton(
                onPressed: () => Navigator.pop(context, true),
                child: Text(S.aiConsentAgree),
              ),
            ),
            const SizedBox(height: 8),
            SizedBox(
              width: double.infinity,
              child: TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: Text(S.aiConsentDecline, style: TextStyle(color: CwColors.ink3)),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _bullet(String text) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(Icons.check_circle_outline, size: 16, color: CwColors.accent),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              text,
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 13.5, height: 1.4, color: CwColors.ink1),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _openPrivacy() async {
    final uri = Uri.parse(kPrivacyPolicyUrl);
    try {
      await launchUrl(uri, mode: LaunchMode.externalApplication);
    } catch (_) {
      // Best-effort; the button is informational.
    }
  }
}
