import 'package:cowallet/theme/typography.dart';
import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../l10n/s.dart';
import '../../widgets/cw_chip.dart';
import '../../widgets/section_label.dart';
import '../../widgets/top_toast.dart';
import '../../widgets/loading_overlay.dart';
import '../../main.dart';
import '../../services/locator.dart';
import '../../services/settings_service.dart';
import '../../widgets/pin_verify_dialog.dart';
import '../../services/key_health_service.dart';
import '../../utils/secure_storage.dart';
import '../../api/wallet_api.dart';

class SettingsView extends StatefulWidget {
  const SettingsView({super.key});

  @override
  State<SettingsView> createState() => _SettingsViewState();
}

class _SettingsViewState extends State<SettingsView> {
  bool _biometricEnabled = false;
  bool _biometricAvailable = false;
  bool _hasEnrolledBiometrics = false;
  String _biometricType = 'Biometric';
  String? _lastRotationDate;

  KeyStatus _phoneStatus = KeyStatus.unknown;
  KeyStatus _serverStatus = KeyStatus.unknown;
  KeyStatus _backupStatus = KeyStatus.unknown;

  SettingsService get _settings => Services.settings;

  @override
  void initState() {
    super.initState();
    _loadBiometricStatus();
    _loadKeySecuritySettings();
    _loadKeyHealth();
    _settings.addListener(_onSettingsChanged);
  }

  @override
  void dispose() {
    _settings.removeListener(_onSettingsChanged);
    super.dispose();
  }

  void _onSettingsChanged() {
    if (mounted) setState(() {});
  }


  Future<void> _loadKeyHealth() async {
    final addr = await SecureStorage.get('mpc_address');
    final suffix = (addr != null && addr.length >= 10) ? addr.toLowerCase().substring(0, 10) : 'unknown';
    final phoneStr = await SecureStorage.get('key_phone_status_$suffix');
    final serverStr = await SecureStorage.get('key_server_status_$suffix');
    final backupStr = await SecureStorage.get('key_backup_status_$suffix');
    final lastCheckedStr = await SecureStorage.get('key_backup_last_checked_$suffix');

    final expired = _isExpired(lastCheckedStr);

    if (mounted) {
      setState(() {
        _phoneStatus = _parseStatus(phoneStr);
        _serverStatus = _parseStatus(serverStr);
        _backupStatus = expired ? KeyStatus.warning : _parseStatus(backupStr);
      });
    }
  }

  bool _isExpired(String? lastCheckedStr) {
    if (lastCheckedStr == null) return true;
    final lastChecked = DateTime.tryParse(lastCheckedStr);
    if (lastChecked == null) return true;
    return DateTime.now().difference(lastChecked).inDays >= KeyHealthService.verifyExpiryDays;
  }

  KeyStatus _parseStatus(String? value) {
    if (value == null) return KeyStatus.unknown;
    return KeyStatus.values.where((e) => e.name == value).firstOrNull ?? KeyStatus.unknown;
  }

  Future<void> _loadBiometricStatus() async {
    final available = await Services.biometrics.isAvailable();
    final enabled = await Services.biometrics.isEnabled();
    final hasEnrolled = await Services.biometrics.hasEnrolledBiometrics();
    final bioType = await Services.biometrics.getPrimaryBiometricType();
    print('[Settings] biometric available=$available, enabled=$enabled, hasEnrolled=$hasEnrolled, type=$bioType');

    if (mounted) {
      setState(() {
        _biometricAvailable = available;
        _biometricEnabled = enabled;
        _hasEnrolledBiometrics = hasEnrolled;
        _biometricType = bioType;
      });
    }
  }

  String _getBiometricSubtitle() {
    if (!_biometricAvailable) {
      return S.biometricNotAvailable;
    }
    if (!_hasEnrolledBiometrics) {
      return 'Please set up $_biometricType in your device settings first';
    }
    return 'Use $_biometricType to verify sensitive operations';
  }

  Future<void> _toggleBiometric(bool value) async {
    if (!_biometricAvailable) return;

    if (value) {
      final ctx = Services.navigatorKey.currentContext;
      if (ctx == null) return;
      final pinOk = await PinVerifyDialog.show(ctx, reason: S.biometricAuthReason);
      if (!pinOk) return;

      final bioOk = await Services.biometrics.authenticate(
        reason: S.biometricAuthReason,
      );
      if (!bioOk) return;
    } else {
      final authenticated = await Services.authenticate(reason: S.biometricAuthReason);
      if (!authenticated) return;
    }

    if (!mounted) return;
    LoadingOverlay.show(context);
    await Services.biometrics.setEnabled(value);
    LoadingOverlay.dismiss();
    if (mounted) {
      setState(() => _biometricEnabled = value);
    }
  }

  Future<void> _toggleEmergencyFreeze() async {
    if (_settings.emergencyFreezeActive) {
      // Deactivating — require auth first
      final authed = await Services.authenticate(reason: S.biometricAuthReason);
      if (!authed) return;

      if (!mounted) return;
      LoadingOverlay.show(context);
      final address = await SecureStorage.get('mpc_address');
      if (address != null) {
        await WalletApi.unfreezeWallet(address);
      }
      await _settings.setEmergencyFreezeActive(false);
      LoadingOverlay.dismiss();
      if (mounted) {
        showTopToast(context, S.emergencyFreezeDeactivated, backgroundColor: CwColors.success);
      }
    } else {
      // Activating — show confirmation dialog
      final confirmed = await showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: Text(S.emergencyFreezeConfirmTitle),
          content: Text(S.emergencyFreezeConfirmBody),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: Text(S.cancel),
            ),
            TextButton(
              onPressed: () => Navigator.pop(ctx, true),
              style: TextButton.styleFrom(foregroundColor: CwColors.danger),
              child: Text(S.confirm),
            ),
          ],
        ),
      );
      if (confirmed == true) {
        if (!mounted) return;
        LoadingOverlay.show(context);
        final address = await SecureStorage.get('mpc_address');
        if (address != null) {
          await WalletApi.freezeWallet(address);
        }
        await _settings.setEmergencyFreezeActive(true);
        LoadingOverlay.dismiss();
        if (mounted) {
          showTopToast(context, S.emergencyFreezeActivated, backgroundColor: CwColors.danger);
        }
      }
    }
  }

  void _toggleLanguage() {
    final locale = Localizations.localeOf(context);
    final newLang = locale.languageCode == 'zh' ? 'en' : 'zh';
    CowalletApp.setLocale(context, Locale(newLang));
  }


  void _toggleVoiceInput() {
    _settings.setVoiceInputEnabled(!_settings.voiceInputEnabled);
  }

  void _toggleWeeklyReport() {
    if (!_settings.weeklyReportEnabled) {
      showTopToast(
        context,
        S.comingSoon,
        backgroundColor: CwColors.ink3,
      );
      return;
    }
    _settings.setWeeklyReportEnabled(!_settings.weeklyReportEnabled);
  }

  Future<void> _handleResetOnboarding() async {
    final balanceService = Services.balance;
    final address = CowalletApp.of(context).walletAddress;

    // Show loading while refreshing balance
    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (_) => AlertDialog(
        content: Row(
          children: [
            const SizedBox(
              width: 20, height: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
            const SizedBox(width: 16),
            Text(S.resetWalletChecking),
          ],
        ),
      ),
    );

    await balanceService.refresh(address);
    if (!mounted) return;
    Navigator.pop(context); // dismiss loading

    final totalUsd = double.tryParse(balanceService.portfolioTotalUsd) ?? 0.0;

    if (totalUsd > 0) {
      // Wallet has assets — warn user to transfer first
      if (!mounted) return;
      showDialog(
        context: context,
        builder: (ctx) => AlertDialog(
          title: Text(S.resetWalletTitle),
          content: Text(S.resetWalletHasBalance),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: Text(S.cancel),
            ),
            FilledButton(
              onPressed: () {
                Navigator.pop(ctx);
                Navigator.pushNamedAndRemoveUntil(
                    context, '/', (_) => false);
              },
              child: Text(S.resetWalletGoTransfer),
            ),
          ],
        ),
      );
      return;
    }

    // Balance is zero — show final confirmation
    if (!mounted) return;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(S.resetWalletConfirmTitle),
        content: Text(S.resetWalletConfirmBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(S.cancel),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: CwColors.danger),
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(S.resetWalletConfirm),
          ),
        ],
      ),
    );

    if (confirmed == true && mounted) {
      CowalletApp.of(context).resetOnboarding();
      Navigator.pushNamedAndRemoveUntil(
          context, '/onboarding', (_) => false);
    }
  }

  Future<void> _loadKeySecuritySettings() async {
    final lastRotation = await SecureStorage.get('last_key_rotation');

    if (mounted) {
      setState(() {
        _lastRotationDate = lastRotation;
      });
    }
  }



  String _formatLastRotation() {
    if (_lastRotationDate == null) return S.never;

    final locale = Localizations.localeOf(context);
    final isZh = locale.languageCode == 'zh';

    try {
      final date = DateTime.parse(_lastRotationDate!);
      final now = DateTime.now();
      final diff = now.difference(date);

      if (diff.inDays == 0) {
        return S.today;
      } else if (diff.inDays == 1) {
        return S.yesterday;
      } else if (diff.inDays < 30) {
        return S.daysAgo(diff.inDays);
      } else {
        final months = (diff.inDays / 30).floor();
        return S.monthsAgo(months);
      }
    } catch (e) {
      return S.never;
    }
  }

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: ListView(
        padding: const EdgeInsets.fromLTRB(20, 8, 20, 40),
        children: [
          // Emergency freeze banner
          if (_settings.emergencyFreezeActive)
            Container(
              margin: const EdgeInsets.only(bottom: 10),
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
              decoration: BoxDecoration(
                color: CwColors.danger.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: CwColors.danger.withValues(alpha: 0.4)),
              ),
              child: Row(
                children: [
                  const Icon(Icons.ac_unit, size: 18, color: CwColors.danger),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      S.frozenBanner,
                      style: const TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                        color: CwColors.danger,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          // Header
          Padding(
            padding: const EdgeInsets.only(top: 8, bottom: 4),
            child: Text(S.settings,
                style: Theme.of(context).textTheme.titleLarge),
          ),

          // ── Section: 安全 ──
          SectionLabel(title: S.security),
          _keysCard(context),
          const SizedBox(height: 10),
          _securityList(context),

          // ── Section: 密钥安全 ──
          SectionLabel(title: S.keySecurity),
          _keySecurityList(context),

          // ── Section: 对话 ──
          SectionLabel(title: S.conversation),
          _conversationList(context),

          // ── Section: 一般 ──
          SectionLabel(title: S.general),
          _generalList(context),

          // ── Signoff ──
          const SizedBox(height: 28),
          Center(
            child: Text(
              S.signoff1,
              style: TextStyle(
                fontFamily: CwTypography.monoFamily,
                fontSize: 10,
                color: CwColors.ink4,
              ),
            ),
          ),
          const SizedBox(height: 2),
          Center(
            child: Text(
              S.signoff2('1.0.0'),
              style: TextStyle(
                fontFamily: CwTypography.monoFamily,
                fontSize: 10,
                color: CwColors.ink4,
              ),
            ),
          ),
        ],
      ),
    );
  }

  // ── Keys health card ──
  Widget _keysCard(BuildContext context) {
    return GestureDetector(
      onTap: () => Navigator.pushNamed(context, '/keys').then((_) => _loadKeyHealth()),
      child: Container(
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: CwColors.bgCard,
          borderRadius: BorderRadius.circular(18),
          border: Border.all(color: CwColors.line),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Top row: title + chip
            Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        S.keysCheckup,
                        style: TextStyle(
                          fontFamily: CwTypography.serifFamily,
                          fontSize: 13.5,
                          fontWeight: FontWeight.w600,
                          color: CwColors.ink1,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        S.keysCheckupSub,
                        style: const TextStyle(
                          fontSize: 11,
                          color: CwColors.ink3,
                        ),
                      ),
                    ],
                  ),
                ),
                CwChip(
                  label: _keysChipLabel(),
                  variant: _keysChipVariant(),
                  showDot: true,
                ),
              ],
            ),
            const SizedBox(height: 14),
            // 3-column grid
            Row(
              children: [
                _keyIndicator(
                  icon: Icons.phone_iphone,
                  label: S.onPhone,
                  color: _statusColor(_phoneStatus),
                  bgColor: _statusBgColor(_phoneStatus),
                ),
                const SizedBox(width: 10),
                _keyIndicator(
                  icon: Icons.cloud_outlined,
                  label: S.inCloud,
                  color: _statusColor(_serverStatus),
                  bgColor: _statusBgColor(_serverStatus),
                ),
                const SizedBox(width: 10),
                _keyIndicator(
                  icon: Icons.lock_outline,
                  label: S.recovery,
                  color: _statusColor(_backupStatus),
                  bgColor: _statusBgColor(_backupStatus),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Color _statusColor(KeyStatus status) {
    switch (status) {
      case KeyStatus.ok: return CwColors.success;
      case KeyStatus.warning: return CwColors.warn;
      case KeyStatus.error: return CwColors.danger;
      case KeyStatus.unknown: return CwColors.ink3;
    }
  }

  Color _statusBgColor(KeyStatus status) {
    switch (status) {
      case KeyStatus.ok: return CwColors.successSoft;
      case KeyStatus.warning: return CwColors.warnSoft;
      case KeyStatus.error: return CwColors.dangerSoft;
      case KeyStatus.unknown: return CwColors.bgSubtle;
    }
  }

  String _keysChipLabel() {
    if (_phoneStatus == KeyStatus.ok && _serverStatus == KeyStatus.ok && _backupStatus == KeyStatus.ok) {
      return S.allSafe;
    }
    if (_phoneStatus == KeyStatus.error || _serverStatus == KeyStatus.error || _backupStatus == KeyStatus.error) {
      return S.keyStatusError;
    }
    return S.keyStatusWarning;
  }

  ChipVariant _keysChipVariant() {
    if (_phoneStatus == KeyStatus.ok && _serverStatus == KeyStatus.ok && _backupStatus == KeyStatus.ok) {
      return ChipVariant.green;
    }
    if (_phoneStatus == KeyStatus.error || _serverStatus == KeyStatus.error || _backupStatus == KeyStatus.error) {
      return ChipVariant.danger;
    }
    return ChipVariant.amber;
  }

  Widget _keyIndicator({
    required IconData icon,
    required String label,
    required Color color,
    required Color bgColor,
  }) {
    return Expanded(
      child: Column(
        children: [
          Container(
            width: 30,
            height: 30,
            decoration: BoxDecoration(
              color: bgColor,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(icon, size: 16, color: color),
          ),
          const SizedBox(height: 4),
          Text(
            label,
            style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 11, color: CwColors.ink3),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }

  // ── Security settings list ──
  Widget _securityList(BuildContext context) {
    return _settingsContainer(
      children: [
        _settingRow(
          context,
          icon: Icons.fingerprint,
          iconColor: CwColors.accent,
          iconBg: CwColors.accentSoft,
          title: S.biometricAuth,
          subtitle: _getBiometricSubtitle(),
          trailing: Switch(
            value: _biometricEnabled,
            onChanged: _biometricAvailable ? _toggleBiometric : null,
            activeThumbColor: CwColors.accent,
          ),
        ),
        const Divider(indent: 52, height: 1),
        _settingRow(
          context,
          icon: Icons.error_outline,
          iconColor: CwColors.danger,
          iconBg: CwColors.dangerSoft,
          title: S.emergencyFreeze,
          subtitle: S.emergencyFreezeSub,
          trailing: Switch(
            value: _settings.emergencyFreezeActive,
            onChanged: (_) => _toggleEmergencyFreeze(),
            activeTrackColor: CwColors.danger.withValues(alpha: 0.5),
            activeThumbColor: CwColors.danger,
          ),
          onTap: _toggleEmergencyFreeze,
        ),
        const Divider(indent: 52, height: 1),
        _settingRow(
          context,
          icon: Icons.people_outline,
          iconColor: CwColors.warn,
          iconBg: CwColors.warnSoft,
          title: S.emergencyContact,
          subtitle: S.emergencyContactSub,
        ),
        const Divider(indent: 52, height: 1),
        _settingRow(
          context,
          icon: Icons.shield_outlined,
          iconColor: CwColors.info,
          iconBg: CwColors.infoSoft,
          title: S.riskGuard,
          subtitle: S.riskGuardSub,
          trailing: const Icon(Icons.chevron_right, size: 18, color: CwColors.ink4),
          onTap: () => Navigator.pushNamed(context, '/policy'),
        ),
      ],
    );
  }

  // ── Conversation settings list ──
  Widget _conversationList(BuildContext context) {
    return _settingsContainer(
      children: [
        _settingRow(
          context,
          icon: Icons.mic_none,
          iconColor: CwColors.ink3,
          iconBg: CwColors.bgSubtle,
          title: S.voiceInput,
          subtitle: S.voiceInputSub,
          trailing: CwChip(
            label: _settings.voiceInputEnabled ? S.on : S.off,
            variant: _settings.voiceInputEnabled
                ? ChipVariant.green
                : ChipVariant.neutral,
          ),
          onTap: _toggleVoiceInput,
        ),
        const Divider(indent: 52, height: 1),
        _settingRow(
          context,
          icon: Icons.auto_awesome_outlined,
          iconColor: CwColors.ink3,
          iconBg: CwColors.bgSubtle,
          title: S.aiModel,
          subtitle: S.aiModelSub,
          trailing: const Icon(Icons.chevron_right,
              size: 18, color: CwColors.ink4),
        ),
      ],
    );
  }

  // ── General settings list ──
  Widget _generalList(BuildContext context) {
    return _settingsContainer(
      children: [
        _settingRow(
          context,
          icon: Icons.language,
          iconColor: CwColors.ink3,
          iconBg: CwColors.bgSubtle,
          title: S.language,
          trailing: Text(
            S.languageLabel,
            style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 11, color: CwColors.ink3),
          ),
          onTap: _toggleLanguage,
        ),
        const Divider(indent: 52, height: 1),
        _settingRow(
          context,
          icon: Icons.bar_chart_rounded,
          iconColor: CwColors.ink3,
          iconBg: CwColors.bgSubtle,
          title: S.weeklyReport,
          subtitle: S.weeklyReportSub,
          trailing: Switch(
            value: _settings.weeklyReportEnabled,
            onChanged: (_) => _toggleWeeklyReport(),
            activeThumbColor: CwColors.accent,
          ),
        ),
        const Divider(indent: 52, height: 1),
        _settingRow(
          context,
          icon: Icons.restart_alt,
          iconColor: CwColors.ink3,
          iconBg: CwColors.bgSubtle,
          title: S.redoOnboarding,
          subtitle: S.redoOnboardingSub,
          trailing: Text(
            '↻',
            style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 16, color: CwColors.ink3),
          ),
          onTap: () => _handleResetOnboarding(),
        ),
      ],
    );
  }

  // ── Shared container for setting groups ──
  Widget _settingsContainer({required List<Widget> children}) {
    return Container(
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: CwColors.line),
      ),
      child: Column(children: children),
    );
  }

  // ── Key Security settings list ──
  Widget _keySecurityList(BuildContext context) {
    return _settingsContainer(
      children: [
        _settingRow(
          context,
          icon: Icons.autorenew,
          iconColor: CwColors.accent,
          iconBg: CwColors.accentSoft,
          title: S.rotateKeyShares,
          subtitle: '${S.lastRotation}: ${_formatLastRotation()}',
          trailing: const Icon(Icons.chevron_right, size: 18, color: CwColors.ink4),
          onTap: () => showTopToast(
            context,
            S.comingSoon,
            backgroundColor: CwColors.ink3,
          ),
        ),
        const Divider(indent: 52, height: 1),
        _settingRow(
          context,
          icon: Icons.bolt_outlined,
          iconColor: CwColors.accent,
          iconBg: CwColors.accentSoft,
          title: S.presignatures,
          subtitle: S.presignaturesSub,
          trailing: Text(
            '5',
            style: TextStyle(
              fontFamily: CwTypography.monoFamily,
              fontSize: 15,
              fontWeight: FontWeight.w700,
              color: CwColors.success,
            ),
          ),
          onTap: () => showTopToast(
            context,
            S.comingSoon,
            backgroundColor: CwColors.ink3,
          ),
        ),
      ],
    );
  }

  // ── Setting row ──
  Widget _settingRow(
    BuildContext context, {
    required IconData icon,
    required Color iconColor,
    required Color iconBg,
    required String title,
    String? subtitle,
    Widget? trailing,
    VoidCallback? onTap,
  }) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap ?? () {},
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
        child: Row(
          children: [
            // Leading icon
            Container(
              width: 32,
              height: 32,
              decoration: BoxDecoration(
                color: iconBg,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Icon(icon, size: 17, color: iconColor),
            ),
            const SizedBox(width: 10),
            // Title + subtitle
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    style: TextStyle(
                      fontFamily: CwTypography.serifFamily,
                      fontSize: 13.5,
                      fontWeight: FontWeight.w500,
                      color: CwColors.ink1,
                    ),
                  ),
                  if (subtitle != null) ...[
                    const SizedBox(height: 1),
                    Text(
                      subtitle,
                      style: const TextStyle(
                        fontSize: 11,
                        color: CwColors.ink3,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            // Trailing
            ?trailing,
          ],
        ),
      ),
    );
  }
}
