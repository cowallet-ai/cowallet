import 'package:flutter/material.dart';
import '../../l10n/strings.dart';
import '../../services/locator.dart';
import '../../services/mpc_wallet_service.dart';
import '../../platform/se_manager.dart';
import '../../platform/sb_manager.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../controller.dart';
import '../routes.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 5: biometric/device-auth setup. Group floor — no back navigation.
class BioStage extends StatefulWidget {
  const BioStage({super.key});

  @override
  State<BioStage> createState() => _BioStageState();
}

class _BioStageState extends State<BioStage> {
  bool _bioAuthenticating = false;
  bool _bioDone = false;

  @override
  Widget build(BuildContext context) {
    final c = OnboardingScope.of(context);
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(child: _bioStage(context, c)),
    );
  }

  // ---- Biometric setup ----
  // The user reaches this screen AFTER DKG (the device shard is still only in
  // Rust memory — runDkg no longer auto-persists it). When the user opts into
  // biometric protection here, we persist the shard under the hardware-backed
  // auth-bound key, which triggers the biometric/device-credential prompt now —
  // as a result of the user's explicit choice, not automatically mid-DKG.
  Future<void> _startBioScan(OnboardingController c) async {
    // Reentrancy guard: this runs several async steps (availability checks,
    // keystore init, shard persistence + native prompt). Without this a rapid
    // double-tap — or a tap during the gap before the first setState rebuilds —
    // would fire it concurrently, repeating initializeWallet/persistDeviceShard
    // and causing the "hangs and stays tappable" behaviour. Bail if already
    // running or done.
    if (_bioAuthenticating || _bioDone) return;

    // Immediately update UI before any async work
    setState(() {
      _bioAuthenticating = true;
    });

    try {
      // The device shard is hardware-backed and its key is bound to device
      // auth. If the device has NO lock at all (no biometric, no passcode),
      // there is nothing to bind to — guide the user to set one up in system
      // settings, then let them retry. We do NOT proceed without a lock.
      final supported = await Services.biometrics.isDeviceSupported();
      if (!mounted) return;
      if (!supported) {
        setState(() => _bioAuthenticating = false);
        await Services.promptDeviceSecuritySetup();
        return;
      }

      // Keep _bioAuthenticating = true through the slow steps below (keystore
      // init + shard persistence, which run BEFORE the native prompt appears).
      // Previously this was reset to false here, so the spinner vanished and the
      // button reappeared — letting the user tap again during the wait and
      // defeating the reentrancy guard.

      // Mark device-auth protection as enabled and initialize the
      // hardware-backed key store. The native prompt (biometric, with device
      // passcode fallback) fires during persistDeviceShard below.
      await Services.biometrics.setEnabled(true);

      final seManager = SecureEnclaveManager();
      final sbManager = StrongBoxManager();
      if (await seManager.isAvailable()) {
        await seManager.initializeWallet('onboarding');
      } else if (await sbManager.isAvailable()) {
        await sbManager.initializeWallet('onboarding');
      } else {
        setState(() {
          _bioAuthenticating = false;
        });
        return;
      }

      // Persist the device shard now — this fires the biometric prompt as a
      // direct result of the user enabling protection here.
      final walletService = Services.wallet as MpcWalletService;
      await walletService.persistDeviceShard();

      if (!mounted) return;
      setState(() => _bioDone = true);
      Future.delayed(const Duration(milliseconds: 600), () {
        if (mounted) c.goTo(OnboardingStep.name);
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _bioAuthenticating = false;
      });
    }
  }

  Widget _bioStage(BuildContext context, OnboardingController c) {
    return SingleChildScrollView(
      key: const ValueKey('bio'),
      child: Column(
        children: [
          obTopBar(context, showBack: false, step: 2, total: 3),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                const SizedBox(height: 40),
                Icon(
                  _bioDone ? Icons.check_circle : Icons.fingerprint,
                  size: 64,
                  color: _bioDone ? CwColors.success : CwColors.accent,
                ),
                const SizedBox(height: 32),
                obHeading(context, _bioDone ? S.bioDone : S.bioH1),
                const SizedBox(height: 8),
                obSubtitle(context, S.bioSub),
                const SizedBox(height: 40),
                if (!_bioDone && !_bioAuthenticating) ...[
                  obPrimaryButton(S.bioActivate, () => _startBioScan(c)),
                  // Device auth (biometric + system passcode fallback) is the
                  // only protection path. If the device has no lock configured,
                  // _startBioScan guides the user to system settings.
                ],
                if (_bioAuthenticating) ...[
                  const SizedBox(
                    width: 28,
                    height: 28,
                    child: CircularProgressIndicator(strokeWidth: 2.5),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    S.bioVerifying,
                    style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 14, color: CwColors.ink3),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}
