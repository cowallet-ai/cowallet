import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../l10n/s.dart';
import '../theme/colors.dart';
import '../widgets/top_toast.dart' show showTopToast;

/// Full-screen, non-dismissible upgrade wall.
///
/// Shown when the running build is below the server's min_build. The v1.0.1 MPC
/// signing protocol is not backward compatible, so an old client cannot sign
/// and must update. Back-navigation is blocked (PopScope) — there is no valid
/// in-app destination for an unsupported build.
class ForceUpgradeView extends StatelessWidget {
  final String iosStoreUrl;
  final String androidStoreUrl;

  const ForceUpgradeView({
    super.key,
    required this.iosStoreUrl,
    required this.androidStoreUrl,
  });

  String get _storeUrl => Platform.isIOS ? iosStoreUrl : androidStoreUrl;

  Future<void> _openStore(BuildContext context) async {
    final url = _storeUrl;
    if (url.isEmpty) {
      showTopToast(context, S.upgradeStoreOpenFailed);
      return;
    }
    final ok = await launchUrl(
      Uri.parse(url),
      mode: LaunchMode.externalApplication,
    ).catchError((_) => false);
    if (!ok && context.mounted) {
      showTopToast(context, S.upgradeStoreOpenFailed);
    }
  }

  @override
  Widget build(BuildContext context) {
    // canPop: false blocks the OS back gesture/button — an unsupported build
    // has nowhere valid to go.
    return PopScope(
      canPop: false,
      child: Scaffold(
        backgroundColor: CwColors.bgPaper,
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 32),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Icon(Icons.system_update, size: 72, color: CwColors.accent),
                const SizedBox(height: 24),
                Text(
                  S.upgradeTitle,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontSize: 24,
                    fontWeight: FontWeight.w700,
                    color: CwColors.ink1,
                  ),
                ),
                const SizedBox(height: 16),
                Text(
                  S.upgradeMessage,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontSize: 15,
                    height: 1.5,
                    color: CwColors.ink3,
                  ),
                ),
                const SizedBox(height: 40),
                FilledButton(
                  onPressed: () => _openStore(context),
                  style: FilledButton.styleFrom(
                    backgroundColor: CwColors.accent,
                    foregroundColor: Colors.white,
                    padding: const EdgeInsets.symmetric(vertical: 16),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(12),
                    ),
                  ),
                  child: Text(
                    S.upgradeButton,
                    style: const TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
