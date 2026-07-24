import 'package:cowallet/theme/typography.dart';
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';
import 'package:pointycastle/digests/sha256.dart';
import 'package:convert/convert.dart' as convert;
import '../theme/colors.dart';
import '../widgets/cw_orb.dart';
import '../widgets/top_toast.dart';
import '../widgets/turnstile_gate.dart';
import '../l10n/strings.dart';
import '../main.dart';
import '../services/locator.dart';
import '../api/auth_api.dart';
import '../api/shards_api.dart';
import '../services/mpc_wallet_service.dart';
import '../services/mpc_session_manager.dart';
import '../platform/se_manager.dart';
import '../platform/sb_manager.dart';
import '../utils/device_id.dart';
import '../utils/secure_storage.dart';
import '../services/backup_shard_service.dart';
import '../platform/cloud_backup.dart';
import '../router/app_router.dart';

/// The onboarding stages of cowallet.
enum _Stage { hero, intro, email, emailOtp, creating, bio, name, backup, ready, persona }

class OnboardingFlow extends StatefulWidget {
  const OnboardingFlow({super.key});

  @override
  State<OnboardingFlow> createState() => _OnboardingFlowState();
}

class _OnboardingFlowState extends State<OnboardingFlow> {
  _Stage _stage = _Stage.hero;

  // --- Intro PageView state ---
  final PageController _pageCtrl = PageController();
  int _guidePage = 0;

  // --- Creating stage state ---
  double _createProgress = 0;
  int _createChecksDone = 0; // 0..3
  Timer? _createTimer;
  bool _isResuming = false; // New: flag for resuming interrupted session

  // --- Bio stage state ---
  bool _bioAuthenticating = false;
  bool _bioDone = false;

  // --- Email stage state ---
  final _emailCtrl = TextEditingController();
  String? _emailError;
  bool _emailSending = false;

  // --- Email OTP stage state ---
  final _otpCtrl = TextEditingController();
  String? _otpError;
  bool _otpVerifying = false;
  bool _forceRegister = false;
  String? _backupShardHash;

  // --- Name stage state ---
  final _nameCtrl = TextEditingController();

  bool _createError = false;


  // --- Persona stage state ---
  String? _selectedPersona;

  // --- Backup stage state ---
  bool _backupSkipped = false;
  bool _backupSaving = false;
  bool _backupDone = false;

  // Keep track of navigation history for back button
  final List<_Stage> _history = [];


  @override
  void initState() {
    super.initState();
    _restoreStep();
  }

  Future<void> _restoreStep() async {
    final saved = await SecureStorage.get(SecureStorage.keyOnboardingStep);
    if (saved == null || saved.isEmpty) return;

    final stage = _Stage.values.where((s) => s.name == saved).firstOrNull;
    if (stage == null) return;

    // Don't restore to 'creating' — that needs a fresh DKG run
    if (stage == _Stage.creating) return;

    if (mounted) {
      // Build history so back button works from restored stage
      final stageOrder = [_Stage.hero, _Stage.intro, _Stage.email, _Stage.emailOtp, _Stage.creating, _Stage.bio, _Stage.name, _Stage.backup, _Stage.ready, _Stage.persona];
      final targetIdx = stageOrder.indexOf(stage);
      setState(() {
        _history.clear();
        for (int i = 0; i < targetIdx && i < stageOrder.length; i++) {
          _history.add(stageOrder[i]);
        }
        _stage = stage;
      });
    }
  }

  @override
  void dispose() {
    _createTimer?.cancel();
    _emailCtrl.dispose();
    _otpCtrl.dispose();
    _nameCtrl.dispose();
    _pageCtrl.dispose();
    super.dispose();
  }


  void _goTo(_Stage next) {
    setState(() {
      _history.add(_stage);
      _stage = next;
    });
    SecureStorage.save(SecureStorage.keyOnboardingStep, next.name);
    if (next == _Stage.creating) _startCreating();
  }

  void _goBack() {
    if (_history.isNotEmpty) {
      setState(() {
        _stage = _history.removeLast();
      });
    }
  }

  // ---- Creating: MPC wallet generation + backend API integration ----
  void _startCreating() async {
    _createProgress = 0;
    _createChecksDone = 0;
    _createError = false;
    _isResuming = false;

    // Check for resumable session first
    final mpcService = Services.mpcWallet;
    final sessionManager = MpcSessionManager(mpcService);

    final canResume = await sessionManager.canResume();
    if (canResume && mounted) {
      setState(() => _isResuming = true);
      debugPrint('[OnboardingFlow] Found resumable session, attempting recovery...');
    }

    bool authDone = false;
    bool mpcSessionDone = false;
    bool mpcProtocolDone = false;
    bool walletDone = false;
    bool animDone = false;
    String? generatedAddress;

    void maybeAdvance() {
      debugPrint('[OnboardingFlow] maybeAdvance: auth=$authDone mpcSession=$mpcSessionDone mpcProto=$mpcProtocolDone wallet=$walletDone anim=$animDone mounted=$mounted addr=$generatedAddress');
      if (!authDone || !mpcSessionDone || !mpcProtocolDone || !walletDone || !animDone || !mounted) return;
      if (generatedAddress == null) return;
      debugPrint('[OnboardingFlow] All conditions met, advancing to backup stage');
      try {
        CowalletApp.of(context).setWalletAddress(generatedAddress!);
      } catch (_) {}
      Future.delayed(const Duration(milliseconds: 400), () {
        if (mounted) _goTo(_Stage.backup);
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

  // ---- Biometric setup ----
  // The user reaches this screen AFTER DKG (the device shard is still only in
  // Rust memory — runDkg no longer auto-persists it). When the user opts into
  // biometric protection here, we persist the shard under the hardware-backed
  // auth-bound key, which triggers the biometric/device-credential prompt now —
  // as a result of the user's explicit choice, not automatically mid-DKG.
  Future<void> _startBioScan() async {
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
        if (mounted) _goTo(_Stage.name);
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _bioAuthenticating = false;
      });
    }
  }

  // ---- Name ----
  void _submitName() {
    final name = _nameCtrl.text.trim();
    if (name.isNotEmpty) {
      CowalletApp.of(context).setUserName(name);
    }
    _goTo(_Stage.ready);
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
          debugPrint('[OnboardingFlow] Loaded backup shard from pending storage');
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
      debugPrint('[OnboardingFlow] Deleted pending backup shard after successful backup');

      // Store SHA-256(backup_shard) on server for future force re-register verification.
      final digest = SHA256Digest();
      final hash = digest.process(Uint8List.fromList(backupBytes));
      final hashHex = convert.hex.encode(hash);
      final hashResult = await ShardsApi.storeBackupHash(backupShardHashHex: hashHex);
      if (hashResult.isSuccess) {
        debugPrint('[OnboardingFlow] Stored backup shard hash on server');
      } else {
        // Non-fatal: save hash locally for retry later
        await SecureStorage.save('pending_backup_hash', hashHex);
        debugPrint('[OnboardingFlow] Failed to store backup hash, saved locally for retry');
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
        if (mounted) _goTo(_Stage.bio);
      });
    } catch (e, st) {
      debugPrint('[OnboardingBackup] Error: $e');
      debugPrint('[OnboardingBackup] StackTrace: $st');
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
    setState(() => _backupSkipped = true);
    _goTo(_Stage.bio);
  }


  // ---- Persona ----
  void _pickPersona(String id) {
    setState(() => _selectedPersona = id);
    CowalletApp.of(context).setPersona(id);
    _finish();
  }

  void _skipPersona() => _finish();

  // ---- Finish ----
  Future<void> _finish() async {
    final appState = CowalletApp.of(context);
    appState.completeOnboarding();
    final addr = appState.walletAddress;
    final navigator = Navigator.of(context);

    // Clear persisted onboarding step
    await SecureStorage.delete(SecureStorage.keyOnboardingStep);

    // Persist onboarding metadata
    await SecureStorage.save('onboarding_completed_at', DateTime.now().toIso8601String());
    await SecureStorage.save('backup_status', _backupSkipped ? 'skipped' : (_backupDone ? 'saved' : 'pending'));
    await SecureStorage.save('mpc_address', addr);

    if (addr.isNotEmpty) {
      Services.balance.refresh(addr);
    }
    navigator.pushReplacementNamed('/');
  }

  // ======================= BUILD =======================

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(
        child: AnimatedSwitcher(
          duration: const Duration(milliseconds: 300),
          transitionBuilder: (child, animation) {
            return FadeTransition(
              opacity: animation,
              child: child,
            );
          },
          layoutBuilder: (currentChild, previousChildren) {
            return Stack(
              alignment: Alignment.topCenter,
              fit: StackFit.expand,
              children: [
                ...previousChildren,
                ?currentChild,
              ],
            );
          },
          child: _buildStage(),
        ),
      ),
    );
  }

  Widget _buildStage() {
    switch (_stage) {
      case _Stage.hero:
      case _Stage.intro:
        return _heroStage();
      case _Stage.email:
        return _emailStage();
      case _Stage.emailOtp:
        return _emailOtpStage();
      case _Stage.creating:
        return _creatingStage();
      case _Stage.bio:
        return _bioStage();
      case _Stage.name:
        return _nameStage();
      case _Stage.backup:
        return _backupStage();
      case _Stage.ready:
        return _readyStage();
      case _Stage.persona:
        return _personaStage();
    }
  }

  // ===================== SHARED WIDGETS =====================

  /// Top bar with optional back button and progress dots.
  Widget _topBar({bool showBack = false, int? step, int total = 3}) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
      child: Row(
        children: [
          if (showBack)
            GestureDetector(
              onTap: _goBack,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.arrow_back_ios_new,
                      size: 16, color: CwColors.ink3),
                  const SizedBox(width: 4),
                  Text(S.back,
                      style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 14, color: CwColors.ink3)),
                ],
              ),
            )
          else
            const SizedBox(width: 48),
          const Spacer(),
          if (step != null) _progressDots(step, total),
          const Spacer(),
          const SizedBox(width: 48),
        ],
      ),
    );
  }

  Widget _progressDots(int current, int total) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: List.generate(total, (i) {
        final isActive = i == current;
        final isDone = i < current;
        return Container(
          width: 8,
          height: 8,
          margin: const EdgeInsets.symmetric(horizontal: 4),
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: (isActive || isDone) ? CwColors.accent : CwColors.line,
          ),
        );
      }),
    );
  }

  Widget _heading(String text) {
    return Text(
      text,
      style: Theme.of(context).textTheme.displayMedium,
      textAlign: TextAlign.center,
    );
  }

  Widget _subtitle(String text) {
    return Text(
      text,
      style: Theme.of(context).textTheme.bodyLarge?.copyWith(
            color: CwColors.ink2,
          ),
      textAlign: TextAlign.center,
    );
  }

  Widget _primaryButton(String label, VoidCallback? onPressed) {
    return SizedBox(
      width: double.infinity,
      child: FilledButton(
        onPressed: onPressed,
        child: Text(label),
      ),
    );
  }

  Widget _secondaryLink(String label, VoidCallback onPressed) {
    return TextButton(
      onPressed: onPressed,
      child: Text(label, style: TextStyle(color: CwColors.ink3, fontSize: 14)),
    );
  }

  // ===================== STAGE 1+2: HERO + INTRO (PageView) =====================

  Widget _heroStage() {
    return Column(
      key: const ValueKey('hero'),
      children: [
        Expanded(
          child: PageView(
            controller: _pageCtrl,
            onPageChanged: (i) => setState(() => _guidePage = i),
            children: [
              _heroPage(),
              _introPageContent(),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(bottom: 32, left: 28, right: 28),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Page indicator dots
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: List.generate(2, (i) {
                  return AnimatedContainer(
                    duration: const Duration(milliseconds: 200),
                    width: i == _guidePage ? 20 : 8,
                    height: 8,
                    margin: const EdgeInsets.symmetric(horizontal: 4),
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(4),
                      color: i == _guidePage ? CwColors.accent : CwColors.line,
                    ),
                  );
                }),
              ),
              const SizedBox(height: 24),
              // CTA button
              _primaryButton(
                _guidePage == 0 ? S.getStarted : S.introStart,
                () {
                  if (_guidePage == 0) {
                    _pageCtrl.animateToPage(1,
                        duration: const Duration(milliseconds: 300),
                        curve: Curves.easeInOut);
                  } else {
                    _goTo(_Stage.email);
                  }
                },
              ),
              const SizedBox(height: 12),
              TextButton(
                onPressed: () => Navigator.pushNamed(context, '/recovery'),
                child: Text(
                  S.recoverWallet,
                  style: TextStyle(color: CwColors.ink3, fontSize: 14),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                S.heroLegal,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: CwColors.ink4,
                      fontSize: 11,
                    ),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _heroPage() {
    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 28),
      child: Column(
        children: [
          const SizedBox(height: 48),
          const CwOrb(size: 140, breathing: true),
          const SizedBox(height: 28),
          Text(
            S.heroKicker,
            style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: CwColors.ink3,
                  letterSpacing: 1.2,
                ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 12),
          RichText(
            textAlign: TextAlign.center,
            text: TextSpan(
              style: Theme.of(context).textTheme.displayLarge,
              children: [
                TextSpan(text: S.heroH1a),
                if (S.heroH1b.isNotEmpty)
                  TextSpan(
                    text: ' ${S.heroH1b} ',
                    style: Theme.of(context).textTheme.displayLarge,
                  ),
                TextSpan(
                  text: S.heroH1em,
                  style: Theme.of(context).textTheme.displayLarge?.copyWith(
                        fontStyle: FontStyle.italic,
                        color: CwColors.accent,
                      ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          Text(
            S.heroExplain,
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: CwColors.ink2,
                ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 32),
          _featureRow(Icons.touch_app_outlined, S.heroFeat1h, S.heroFeat1s),
          const SizedBox(height: 16),
          _featureRow(Icons.public, S.heroFeat2h, S.heroFeat2s),
          const SizedBox(height: 16),
          _featureRow(Icons.auto_awesome, S.heroFeat3h, S.heroFeat3s),
          const SizedBox(height: 24),
        ],
      ),
    );
  }

  Widget _introPageContent() {
    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 28),
      child: Column(
        children: [
          const SizedBox(height: 48),
          Icon(Icons.lock_outline, size: 64, color: CwColors.accent),
          const SizedBox(height: 24),
          _heading(S.introH1),
          const SizedBox(height: 12),
          _subtitle(S.introSub),
          const SizedBox(height: 32),
          _featureRow(Icons.call_split, S.introBullet1h, S.introBullet1s),
          const SizedBox(height: 16),
          _featureRow(Icons.verified_user_outlined, S.introBullet2h, S.introBullet2s),
          const SizedBox(height: 16),
          _featureRow(Icons.hide_source_outlined, S.introBullet3h, S.introBullet3s),
          const SizedBox(height: 24),
        ],
      ),
    );
  }

  Widget _featureRow(IconData icon, String title, String sub) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          width: 40,
          height: 40,
          decoration: BoxDecoration(
            color: CwColors.accentSoft,
            borderRadius: BorderRadius.circular(10),
          ),
          child: Icon(icon, size: 20, color: CwColors.accent),
        ),
        const SizedBox(width: 14),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title,
                  style: Theme.of(context)
                      .textTheme
                      .titleMedium
                      ?.copyWith(color: CwColors.ink1)),
              const SizedBox(height: 2),
              Text(sub,
                  style: Theme.of(context)
                      .textTheme
                      .bodySmall
                      ?.copyWith(color: CwColors.ink3)),
            ],
          ),
        ),
      ],
    );
  }

  // ===================== STAGE 2.5: EMAIL =====================

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
          _goTo(_Stage.emailOtp);
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
                  Navigator.pushNamed(context, AppRouter.recovery, arguments: _emailCtrl.text.trim());
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
    _backupShardHash = convert.hex.encode(hash);

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

    setState(() => _forceRegister = true);
    _goTo(_Stage.emailOtp);
  }

  // ===================== STAGE 2.6: EMAIL OTP =====================

  void _onOtpChanged(String value) {
    if (_otpError != null) setState(() => _otpError = null);
    if (value.length == 6) {
      _verifyEmailOtp();
    }
  }

  Future<void> _verifyEmailOtp() async {
    if (_stage == _Stage.creating || _otpVerifying) return;
    setState(() => _otpVerifying = true);

    try {
      final deviceId = await DeviceIdGenerator.getOrGenerate();
      var result = await AuthApi.register(
        deviceId: deviceId,
        email: _emailCtrl.text.trim(),
        otp: _otpCtrl.text.trim(),
        force: _forceRegister,
        backupShardHash: _backupShardHash,
      );

      // 428 on a force re-register means the server has no backup_shard_hash on
      // record for this account (e.g. registered before the hash mechanism, or
      // the hash was never uploaded). We already proved possession of the backup
      // shard by loading it in _verifyBackupShardForReRegister, so back-fill the
      // hash and retry once.
      if (!result.isSuccess &&
          result.errorCode == 428 &&
          _forceRegister &&
          _backupShardHash != null) {
        final backfill =
            await ShardsApi.storeBackupHash(backupShardHashHex: _backupShardHash!);
        if (backfill.isSuccess) {
          result = await AuthApi.register(
            deviceId: deviceId,
            email: _emailCtrl.text.trim(),
            otp: _otpCtrl.text.trim(),
            force: _forceRegister,
            backupShardHash: _backupShardHash,
          );
        }
      }
      if (!mounted) return;

      if (result.isSuccess) {
        setState(() => _otpVerifying = false);
        _goTo(_Stage.creating);
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

  Future<void> _resendOtp() async {
    _otpCtrl.clear();
    final turnstileToken = await TurnstileGate.getToken(context);
    if (turnstileToken == null) {
      if (!mounted) return;
      setState(() => _otpError = S.emailSendFailed);
      return;
    }
    final result = await AuthApi.sendEmailOtp(
        email: _emailCtrl.text.trim(), turnstileToken: turnstileToken);
    if (!mounted) return;
    if (!result.isSuccess) {
      setState(() => _otpError = S.emailSendFailed);
    }
  }

  Widget _emailOtpStage() {
    return SingleChildScrollView(
      key: const ValueKey('emailOtp'),
      child: Column(
        children: [
          _topBar(showBack: true, step: 0, total: 3),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                const SizedBox(height: 40),
                Icon(Icons.mark_email_read_outlined, size: 56, color: CwColors.accent),
                const SizedBox(height: 24),
                _heading(S.otpH1),
                const SizedBox(height: 8),
                _subtitle(S.otpSub(_emailCtrl.text.trim())),
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
                    style: TextStyle(
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
                    onChanged: _onOtpChanged,
                  ),
                ),
                const SizedBox(height: 24),
                if (_otpVerifying)
                  const CircularProgressIndicator()
                else
                  _secondaryLink(S.otpResend, _resendOtp),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _emailStage() {
    return SingleChildScrollView(
      key: const ValueKey('email'),
      child: Column(
        children: [
          _topBar(showBack: true, step: 0, total: 3),
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
                Center(child: _heading(S.emailH1)),
                const SizedBox(height: 8),
                Center(child: _subtitle(S.emailSub)),
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

  // ===================== STAGE 3: CREATING =====================

  Widget _creatingStage() {
    return SingleChildScrollView(
      key: const ValueKey('creating'),
      child: Column(
        children: [
          _topBar(step: 1, total: 3),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                const SizedBox(height: 24),
                const CwOrb(size: 120, thinking: true),
                const SizedBox(height: 28),
                _heading(_isResuming ? 'Resuming...' : S.creatingH1),
                const SizedBox(height: 8),
                _subtitle(_isResuming ? 'Recovering your wallet session' : S.creatingSub),
                const SizedBox(height: 32),
                // Progress bar
                ClipRRect(
                  borderRadius: BorderRadius.circular(6),
                  child: LinearProgressIndicator(
                    value: _createProgress,
                    minHeight: 8,
                    backgroundColor: CwColors.line,
                    valueColor:
                        AlwaysStoppedAnimation<Color>(CwColors.accent),
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
                  _primaryButton(S.retry, _startCreating),
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

  // ===================== STAGE 4: IMPORTING =====================


  // ===================== STAGE 5: BIO =====================

  Widget _bioStage() {
    return SingleChildScrollView(
      key: const ValueKey('bio'),
      child: Column(
        children: [
          _topBar(step: 2, total: 3),
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
                _heading(_bioDone ? S.bioDone : S.bioH1),
                const SizedBox(height: 8),
                _subtitle(S.bioSub),
                const SizedBox(height: 40),
                if (!_bioDone && !_bioAuthenticating) ...[
                  _primaryButton(S.bioActivate, _startBioScan),
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

  // ===================== STAGE 6: NAME =====================

  Widget _nameStage() {
    return SingleChildScrollView(
      key: const ValueKey('name'),
      child: Column(
        children: [
          _topBar(step: 2, total: 3),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SizedBox(height: 24),
                Center(child: _heading(S.nameH1)),
                const SizedBox(height: 28),
                // Text input
                Container(
                  decoration: BoxDecoration(
                    color: CwColors.bgCard,
                    borderRadius: BorderRadius.circular(14),
                    border: Border.all(color: CwColors.line),
                  ),
                  child: TextField(
                    controller: _nameCtrl,
                    textCapitalization: TextCapitalization.words,
                    style: TextStyle(
                      fontFamily: CwTypography.serifFamily,
                      fontSize: 20,
                      fontWeight: FontWeight.w500,
                      color: CwColors.ink1,
                    ),
                    textAlign: TextAlign.center,
                    decoration: InputDecoration(
                      hintText: S.namePlaceholder,
                      hintStyle: TextStyle(
                        fontFamily: CwTypography.serifFamily,
                        fontSize: 20,
                        fontWeight: FontWeight.w400,
                        color: CwColors.ink4,
                      ),
                      contentPadding: const EdgeInsets.symmetric(
                          horizontal: 16, vertical: 18),
                      border: InputBorder.none,
                    ),
                    onSubmitted: (_) => _submitName(),
                  ),
                ),
                const SizedBox(height: 10),
                // Hint
                Center(
                  child: Text(
                    S.nameHint,
                    style: Theme.of(context)
                        .textTheme
                        .bodySmall
                        ?.copyWith(color: CwColors.ink4),
                  ),
                ),
                const SizedBox(height: 32),
                _primaryButton(S.continueBtn, _submitName),
              ],
            ),
          ),
        ],
      ),
    );
  }

  // ===================== STAGE 7: BACKUP (store 3rd shard) =====================

  Widget _backupStage() {
    return SingleChildScrollView(
      key: const ValueKey('backup'),
      child: Column(
        children: [
          _topBar(step: 1, total: 3),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                const SizedBox(height: 24),
                Icon(Icons.shield_outlined, size: 64, color: CwColors.accent),
                const SizedBox(height: 24),
                _heading(S.backupH1),
                const SizedBox(height: 8),
                _subtitle(S.backupSub),
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
                    child: _secondaryLink(S.backupSkip, _skipBackup),
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

  // ===================== STAGE 9: READY =====================

  Widget _readyStage() {
    final name = CowalletApp.of(context).userName;
    final h1 = name.isNotEmpty ? S.readyH1Named(name) : S.readyH1;

    return SingleChildScrollView(
      key: const ValueKey('ready'),
      padding: const EdgeInsets.symmetric(horizontal: 28),
      child: Column(
        children: [
          const SizedBox(height: 48),
          // CwOrb with checkmark badge
          SizedBox(
            width: 140,
            height: 140,
            child: Stack(
              alignment: Alignment.center,
              children: [
                const CwOrb(size: 120, breathing: true),
                Positioned(
                  right: 8,
                  bottom: 8,
                  child: Container(
                    width: 36,
                    height: 36,
                    decoration: BoxDecoration(
                      color: CwColors.success,
                      shape: BoxShape.circle,
                      border: Border.all(color: CwColors.bgPaper, width: 3),
                    ),
                    child: const Icon(Icons.check, size: 20, color: Colors.white),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 28),
          _heading(h1),
          const SizedBox(height: 8),
          _subtitle(S.readySub),
          const SizedBox(height: 32),
          // "What you can do next" label
          Align(
            alignment: Alignment.centerLeft,
            child: Text(
              S.readyWhat,
              style: Theme.of(context)
                  .textTheme
                  .labelLarge
                  ?.copyWith(color: CwColors.ink3),
            ),
          ),
          const SizedBox(height: 16),
          // 3 numbered next-steps
          _numberedStep(1, S.ready1h, S.ready1s),
          const SizedBox(height: 12),
          _numberedStep(2, S.ready2h, S.ready2s),
          const SizedBox(height: 12),
          _numberedStep(3, S.ready3h, S.ready3s),
          const SizedBox(height: 36),
          _primaryButton(S.readyGo, () => _goTo(_Stage.persona)),
          const SizedBox(height: 24),
        ],
      ),
    );
  }

  Widget _numberedStep(int n, String title, String sub) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          width: 28,
          height: 28,
          decoration: BoxDecoration(
            color: CwColors.accentSoft,
            shape: BoxShape.circle,
          ),
          alignment: Alignment.center,
          child: Text(
            '$n',
            style: TextStyle(
              fontFamily: CwTypography.monoFamily,
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: CwColors.accent,
            ),
          ),
        ),
        const SizedBox(width: 14),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title,
                  style: Theme.of(context)
                      .textTheme
                      .titleMedium
                      ?.copyWith(color: CwColors.ink1)),
              const SizedBox(height: 2),
              Text(sub,
                  style: Theme.of(context)
                      .textTheme
                      .bodySmall
                      ?.copyWith(color: CwColors.ink3)),
            ],
          ),
        ),
      ],
    );
  }

  // ===================== STAGE 8: PERSONA =====================

  Widget _personaStage() {
    return SingleChildScrollView(
      key: const ValueKey('persona'),
      child: Column(
        children: [
          const SizedBox(height: 48),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Column(
              children: [
                _heading(S.personaH1),
                const SizedBox(height: 8),
                _subtitle(S.personaSub),
                const SizedBox(height: 28),
                _personaCard(
                  id: 'daily',
                  icon: Icons.wb_sunny_outlined,
                  title: S.personaDaily,
                  desc: S.personaDailyDesc,
                  tag: S.personaDailyTag,
                ),
                const SizedBox(height: 12),
                _personaCard(
                  id: 'trader',
                  icon: Icons.candlestick_chart,
                  title: S.personaTrader,
                  desc: S.personaTraderDesc,
                ),
                const SizedBox(height: 12),
                _personaCard(
                  id: 'family',
                  icon: Icons.people_outline,
                  title: S.personaFamily,
                  desc: S.personaFamilyDesc,
                  tag: S.personaFamilyTag,
                ),
                const SizedBox(height: 12),
                _personaCard(
                  id: 'builder',
                  icon: Icons.terminal,
                  title: S.personaBuilder,
                  desc: S.personaBuilderDesc,
                ),
                const SizedBox(height: 24),
                _secondaryLink(S.personaSkip, _skipPersona),
                const SizedBox(height: 24),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _personaCard({
    required String id,
    required IconData icon,
    required String title,
    required String desc,
    String? tag,
  }) {
    final selected = _selectedPersona == id;
    return GestureDetector(
      onTap: () => _pickPersona(id),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        width: double.infinity,
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: selected ? CwColors.accentSoft : CwColors.bgCard,
          borderRadius: BorderRadius.circular(16),
          border: Border.all(
            color: selected ? CwColors.accent : CwColors.line,
            width: selected ? 2 : 1,
          ),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: selected
                    ? CwColors.accent.withValues(alpha: 0.15)
                    : CwColors.accentSoft,
                borderRadius: BorderRadius.circular(10),
              ),
              child: Icon(icon, size: 20, color: CwColors.accent),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(title,
                            style: Theme.of(context)
                                .textTheme
                                .titleMedium
                                ?.copyWith(
                                    color: CwColors.ink1,
                                    fontWeight: FontWeight.w600)),
                      ),
                      if (tag != null)
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 2),
                          decoration: BoxDecoration(
                            color: CwColors.accentSoft,
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Text(tag,
                              style: TextStyle(
                                  fontSize: 11,
                                  color: CwColors.accent,
                                  fontWeight: FontWeight.w600)),
                        ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(desc,
                      style: Theme.of(context)
                          .textTheme
                          .bodyMedium
                          ?.copyWith(color: CwColors.ink3)),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
