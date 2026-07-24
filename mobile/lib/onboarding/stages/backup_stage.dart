import 'dart:convert';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:pointycastle/digests/sha256.dart';
import 'package:convert/convert.dart' as convert;
import '../../api/shards_api.dart';
import '../../l10n/strings.dart';
import '../../services/locator.dart';
import '../../services/backup_shard_service.dart';
import '../../services/mpc_wallet_service.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../utils/secure_storage.dart';
import '../../widgets/top_toast.dart';
import '../controller.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 7: backup (store 3rd shard). DKG boundary — no back navigation.
class BackupStage extends StatefulWidget {
  const BackupStage({super.key});

  @override
  State<BackupStage> createState() => _BackupStageState();
}

class _BackupStageState extends State<BackupStage> {
  bool _backupSaving = false;
  bool _backupDone = false;

  late OnboardingController _c;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _c = OnboardingScope.of(context);
  }

  @override
  Widget build(BuildContext context) {
    // Back is blocked centrally by the host's per-route PopScope.
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(child: _backupStage()),
    );
  }

  // ---- Backup: store 3rd shard ----
  Future<void> _saveBackup({required bool useCloud}) async {
    setState(() => _backupSaving = true);
    try {
      final walletService = Services.wallet as MpcWalletService;
      var backupBytes = walletService.lastBackupShard;

      // If not in memory, try loading from pending storage
      if (backupBytes == null || backupBytes.isEmpty) {
        final pendingShard = await SecureStorage.get(SecureStorage.keyPendingBackupShard);
        if (pendingShard != null && pendingShard.isNotEmpty) {
          backupBytes = base64Decode(pendingShard);
          debugPrint('[BackupStage] Loaded backup shard from pending storage');
        }
      }

      if (backupBytes == null || backupBytes.length != 32) {
        throw BackupException(BackupError.shardNotAvailable);
      }

      final result = await walletService.storeBackupShard(
        backupBytes,
        useCloud: useCloud,
      );

      // Delete pending backup shard after successful backup
      await SecureStorage.delete(SecureStorage.keyPendingBackupShard);
      await SecureStorage.delete(SecureStorage.keyPendingBackupCreatedAt);
      debugPrint('[BackupStage] Deleted pending backup shard after successful backup');

      // Store SHA-256(backup_shard) on server for future force re-register verification.
      final digest = SHA256Digest();
      final hash = digest.process(Uint8List.fromList(backupBytes));
      final hashHex = convert.hex.encode(hash);
      final hashResult = await ShardsApi.storeBackupHash(backupShardHashHex: hashHex);
      if (hashResult.isSuccess) {
        debugPrint('[BackupStage] Stored backup shard hash on server');
      } else {
        // Non-fatal: save hash locally for retry later
        await SecureStorage.save('pending_backup_hash', hashHex);
        debugPrint('[BackupStage] Failed to store backup hash, saved locally for retry');
      }

      if (!mounted) return;

      setState(() {
        _backupSaving = false;
        _backupDone = true;
      });

      final msg = result.method == BackupMethod.cloud
          ? S.backupSaved
          : S.backupFileSaved(result.filePath ?? '');
      showTopToast(context, msg, backgroundColor: CwColors.success);

      Future.delayed(const Duration(milliseconds: 600), () {
        if (mounted) {
          _c.backupDone = true;
          _c.goToBioFromBackup();
        }
      });
    } catch (e, st) {
      debugPrint('[BackupStage] Error: $e');
      debugPrint('[BackupStage] StackTrace: $st');
      if (!mounted) return;
      setState(() => _backupSaving = false);
      final errMsg = switch (e) {
        BackupException(error: BackupError.cloudUnavailable) => S.backupErrCloudUnavailable,
        BackupException(error: BackupError.cloudStoreFailed) => S.backupErrCloudStoreFailed,
        BackupException(error: BackupError.fileWriteFailed) => S.backupErrFileWriteFailed,
        BackupException(error: BackupError.shardNotAvailable) => S.backupErrShardNotAvailable,
        _ => S.backupErrCloudStoreFailed,
      };
      showTopToast(context, errMsg, backgroundColor: CwColors.danger);
    }
  }

  void _skipBackup() {
    _c.backupSkipped = true;
    _c.goToBioFromBackup();
  }

  Widget _backupStage() {
    return SingleChildScrollView(
      key: const ValueKey('backup'),
      child: Column(
        children: [
          obTopBar(context, showBack: false, step: 1, total: 3),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                const SizedBox(height: 24),
                Icon(Icons.shield_outlined, size: 64, color: CwColors.accent),
                const SizedBox(height: 24),
                obHeading(context, S.backupH1),
                const SizedBox(height: 8),
                obSubtitle(context, S.backupSub),
                const SizedBox(height: 32),
                if (_backupDone) ...[
                  Container(
                    width: double.infinity,
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: CwColors.success.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(14),
                      border: Border.all(color: CwColors.success.withValues(alpha: 0.3)),
                    ),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(Icons.check_circle, size: 20, color: CwColors.success),
                        const SizedBox(width: 10),
                        Text(
                          S.backupSaved,
                          style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 15, color: CwColors.success, fontWeight: FontWeight.w600),
                        ),
                      ],
                    ),
                  ),
                ] else if (_backupSaving) ...[
                  const CircularProgressIndicator(),
                  const SizedBox(height: 16),
                  Text(S.backupSaving, style: TextStyle(color: CwColors.ink3)),
                ] else ...[
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
                  const SizedBox(height: 24),
                  Center(
                    child: obSecondaryLink(S.backupSkip, _skipBackup),
                  ),
                ],
                const SizedBox(height: 24),
              ],
            ),
          ),
        ],
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
                      style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 13, color: CwColors.ink3)),
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
