import 'package:flutter/material.dart';

import '../../api/auth_api.dart';
import '../../api/shards_api.dart';
import '../../l10n/strings.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../utils/device_id.dart';
import '../../widgets/turnstile_gate.dart';
import '../controller.dart';
import '../routes.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 2.6: email OTP verification.
class OtpStage extends StatefulWidget {
  const OtpStage({super.key});

  @override
  State<OtpStage> createState() => _OtpStageState();
}

class _OtpStageState extends State<OtpStage> {
  final _otpCtrl = TextEditingController();
  String? _otpError;
  bool _otpVerifying = false;

  @override
  void dispose() {
    _otpCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = OnboardingScope.of(context);
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(child: _emailOtpStage(context, c)),
    );
  }

  void _onOtpChanged(BuildContext context, OnboardingController c, String value) {
    if (_otpError != null) setState(() => _otpError = null);
    if (value.length == 6) {
      _verifyEmailOtp(context, c);
    }
  }

  Future<void> _verifyEmailOtp(BuildContext context, OnboardingController c) async {
    if (_otpVerifying) return;
    setState(() => _otpVerifying = true);

    try {
      final deviceId = await DeviceIdGenerator.getOrGenerate();
      var result = await AuthApi.register(
        deviceId: deviceId,
        email: c.email,
        otp: _otpCtrl.text.trim(),
        force: c.forceRegister,
        backupShardHash: c.backupShardHash,
      );

      // 428 on a force re-register means the server has no backup_shard_hash on
      // record for this account (e.g. registered before the hash mechanism, or
      // the hash was never uploaded). We already proved possession of the backup
      // shard by loading it in _verifyBackupShardForReRegister, so back-fill the
      // hash and retry once.
      if (!result.isSuccess &&
          result.errorCode == 428 &&
          c.forceRegister &&
          c.backupShardHash != null) {
        final backfill =
            await ShardsApi.storeBackupHash(backupShardHashHex: c.backupShardHash!);
        if (backfill.isSuccess) {
          result = await AuthApi.register(
            deviceId: deviceId,
            email: c.email,
            otp: _otpCtrl.text.trim(),
            force: c.forceRegister,
            backupShardHash: c.backupShardHash,
          );
        }
      }
      if (!mounted) return;

      if (result.isSuccess) {
        // Keep _otpVerifying latched (do NOT reset to false): we are navigating
        // away, and leaving it true blocks any late onChanged re-entry from
        // re-invoking _verifyEmailOtp and double-pushing the creating route.
        // In the monolith this was guarded by `_stage == _Stage.creating`.
        c.goTo(OnboardingStep.creating);
      } else {
        setState(() {
          _otpVerifying = false;
          _otpError = result.errorMessage ?? S.emailSendFailed;
          _otpCtrl.clear();
        });
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _otpVerifying = false;
        _otpError = S.emailSendFailed;
        _otpCtrl.clear();
      });
    }
  }

  Future<void> _resendOtp(BuildContext context, OnboardingController c) async {
    _otpCtrl.clear();
    final turnstileToken = await TurnstileGate.getToken(context);
    if (turnstileToken == null) {
      if (!mounted) return;
      setState(() => _otpError = S.emailSendFailed);
      return;
    }
    final result =
        await AuthApi.sendEmailOtp(email: c.email, turnstileToken: turnstileToken);
    if (!mounted) return;
    if (!result.isSuccess) {
      setState(() => _otpError = S.emailSendFailed);
    }
  }

  Widget _emailOtpStage(BuildContext context, OnboardingController c) {
    return SingleChildScrollView(
      key: const ValueKey('emailOtp'),
      child: Column(
        children: [
          obTopBar(context, showBack: true, step: 0, total: 3, onBack: c.goBack),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                const SizedBox(height: 40),
                Icon(Icons.mark_email_read_outlined, size: 56, color: CwColors.accent),
                const SizedBox(height: 24),
                obHeading(context, S.otpH1),
                const SizedBox(height: 8),
                obSubtitle(context, S.otpSub(c.email)),
                const SizedBox(height: 32),
                if (_otpError != null) ...[
                  Text(_otpError!, style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 13, color: CwColors.danger)),
                  const SizedBox(height: 12),
                ],
                Container(
                  decoration: BoxDecoration(
                    color: CwColors.bgCard,
                    borderRadius: BorderRadius.circular(14),
                    border: Border.all(color: CwColors.line),
                  ),
                  child: TextField(
                    controller: _otpCtrl,
                    keyboardType: TextInputType.number,
                    textAlign: TextAlign.center,
                    maxLength: 6,
                    autofocus: true,
                    style: const TextStyle(
                      fontSize: 24,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 8,
                      color: CwColors.ink1,
                    ),
                    decoration: InputDecoration(
                      counterText: '',
                      hintText: '------',
                      hintStyle: TextStyle(
                        fontSize: 24,
                        fontWeight: FontWeight.w400,
                        letterSpacing: 8,
                        color: CwColors.ink4,
                      ),
                      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 18),
                      border: InputBorder.none,
                    ),
                    onChanged: (v) => _onOtpChanged(context, c, v),
                  ),
                ),
                const SizedBox(height: 24),
                if (_otpVerifying)
                  const CircularProgressIndicator()
                else
                  obSecondaryLink(S.otpResend, () => _resendOtp(context, c)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
