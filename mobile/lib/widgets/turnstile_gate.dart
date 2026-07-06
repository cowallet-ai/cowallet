import 'dart:async';

import 'package:cloudflare_turnstile/cloudflare_turnstile.dart';
import 'package:flutter/material.dart';

import '../config/api_config.dart';
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

    await showDialog<void>(
      context: context,
      barrierDismissible: true,
      builder: (dialogCtx) {
        return Dialog(
          backgroundColor: CwColors.bgCard,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                // Managed widget: usually auto-passes with no interaction.
                CloudflareTurnstile(
                  siteKey: siteKey,
                  options: TurnstileOptions(
                    size: TurnstileSize.normal,
                    theme: TurnstileTheme.auto,
                  ),
                  onTokenReceived: (token) {
                    if (!completer.isCompleted) completer.complete(token);
                    if (Navigator.of(dialogCtx).canPop()) {
                      Navigator.of(dialogCtx).pop();
                    }
                  },
                  onError: (error) {
                    if (!completer.isCompleted) completer.complete(null);
                    if (Navigator.of(dialogCtx).canPop()) {
                      Navigator.of(dialogCtx).pop();
                    }
                  },
                ),
              ],
            ),
          ),
        );
      },
    );

    // Dialog dismissed by tapping outside without a token/error callback.
    if (!completer.isCompleted) completer.complete(null);
    return completer.future;
  }
}
