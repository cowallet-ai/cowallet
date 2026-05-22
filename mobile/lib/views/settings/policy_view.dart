import 'package:cowallet/theme/typography.dart';
import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../l10n/strings.dart';
import '../../api/policy_api.dart';
import '../../widgets/top_toast.dart';
import '../../widgets/loading_overlay.dart';

class PolicyView extends StatefulWidget {
  const PolicyView({super.key});

  @override
  State<PolicyView> createState() => _PolicyViewState();
}

class _PolicyViewState extends State<PolicyView> {
  List<_PolicyItem> _policies = [];
  bool _loading = true;

  // Default thresholds for quick setup
  double _dailyLimit = 10000;
  double _singleLimit = 1000;

  @override
  void initState() {
    super.initState();
    _loadPolicies();
  }

  Future<void> _loadPolicies() async {
    setState(() => _loading = true);
    final result = await PolicyApi.getPolicies();
    if (result.isSuccess && result.data != null) {
      setState(() {
        _policies = (result.data as List).map((p) {
          final map = p as Map<String, dynamic>;
          final rules = map['rules'] is Map<String, dynamic>
              ? map['rules'] as Map<String, dynamic>
              : <String, dynamic>{};
          return _PolicyItem(
            id: map['id'] as String? ?? '',
            name: map['name'] as String? ?? '',
            description: map['description'] as String? ?? '',
            enabled: map['enabled'] as bool? ?? true,
            ruleType: rules['type'] as String? ?? '',
            rules: rules,
          );
        }).toList();

        // Sync thresholds from loaded policies
        for (final p in _policies) {
          if (p.ruleType == 'daily_limit') {
            _dailyLimit = (p.rules['max_daily_usd'] as num?)?.toDouble() ?? 10000;
          } else if (p.ruleType == 'amount_threshold') {
            _singleLimit = (p.rules['threshold_usd'] as num?)?.toDouble() ?? 1000;
          }
        }
      });
    }
    setState(() => _loading = false);
  }

  Future<void> _togglePolicy(String id, bool enabled) async {
    if (!mounted) return;
    LoadingOverlay.show(context);
    await PolicyApi.updatePolicy(policyId: id, enabled: enabled);
    LoadingOverlay.dismiss();
    _loadPolicies();
  }

  Future<void> _deletePolicy(String id) async {
    if (!mounted) return;
    LoadingOverlay.show(context);
    await PolicyApi.deletePolicy(id);
    LoadingOverlay.dismiss();
    _loadPolicies();
    if (mounted) {
      showTopToast(context, S.lang == Lang.zh ? '策略已删除' : 'Policy deleted',
          backgroundColor: CwColors.success);
    }
  }

  Future<void> _createDailyLimit() async {
    if (!mounted) return;
    LoadingOverlay.show(context);
    final template = PolicyApi.templateDailyLimit(
      name: S.lang == Lang.zh ? '每日限额' : 'Daily limit',
      description: S.lang == Lang.zh ? '每日累计转账不超过此金额' : 'Daily transfer cap',
      maxDailyUsd: _dailyLimit,
    );
    await PolicyApi.createPolicy(
      name: template['name'],
      description: template['description'],
      rules: template['rules'],
      action: template['action'],
      priority: template['priority'],
    );
    LoadingOverlay.dismiss();
    _loadPolicies();
    if (mounted) {
      showTopToast(context, S.lang == Lang.zh ? '每日限额已设置' : 'Daily limit set',
          backgroundColor: CwColors.success);
    }
  }

  Future<void> _createSingleLimit() async {
    if (!mounted) return;
    LoadingOverlay.show(context);
    final template = PolicyApi.templateLargeAmountConfirm(
      name: S.lang == Lang.zh ? '大额转账确认' : 'Large transfer confirm',
      description: S.lang == Lang.zh ? '单笔超过此金额需二次确认' : 'Confirm for large single transfers',
      thresholdUsd: _singleLimit,
    );
    await PolicyApi.createPolicy(
      name: template['name'],
      description: template['description'],
      rules: template['rules'],
      action: template['action'],
      priority: template['priority'],
    );
    LoadingOverlay.dismiss();
    _loadPolicies();
    if (mounted) {
      showTopToast(context, S.lang == Lang.zh ? '大额确认已设置' : 'Large transfer confirm set',
          backgroundColor: CwColors.success);
    }
  }

  @override
  Widget build(BuildContext context) {
    final isZh = S.lang == Lang.zh;
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      appBar: AppBar(
        title: Text(S.riskGuard),
        backgroundColor: CwColors.bgPaper,
        elevation: 0,
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.fromLTRB(20, 8, 20, 40),
              children: [
                // Quick setup section
                _sectionTitle(isZh ? '快速设置' : 'Quick Setup'),
                const SizedBox(height: 8),
                _quickSetupCard(
                  icon: Icons.today,
                  title: isZh ? '每日限额' : 'Daily Limit',
                  subtitle: isZh
                      ? '每日累计转账不超过 \$${_dailyLimit.toInt()}'
                      : 'Daily transfers capped at \$${_dailyLimit.toInt()}',
                  value: _dailyLimit,
                  onChanged: (v) => setState(() => _dailyLimit = v),
                  onSave: _createDailyLimit,
                  hasExisting: _policies.any((p) => p.ruleType == 'daily_limit'),
                ),
                const SizedBox(height: 10),
                _quickSetupCard(
                  icon: Icons.warning_amber_rounded,
                  title: isZh ? '大额确认' : 'Large Transfer Confirm',
                  subtitle: isZh
                      ? '单笔超过 \$${_singleLimit.toInt()} 需二次确认'
                      : 'Confirm transfers over \$${_singleLimit.toInt()}',
                  value: _singleLimit,
                  onChanged: (v) => setState(() => _singleLimit = v),
                  onSave: _createSingleLimit,
                  hasExisting: _policies.any((p) => p.ruleType == 'amount_threshold'),
                ),

                const SizedBox(height: 24),
                _sectionTitle(isZh ? '已启用策略' : 'Active Policies'),
                const SizedBox(height: 8),

                if (_policies.isEmpty)
                  Container(
                    padding: const EdgeInsets.all(20),
                    decoration: BoxDecoration(
                      color: CwColors.bgCard,
                      borderRadius: BorderRadius.circular(14),
                      border: Border.all(color: CwColors.line),
                    ),
                    child: Center(
                      child: Text(
                        isZh ? '暂无策略，请使用快速设置添加' : 'No policies. Use quick setup above.',
                        style: const TextStyle(color: CwColors.ink3, fontSize: 13),
                      ),
                    ),
                  )
                else
                  ..._policies.map(_buildPolicyCard),
              ],
            ),
    );
  }

  Widget _sectionTitle(String title) {
    return Text(
      title,
      style: TextStyle(
        fontFamily: CwTypography.serifFamily,
        fontSize: 14,
        fontWeight: FontWeight.w600,
        color: CwColors.ink1,
      ),
    );
  }

  Widget _quickSetupCard({
    required IconData icon,
    required String title,
    required String subtitle,
    required double value,
    required ValueChanged<double> onChanged,
    required VoidCallback onSave,
    required bool hasExisting,
  }) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: CwColors.line),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 32,
                height: 32,
                decoration: BoxDecoration(
                  color: CwColors.accentSoft,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(icon, size: 17, color: CwColors.accent),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: TextStyle(
                      fontFamily: CwTypography.serifFamily,
                      fontSize: 13.5,
                      fontWeight: FontWeight.w500,
                      color: CwColors.ink1,
                    )),
                    const SizedBox(height: 2),
                    Text(subtitle, style: const TextStyle(fontSize: 11, color: CwColors.ink3)),
                  ],
                ),
              ),
              if (hasExisting)
                const Icon(Icons.check_circle, size: 18, color: CwColors.success),
            ],
          ),
          const SizedBox(height: 12),
          SliderTheme(
            data: SliderThemeData(
              activeTrackColor: CwColors.accent,
              inactiveTrackColor: CwColors.line,
              thumbColor: CwColors.accent,
              overlayColor: CwColors.accent.withValues(alpha: 0.1),
            ),
            child: Slider(
              value: value,
              min: 100,
              max: 50000,
              divisions: 499,
              onChanged: onChanged,
            ),
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('\$${value.toInt()}',
                  style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: CwColors.ink1)),
              TextButton(
                onPressed: hasExisting ? null : onSave,
                child: Text(
                  hasExisting
                      ? (S.lang == Lang.zh ? '已设置' : 'Set')
                      : (S.lang == Lang.zh ? '启用' : 'Enable'),
                  style: TextStyle(
                    color: hasExisting ? CwColors.ink4 : CwColors.accent,
                    fontSize: 13,
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildPolicyCard(_PolicyItem policy) {
    String typeLabel;
    IconData typeIcon;
    switch (policy.ruleType) {
      case 'daily_limit':
        typeLabel = S.lang == Lang.zh ? '每日限额' : 'Daily Limit';
        typeIcon = Icons.today;
        break;
      case 'amount_threshold':
        typeLabel = S.lang == Lang.zh ? '大额确认' : 'Large Confirm';
        typeIcon = Icons.warning_amber_rounded;
        break;
      case 'whitelist':
        typeLabel = S.lang == Lang.zh ? '白名单' : 'Whitelist';
        typeIcon = Icons.verified_user;
        break;
      default:
        typeLabel = policy.ruleType;
        typeIcon = Icons.rule;
    }

    return Container(
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: CwColors.line),
      ),
      child: Row(
        children: [
          Container(
            width: 32,
            height: 32,
            decoration: BoxDecoration(
              color: policy.enabled ? CwColors.successSoft : CwColors.bgSubtle,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(typeIcon, size: 17,
                color: policy.enabled ? CwColors.success : CwColors.ink4),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(policy.name, style: const TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                  color: CwColors.ink1,
                )),
                const SizedBox(height: 2),
                Text(typeLabel, style: const TextStyle(fontSize: 11, color: CwColors.ink3)),
              ],
            ),
          ),
          Switch(
            value: policy.enabled,
            onChanged: (v) => _togglePolicy(policy.id, v),
            activeThumbColor: CwColors.accent,
          ),
          GestureDetector(
            onTap: () => _confirmDelete(policy),
            child: const Padding(
              padding: EdgeInsets.only(left: 4),
              child: Icon(Icons.delete_outline, size: 18, color: CwColors.ink4),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _confirmDelete(_PolicyItem policy) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(S.lang == Lang.zh ? '删除策略' : 'Delete Policy'),
        content: Text(S.lang == Lang.zh
            ? '确定删除"${policy.name}"？'
            : 'Delete "${policy.name}"?'),
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
      _deletePolicy(policy.id);
    }
  }
}

class _PolicyItem {
  final String id;
  final String name;
  final String description;
  final bool enabled;
  final String ruleType;
  final Map<String, dynamic> rules;

  _PolicyItem({
    required this.id,
    required this.name,
    required this.description,
    required this.enabled,
    required this.ruleType,
    required this.rules,
  });
}
