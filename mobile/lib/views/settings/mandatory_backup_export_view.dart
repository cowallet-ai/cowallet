import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../bridge/mpc_bridge.dart';
import '../../l10n/s.dart';
import '../../services/locator.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/top_toast.dart';

/// Full-screen, non-dismissible backup export shown after a key rotation.
///
/// After a proactive reshare, the device/server shards are refreshed and the
/// backup shard is recomputed. Any previously-exported offline backup no longer
/// lies on the new polynomial, so it can NO LONGER recover the wallet. If the
/// user skipped re-exporting and later lost the device, funds would be
/// unrecoverable. This screen blocks back-navigation until the user has
/// exported (and thereby persisted) the refreshed backup.
///
/// Only reached when [MpcWalletService.backupNeedsReExport] is true — i.e. the
/// user's backup method is offline file, not auto-updated cloud.
class MandatoryBackupExportView extends StatefulWidget {
  const MandatoryBackupExportView({super.key});

  @override
  State<MandatoryBackupExportView> createState() =>
      _MandatoryBackupExportViewState();
}

class _MandatoryBackupExportViewState extends State<MandatoryBackupExportView> {
  final _passwordController = TextEditingController();
  final _confirmController = TextEditingController();
  bool _isExporting = false;
  bool _done = false;
  String? _error;
  bool _obscurePassword = true;
  bool _obscureConfirm = true;

  @override
  void dispose() {
    _passwordController.dispose();
    _confirmController.dispose();
    super.dispose();
  }

  Future<void> _doExportAndSave() async {
    final password = _passwordController.text;
    final confirm = _confirmController.text;

    if (password.length < 8) {
      setState(() => _error = S.backupPasswordTooShort);
      return;
    }
    if (password != confirm) {
      setState(() => _error = S.backupPasswordMismatch);
      return;
    }

    setState(() {
      _isExporting = true;
      _error = null;
    });

    try {
      // Ensure the refreshed shard is in Rust memory (Party 2). On a cold start
      // after app-kill the in-memory slot is empty, so load the staged shard
      // that reshare persisted to SecureStorage before exporting.
      final staged = await Services.mpcWallet.loadStagedBackupShard();
      if (staged == null) {
        throw Exception(S.backupExportFailed);
      }
      await MpcBridge.loadBackupShardForExport(staged);

      // Persist the refreshed shard as a password-encrypted file. This records
      // the backup method and writes the file to disk.
      final filePath = await Services.backup.exportEncryptedToFile(password);

      // Clear the post-rotation re-export requirement now that the refreshed
      // shard is safely persisted (this path bypasses storeBackupShard).
      await Services.mpcWallet.markBackupReExported();

      if (!mounted) return;
      setState(() {
        _isExporting = false;
        _done = true;
      });
      showTopToast(context, S.backupFileSaved(filePath),
          backgroundColor: CwColors.success);
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _isExporting = false;
        _error = '${S.backupExportFailed}: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return PopScope(
      // Block back/gesture navigation until the backup is exported.
      canPop: _done,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop && !_done) {
          showTopToast(context, S.mandatoryBackupExitBlocked,
              backgroundColor: CwColors.warn);
        }
      },
      child: Scaffold(
        backgroundColor: CwColors.bgPaper,
        appBar: AppBar(
          backgroundColor: CwColors.bgPaper,
          elevation: 0,
          automaticallyImplyLeading: false,
          title: Text(S.mandatoryBackupTitle,
              style: const TextStyle(color: CwColors.ink1, fontSize: 17)),
        ),
        body: SafeArea(
          child: ListView(
            padding: const EdgeInsets.all(20),
            children: [
              // Warning banner
              Container(
                padding: const EdgeInsets.all(14),
                decoration: BoxDecoration(
                  color: CwColors.warnSoft,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: CwColors.warn),
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Icon(Icons.warning_amber_rounded,
                        color: CwColors.warn, size: 22),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Text(
                        S.mandatoryBackupBody,
                        style: TextStyle(
                          fontFamily: CwTypography.serifFamily,
                          fontSize: 13,
                          height: 1.4,
                          color: CwColors.ink2,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 20),

              if (_done) ...[
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: CwColors.successSoft,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.check_circle,
                          color: CwColors.success, size: 22),
                      const SizedBox(width: 10),
                      Text(S.mandatoryBackupDone,
                          style: const TextStyle(
                              color: CwColors.ink1,
                              fontWeight: FontWeight.w600)),
                    ],
                  ),
                ),
                const SizedBox(height: 20),
                SizedBox(
                  width: double.infinity,
                  height: 48,
                  child: ElevatedButton(
                    onPressed: () => Navigator.of(context).pop(),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: CwColors.accent,
                      foregroundColor: Colors.white,
                      shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(12)),
                    ),
                    child: Text(S.confirm),
                  ),
                ),
              ] else ...[
                // Password field
                TextField(
                  controller: _passwordController,
                  obscureText: _obscurePassword,
                  decoration: InputDecoration(
                    labelText: S.backupPasswordHint,
                    border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12)),
                    suffixIcon: IconButton(
                      icon: Icon(
                          _obscurePassword
                              ? Icons.visibility_off
                              : Icons.visibility,
                          size: 20),
                      onPressed: () => setState(
                          () => _obscurePassword = !_obscurePassword),
                    ),
                  ),
                ),
                const SizedBox(height: 12),

                // Confirm password field
                TextField(
                  controller: _confirmController,
                  obscureText: _obscureConfirm,
                  decoration: InputDecoration(
                    labelText: S.backupPasswordConfirmHint,
                    border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12)),
                    suffixIcon: IconButton(
                      icon: Icon(
                          _obscureConfirm
                              ? Icons.visibility_off
                              : Icons.visibility,
                          size: 20),
                      onPressed: () =>
                          setState(() => _obscureConfirm = !_obscureConfirm),
                    ),
                  ),
                ),
                const SizedBox(height: 8),

                if (_error != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: Text(_error!,
                        style: TextStyle(
                            fontFamily: CwTypography.serifFamily,
                            fontSize: 12,
                            color: CwColors.danger)),
                  ),

                const SizedBox(height: 12),
                SizedBox(
                  width: double.infinity,
                  height: 48,
                  child: ElevatedButton(
                    onPressed: _isExporting ? null : _doExportAndSave,
                    style: ElevatedButton.styleFrom(
                      backgroundColor: CwColors.accent,
                      foregroundColor: Colors.white,
                      shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(12)),
                    ),
                    child: _isExporting
                        ? const SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(
                                strokeWidth: 2, color: Colors.white),
                          )
                        : Text(S.backupSaveToFile),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
