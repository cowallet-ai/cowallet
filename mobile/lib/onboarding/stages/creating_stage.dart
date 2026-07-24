import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import '../../l10n/strings.dart';
import '../../main.dart';
import '../../services/locator.dart';
import '../../theme/colors.dart';
import '../../utils/secure_storage.dart';
import '../../widgets/cw_orb.dart';
import '../controller.dart';
import '../scope.dart';
import 'shared.dart';

/// Stage 3: MPC wallet creation (DKG). DKG boundary — no back navigation.
class CreatingStage extends StatefulWidget {
  const CreatingStage({super.key});

  @override
  State<CreatingStage> createState() => _CreatingStageState();
}

class _CreatingStageState extends State<CreatingStage> {
  double _createProgress = 0;
  int _createChecksDone = 0; // 0..3
  Timer? _createTimer;
  bool _isResuming = false;
  bool _createError = false;

  late OnboardingController _c;
  bool _startedDkg = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _c = OnboardingScope.of(context);
    if (!_startedDkg) {
      _startedDkg = true;
      _startCreating();
    }
  }

  @override
  void dispose() {
    _createTimer?.cancel();
    super.dispose();
  }

  // ---- Creating: MPC wallet generation + backend API integration ----
  void _startCreating() async {
    _createProgress = 0;
    _createChecksDone = 0;
    _createError = false;
    _isResuming = false;

    // Check for resumable session first
    final mpcService = Services.mpcWallet;
    final sessionManager = _c.sessionManager;

    final canResume = await sessionManager.canResume();
    if (canResume && mounted) {
      setState(() => _isResuming = true);
      debugPrint('[CreatingStage] Found resumable session, attempting recovery...');
    }

    bool authDone = false;
    bool mpcSessionDone = false;
    bool mpcProtocolDone = false;
    bool walletDone = false;
    bool animDone = false;
    String? generatedAddress;

    void maybeAdvance() {
      debugPrint('[CreatingStage] maybeAdvance: auth=$authDone mpcSession=$mpcSessionDone mpcProto=$mpcProtocolDone wallet=$walletDone anim=$animDone mounted=$mounted addr=$generatedAddress');
      if (!authDone || !mpcSessionDone || !mpcProtocolDone || !walletDone || !animDone || !mounted) return;
      if (generatedAddress == null) return;
      debugPrint('[CreatingStage] All conditions met, advancing to backup stage');
      try {
        CowalletApp.of(context).setWalletAddress(generatedAddress!);
      } catch (_) {}
      Future.delayed(const Duration(milliseconds: 400), () {
        if (mounted) _c.onDkgSuccess();
      });
    }

    // Registration already completed in OTP stage, proceed directly to DKG
    () async {
      authDone = true;
      mpcSessionDone = true;
      if (mounted) setState(() => _createChecksDone = 2); // ✅ 设备验证 + MPC 会话
      maybeAdvance();

      // DKG 协议（多轮消息交换）
      try {
        final walletInfo = await sessionManager.runDkgWithRecovery();
        generatedAddress = walletInfo.address;

        // Save pending backup shard to SecureStorage
        final backupShard = mpcService.lastBackupShard;
        if (backupShard != null && backupShard.isNotEmpty) {
          final base64Shard = base64Encode(backupShard);
          await SecureStorage.save(SecureStorage.keyPendingBackupShard, base64Shard);
          await SecureStorage.save(SecureStorage.keyPendingBackupCreatedAt, DateTime.now().toIso8601String());
        }

        if (mounted) {
          setState(() {
            _createChecksDone = 3; // ✅ 密钥分片完成
            _isResuming = false;
          });
          mpcProtocolDone = true;
          walletDone = true;
          maybeAdvance();
        }
      } catch (e) {
        if (!mounted) return;
        _createTimer?.cancel();
        setState(() {
          _createError = true;
          _isResuming = false;
        });
        return;
      }
    }();

    // 动画时间线 (最小 2.5 秒保证用户体验)
    const tick = Duration(milliseconds: 50);
    int ticks = 0;
    _createTimer?.cancel();
    _createTimer = Timer.periodic(tick, (t) {
      if (!mounted) {
        t.cancel();
        return;
      }
      ticks++;
      setState(() {
        _createProgress = (ticks / 50).clamp(0.0, 1.0); // 50 ticks = 2.5s
        if (_createProgress >= 1.0) {
          t.cancel();
          animDone = true;
          maybeAdvance();
        }
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return PopScope(
      canPop: false,
      child: Scaffold(
        backgroundColor: CwColors.bgPaper,
        body: SafeArea(child: _creatingStage()),
      ),
    );
  }

  Widget _creatingStage() {
    return SingleChildScrollView(
      key: const ValueKey('creating'),
      child: Column(
        children: [
          obTopBar(context, showBack: false, step: 1, total: 3),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                const SizedBox(height: 24),
                const CwOrb(size: 120, thinking: true),
                const SizedBox(height: 28),
                obHeading(context, _isResuming ? 'Resuming...' : S.creatingH1),
                const SizedBox(height: 8),
                obSubtitle(context, _isResuming ? 'Recovering your wallet session' : S.creatingSub),
                const SizedBox(height: 32),
                // Progress bar
                ClipRRect(
                  borderRadius: BorderRadius.circular(6),
                  child: LinearProgressIndicator(
                    value: _createProgress,
                    minHeight: 8,
                    backgroundColor: CwColors.line,
                    valueColor:
                        const AlwaysStoppedAnimation<Color>(CwColors.accent),
                  ),
                ),
                const SizedBox(height: 8),
                Align(
                  alignment: Alignment.centerRight,
                  child: Text(
                    '${(_createProgress * 100).clamp(0, 100).toInt()}%',
                    style: Theme.of(context)
                        .textTheme
                        .labelMedium
                        ?.copyWith(color: CwColors.ink3),
                  ),
                ),
                const SizedBox(height: 24),
                // 3 check-lines
                _checkLine(S.cl1, _createChecksDone >= 1),
                const SizedBox(height: 12),
                _checkLine(S.cl2, _createChecksDone >= 2),
                const SizedBox(height: 12),
                _checkLine(S.cl3, _createChecksDone >= 3),
                if (_createError) ...[
                  const SizedBox(height: 24),
                  Container(
                    width: double.infinity,
                    padding: const EdgeInsets.all(14),
                    decoration: BoxDecoration(
                      color: CwColors.warnSoft,
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(
                          color: CwColors.warn.withValues(alpha: 0.3)),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.error_outline,
                            size: 20, color: CwColors.warn),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Text(S.createError,
                              style: TextStyle(
                                  fontSize: 13, color: CwColors.ink2)),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 16),
                  obPrimaryButton(S.retry, _startCreating),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _checkLine(String text, bool done) {
    return Row(
      children: [
        AnimatedSwitcher(
          duration: const Duration(milliseconds: 300),
          child: done
              ? Icon(Icons.check_circle, key: ValueKey('$text-done'),
                  size: 20, color: CwColors.success)
              : SizedBox(
                  key: ValueKey('$text-wait'),
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: CwColors.ink4,
                  ),
                ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Text(
            text,
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: done ? CwColors.ink1 : CwColors.ink3,
                ),
          ),
        ),
      ],
    );
  }
}
