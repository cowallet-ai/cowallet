import 'dart:io';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:pointycastle/digests/sha256.dart';
import 'package:convert/convert.dart' as convert;

import '../../api/auth_api.dart';
import '../../l10n/strings.dart';
import '../../platform/cloud_backup.dart';
import '../../router/app_router.dart';
import '../../services/backup_shard_service.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/top_toast.dart';
import '../../widgets/turnstile_gate.dart';
import '../controller.dart';
import '../routes.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 2.5: email entry.
class EmailStage extends StatefulWidget {
  const EmailStage({super.key});

  @override
  State<EmailStage> createState() => _EmailStageState();
}

class _EmailStageState extends State<EmailStage> {
  final _emailCtrl = TextEditingController();
  String? _emailError;
  bool _emailSending = false;

  late OnboardingController _c;

  @override
  void dispose() {
    _emailCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    _c = OnboardingScope.of(context);
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(child: _emailStage()),
    );
  }

  Future<void> _submitEmail() async {
    final email = _emailCtrl.text.trim();
    if (email.isEmpty || !email.contains('@') || !email.contains('.')) {
      setState(() => _emailError = S.invalidEmail);
      return;
    }
    // Human check before sending. Returns '' in compat mode (not configured),
    // a token on success, or null if the user dismissed / it errored.
    final turnstileToken = await TurnstileGate.getToken(context);
    if (turnstileToken == null) {
      if (!mounted) return;
      setState(() => _emailError = S.emailSendFailed);
      return;
    }

    setState(() {
      _emailError = null;
      _emailSending = true;
    });

    try {
      final result =
          await AuthApi.sendEmailOtp(email: email, turnstileToken: turnstileToken);
      if (!mounted) return;
      if (result.isSuccess) {
        final isRegistered = result.data?["is_registered"] == true;
        setState(() => _emailSending = false);
        if (isRegistered) {
          _showRecoveryDialog();
        } else {
          _c.email = _emailCtrl.text.trim();
          _c.goTo(OnboardingStep.emailOtp);
        }
      } else {
        setState(() {
          _emailSending = false;
          _emailError = result.errorMessage ?? S.emailSendFailed;
        });
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _emailSending = false;
        _emailError = S.emailSendFailed;
      });
    }
  }

  void _showRecoveryDialog() {
    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (ctx) => Container(
        margin: const EdgeInsets.all(16),
        padding: const EdgeInsets.fromLTRB(24, 28, 24, 24),
        decoration: BoxDecoration(
          color: CwColors.bgCard,
          borderRadius: BorderRadius.circular(20),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.info_outline_rounded, size: 40, color: CwColors.accent),
            const SizedBox(height: 16),
            Text(
              S.emailAlreadyRegistered,
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 18, fontWeight: FontWeight.w600, color: CwColors.ink1),
            ),
            const SizedBox(height: 8),
            Text(
              S.emailAlreadyRegisteredDesc,
              textAlign: TextAlign.center,
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 14, color: CwColors.ink3),
            ),
            const SizedBox(height: 24),
            SizedBox(
              width: double.infinity,
              child: FilledButton(
                onPressed: () {
                  Navigator.pop(ctx);
                  Navigator.of(context, rootNavigator: true)
                      .pushNamed(AppRouter.recovery, arguments: _emailCtrl.text.trim());
                },
                child: Text(S.goRecovery),
              ),
            ),
            const SizedBox(height: 10),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton(
                onPressed: () {
                  Navigator.pop(ctx);
                  _showReRegisterConfirm();
                },
                child: Text(S.reRegister),
              ),
            ),
            const SizedBox(height: 10),
            SizedBox(
              width: double.infinity,
              child: TextButton(
                onPressed: () => Navigator.pop(ctx),
                child: Text(S.cancel, style: TextStyle(color: CwColors.ink3)),
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _showReRegisterConfirm() {
    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (ctx) => Container(
        margin: const EdgeInsets.all(16),
        padding: const EdgeInsets.fromLTRB(24, 28, 24, 24),
        decoration: BoxDecoration(
          color: CwColors.bgCard,
          borderRadius: BorderRadius.circular(20),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.warning_amber_rounded, size: 40, color: CwColors.danger),
            const SizedBox(height: 16),
            Text(
              S.reRegister,
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 18, fontWeight: FontWeight.w600, color: CwColors.ink1),
            ),
            const SizedBox(height: 8),
            Text(
              S.reRegisterDesc,
              textAlign: TextAlign.center,
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 14, color: CwColors.ink3),
            ),
            const SizedBox(height: 24),
            SizedBox(
              width: double.infinity,
              child: FilledButton(
                style: FilledButton.styleFrom(backgroundColor: CwColors.danger),
                onPressed: () {
                  Navigator.pop(ctx);
                  _verifyBackupShardForReRegister();
                },
                child: Text(S.reRegisterConfirm),
              ),
            ),
            const SizedBox(height: 10),
            SizedBox(
              width: double.infinity,
              child: TextButton(
                onPressed: () => Navigator.pop(ctx),
                child: Text(S.cancel, style: TextStyle(color: CwColors.ink3)),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _verifyBackupShardForReRegister() async {
    final backupService = BackupShardService(PlatformCloudBackup());
    final shardBytes = await backupService.retrieveFromCloud();
    if (!mounted) return;

    // Cloud backup found — proceed directly.
    if (shardBytes != null && shardBytes.isNotEmpty) {
      await _continueReRegisterWithShard(shardBytes);
      return;
    }

    // No cloud backup: the shard may have been saved to a local file. Let the
    // user pick their backup file instead of dead-ending on "backup required".
    await _pickLocalBackupForReRegister(backupService);
  }

  /// Prompt the user to select their local backup file, parse the shard from
  /// it, and continue re-registration. Used when no cloud backup is present.
  Future<void> _pickLocalBackupForReRegister(
    BackupShardService backupService,
  ) async {
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['json'],
      );
      if (!mounted) return;
      if (result == null || result.files.isEmpty) return; // user cancelled

      final filePath = result.files.single.path;
      if (filePath == null) {
        showTopToast(context, S.backupShardRequired, backgroundColor: CwColors.danger);
        return;
      }

      final content = await File(filePath).readAsString();
      final shardBytes = backupService.parseBackupFile(content);
      if (!mounted) return;

      if (shardBytes == null || shardBytes.isEmpty) {
        showTopToast(context, S.backupFormatInvalid, backgroundColor: CwColors.danger);
        return;
      }
      await _continueReRegisterWithShard(shardBytes);
    } catch (_) {
      if (!mounted) return;
      showTopToast(context, S.backupFormatInvalid, backgroundColor: CwColors.danger);
    }
  }

  /// Shared tail of re-registration: hash the backup shard, re-send the OTP
  /// with the force flag, and advance to the OTP stage.
  Future<void> _continueReRegisterWithShard(List<int> shardBytes) async {
    final digest = SHA256Digest();
    final hash = digest.process(Uint8List.fromList(shardBytes));
    final backupShardHash = convert.hex.encode(hash);

    // Human check before the forced re-send.
    final turnstileToken = await TurnstileGate.getToken(context);
    if (turnstileToken == null) {
      if (!mounted) return;
      showTopToast(context, S.emailSendFailed, backgroundColor: CwColors.danger);
      return;
    }

    // Re-send OTP with force flag since the initial send was blocked
    final result = await AuthApi.sendEmailOtp(
      email: _emailCtrl.text.trim(),
      force: true,
      turnstileToken: turnstileToken,
    );
    if (!mounted) return;

    if (!result.isSuccess) {
      showTopToast(context, S.emailSendFailed, backgroundColor: CwColors.danger);
      return;
    }

    _c.email = _emailCtrl.text.trim();
    _c.forceRegister = true;
    _c.backupShardHash = backupShardHash;
    _c.goTo(OnboardingStep.emailOtp);
  }

  Widget _emailStage() {
    return SingleChildScrollView(
      key: const ValueKey('email'),
      child: Column(
        children: [
          obTopBar(context, showBack: true, step: 0, total: 3, onBack: _c.goBack),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SizedBox(height: 40),
                Center(
                  child: Icon(Icons.email_outlined, size: 56, color: CwColors.accent),
                ),
                const SizedBox(height: 24),
                Center(child: obHeading(context, S.emailH1)),
                const SizedBox(height: 8),
                Center(child: obSubtitle(context, S.emailSub)),
                const SizedBox(height: 32),
                Container(
                  decoration: BoxDecoration(
                    color: CwColors.bgCard,
                    borderRadius: BorderRadius.circular(14),
                    border: Border.all(
                      color: _emailError != null ? CwColors.danger : CwColors.line,
                    ),
                  ),
                  child: TextField(
                    controller: _emailCtrl,
                    keyboardType: TextInputType.emailAddress,
                    autocorrect: false,
                    style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 16, color: CwColors.ink1),
                    decoration: InputDecoration(
                      hintText: 'your@email.com',
                      hintStyle: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 16, color: CwColors.ink4),
                      contentPadding: const EdgeInsets.symmetric(
                          horizontal: 16, vertical: 16),
                      border: InputBorder.none,
                      prefixIcon: Icon(Icons.mail_outline, color: CwColors.ink3),
                    ),
                    onSubmitted: (_) => _submitEmail(),
                    onChanged: (_) {
                      if (_emailError != null) setState(() => _emailError = null);
                    },
                  ),
                ),
                if (_emailError != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    _emailError!,
                    style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 13, color: CwColors.danger),
                  ),
                ],
                const SizedBox(height: 12),
                Text(
                  S.emailHint,
                  style: Theme.of(context)
                      .textTheme
                      .bodySmall
                      ?.copyWith(color: CwColors.ink4),
                ),
                const SizedBox(height: 32),
                SizedBox(
                  width: double.infinity,
                  child: FilledButton(
                    onPressed: _emailSending ? null : _submitEmail,
                    child: _emailSending
                        ? const SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                          )
                        : Text(S.continueBtn),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
