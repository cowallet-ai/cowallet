import 'dart:async';

import 'package:cloudflare_turnstile/cloudflare_turnstile.dart';
import 'package:flutter/material.dart';

import '../config/api_config.dart';
import '../l10n/s.dart';
import '../theme/colors.dart';

/// Human/bot verification gate backed by Cloudflare Turnstile.
///
/// Call [TurnstileGate.getToken] before an abuse-prone unauthenticated request
/// (e.g. sending an email OTP). Returns a token string to pass to the backend.
///
/// Compatibility: when no site key is configured
/// ([ApiConfig.turnstileSiteKey] empty), this returns an empty string and the
/// backend — also in compat mode — skips enforcement. So the whole feature is
/// a no-op until both a site key (client) and secret key (server) are set.
class TurnstileGate {
  /// Obtain a Turnstile token, showing a brief modal that hosts the widget.
  ///
  /// Returns:
  ///  - the token on success,
  ///  - `''` when Turnstile is not configured (compat mode),
  ///  - `null` when the user dismissed the check or it errored.
  static Future<String?> getToken(BuildContext context) async {
    final siteKey = ApiConfig.turnstileSiteKey;
    if (siteKey.isEmpty) {
      // Not configured — compat mode, nothing to verify.
      return '';
    }

    final completer = Completer<String?>();

    void finish(BuildContext dialogCtx, String? value) {
      if (!completer.isCompleted) completer.complete(value);
      if (Navigator.of(dialogCtx).canPop()) {
        Navigator.of(dialogCtx).pop();
      }
    }

    await showDialog<void>(
      context: context,
      // Do NOT dismiss on outside tap or back — an accidental tap would cancel
      // the check and block the OTP. The user cancels only via the explicit
      // button below.
      barrierDismissible: false,
      builder: (dialogCtx) {
        return PopScope(
          canPop: false,
          child: Dialog(
            backgroundColor: CwColors.bgCard,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(16),
            ),
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    S.turnstileTitle,
                    style: const TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                      color: CwColors.ink1,
                    ),
                  ),
                  const SizedBox(height: 16),
                  // Managed widget: usually auto-passes with no interaction.
                  // Reserve height so the dialog doesn't jump while it loads.
                  ConstrainedBox(
                    constraints: const BoxConstraints(minHeight: 70),
                    child: Center(
                      child: CloudflareTurnstile(
                        siteKey: siteKey,
                        // The widget runs inside a webview; its origin must
                        // match a hostname allowed for this Turnstile widget in
                        // the Cloudflare dashboard. Default origin is
                        // http://localhost, which is NOT whitelisted (→ error
                        // 110200). Pin it to the registered domain so the check
                        // passes on device in both dev and prod.
                        baseUrl: 'https://cowallet.ai',
                        options: TurnstileOptions(
                          size: TurnstileSize.normal,
                          theme: TurnstileTheme.auto,
                        ),
                        onTokenReceived: (token) => finish(dialogCtx, token),
                        onError: (error) => finish(dialogCtx, null),
                      ),
                    ),
                  ),
                  const SizedBox(height: 8),
                  Align(
                    alignment: Alignment.centerRight,
                    child: TextButton(
                      onPressed: () => finish(dialogCtx, null),
                      child: Text(S.cancel),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );

    // Safety net: if the dialog closed without completing (should not happen
    // given barrierDismissible/PopScope are off), treat as cancelled.
    if (!completer.isCompleted) completer.complete(null);
    return completer.future;
  }
}
