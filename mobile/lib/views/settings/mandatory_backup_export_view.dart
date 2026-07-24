import 'dart:async';

import 'package:flutter/material.dart';

import '../../l10n/s.dart';
import '../../services/backup_shard_service.dart';
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
/// re-saved the refreshed backup.
///
/// The storage options here are intentionally IDENTICAL to the onboarding
/// backup step (cloud / local file, no password) so the two flows are aligned.
/// Only reached when [MpcWalletService.backupNeedsReExport] is true.
class MandatoryBackupExportView extends StatefulWidget {
  const MandatoryBackupExportView({super.key});

  @override
  State<MandatoryBackupExportView> createState() =>
      _MandatoryBackupExportViewState();
}

class _MandatoryBackupExportViewState extends State<MandatoryBackupExportView> {
  bool _isSaving = false;
  bool _done = false;
  String? _error;

  Future<void> _saveBackup({required bool useCloud}) async {
    setState(() {
      _isSaving = true;
      _error = null;
    });

    try {
      // Load the refreshed shard that reshare staged to SecureStorage (survives
      // app-kill). Same 32-byte device+server backup contribution the onboarding
      // flow persists via storeBackupShard.
      final staged = await Services.mpcWallet.loadStagedBackupShard();
      if (staged == null || staged.length != 32) {
        throw Exception(S.backupExportFailed);
      }

      // Store via the SAME path as onboarding — cloud or plain file, no
      // password. storeBackupShard clears the in-memory backup state on success.
      final result =
          await Services.mpcWallet.storeBackupShard(staged, useCloud: useCloud);

      // Clear the post-rotation re-export requirement and staging slots.
      await Services.mpcWallet.markBackupReExported();

      if (!mounted) return;
      setState(() {
        _isSaving = false;
        _done = true;
      });
      final msg = result.method == BackupMethod.cloud
          ? S.backupSaved
          : S.backupFileSaved(result.filePath ?? '');
      showTopToast(context, msg, backgroundColor: CwColors.success);
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _isSaving = false;
        _error = '${S.backupExportFailed}: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return PopScope(
      // Block back/gesture navigation until the backup is re-saved.
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
              style: TextStyle(color: CwColors.ink1, fontSize: 17)),
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
                    Icon(Icons.warning_amber_rounded,
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
                      Icon(Icons.check_circle,
                          color: CwColors.success, size: 22),
                      const SizedBox(width: 10),
                      Text(S.mandatoryBackupDone,
                          style: TextStyle(
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
              ] else if (_isSaving) ...[
                const Center(child: CircularProgressIndicator()),
                const SizedBox(height: 16),
                Center(
                  child: Text(S.backupSaving,
                      style: TextStyle(color: CwColors.ink3)),
                ),
              ] else ...[
                // Same two options as the onboarding backup step: cloud / file.
                _backupOptionCard(
                  icon: Icons.cloud_upload_outlined,
                  title: S.backupCloudTitle,
                  desc: S.backupCloudDesc,
                  onTap: () => _saveBackup(useCloud: true),
                ),
                const SizedBox(height: 12),
                _backupOptionCard(
                  icon: Icons.save_alt_outlined,
                  title: S.backupFileTitle,
                  desc: S.backupFileDesc,
                  onTap: () => _saveBackup(useCloud: false),
                ),
                if (_error != null) ...[
                  const SizedBox(height: 12),
                  Text(_error!,
                      style: TextStyle(
                          fontFamily: CwTypography.serifFamily,
                          fontSize: 12,
                          color: CwColors.danger)),
                ],
              ],
            ],
          ),
        ),
      ),
    );
  }

  Widget _backupOptionCard({
    required IconData icon,
    required String title,
    required String desc,
    required VoidCallback onTap,
  }) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: CwColors.bgCard,
          borderRadius: BorderRadius.circular(16),
          border: Border.all(color: CwColors.line),
        ),
        child: Row(
          children: [
            Container(
              width: 44,
              height: 44,
              decoration: BoxDecoration(
                color: CwColors.accentSoft,
                borderRadius: BorderRadius.circular(12),
              ),
              child: Icon(icon, size: 22, color: CwColors.accent),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title,
                      style: TextStyle(
                          fontSize: 15,
                          fontWeight: FontWeight.w600,
                          color: CwColors.ink1)),
                  const SizedBox(height: 2),
                  Text(desc,
                      style: TextStyle(
                          fontFamily: CwTypography.serifFamily,
                          fontSize: 13,
                          color: CwColors.ink3)),
                ],
              ),
            ),
            Icon(Icons.chevron_right, size: 20, color: CwColors.ink4),
          ],
        ),
      ),
    );
  }
}
