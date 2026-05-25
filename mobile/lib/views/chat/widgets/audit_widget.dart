import 'package:cowallet/theme/typography.dart';
import 'package:flutter/material.dart';
import '../../../theme/colors.dart';

class ChatAuditWidget extends StatelessWidget {
  final Map<String, dynamic> data;

  const ChatAuditWidget({super.key, required this.data});

  @override
  Widget build(BuildContext context) {
    final score = data['score'] as int? ?? 0;
    final riskLevel = data['risk_level'] as String? ?? 'unknown';
    final findings = (data['findings'] as List<dynamic>?) ?? [];
    final recommendations = (data['recommendations'] as List<dynamic>?) ?? [];
    final auditTime = data['audit_time'] as String?;

    final scoreColor = score >= 90
        ? CwColors.success
        : (score >= 70 ? const Color(0xFFE5A100) : CwColors.danger);

    // Separate findings by severity for grouped display
    final highFindings = findings.where((f) => _getSeverity(f) == 'high').toList();
    final mediumFindings = findings.where((f) => _getSeverity(f) == 'medium').toList();
    final infoFindings = findings.where((f) => _getSeverity(f) == 'info').toList();

    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: CwColors.line),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.shield_outlined, size: 16, color: CwColors.accent),
              const SizedBox(width: 6),
              const Text(
                '安全审计报告',
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  color: CwColors.ink3,
                  letterSpacing: 0.5,
                ),
              ),
              const Spacer(),
              if (auditTime != null)
                Text(
                  _formatTime(auditTime),
                  style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 10, color: CwColors.ink4),
                ),
            ],
          ),
          const SizedBox(height: 16),
          // Score circle
          Center(
            child: Column(
              children: [
                Container(
                  width: 72,
                  height: 72,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    border: Border.all(color: scoreColor, width: 3),
                  ),
                  child: Center(
                    child: Text(
                      '$score',
                      style: TextStyle(
                        fontSize: 28,
                        fontWeight: FontWeight.w700,
                        fontFamily: CwTypography.monoFamily,
                        color: scoreColor,
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  _riskLabel(riskLevel),
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w500,
                    color: scoreColor,
                  ),
                ),
              ],
            ),
          ),
          // High severity findings
          if (highFindings.isNotEmpty) ...[
            const SizedBox(height: 16),
            _sectionHeader('风险项', CwColors.danger),
            const SizedBox(height: 6),
            ...highFindings.map((f) => _buildFinding(f)),
          ],
          // Medium severity findings
          if (mediumFindings.isNotEmpty) ...[
            const SizedBox(height: 12),
            _sectionHeader('注意项', const Color(0xFFE5A100)),
            const SizedBox(height: 6),
            ...mediumFindings.map((f) => _buildFinding(f)),
          ],
          // Info findings (passed checks)
          if (infoFindings.isNotEmpty) ...[
            const SizedBox(height: 12),
            _sectionHeader('已通过', CwColors.success),
            const SizedBox(height: 6),
            ...infoFindings.map((f) => _buildFinding(f)),
          ],
          if (recommendations.isNotEmpty) ...[
            const SizedBox(height: 14),
            const Divider(height: 1, color: CwColors.line),
            const SizedBox(height: 12),
            const Text(
              '安全建议',
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 11, fontWeight: FontWeight.w600, color: CwColors.ink3),
            ),
            const SizedBox(height: 6),
            ...recommendations.map((r) => Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('•  ', style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 12, color: CwColors.ink3)),
                  Expanded(
                    child: Text(
                      r.toString(),
                      style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 12, color: CwColors.ink2, height: 1.4),
                    ),
                  ),
                ],
              ),
            )),
          ],
        ],
      ),
    );
  }

  Widget _sectionHeader(String title, Color color) {
    return Row(
      children: [
        Container(
          width: 3,
          height: 12,
          decoration: BoxDecoration(
            color: color,
            borderRadius: BorderRadius.circular(2),
          ),
        ),
        const SizedBox(width: 6),
        Text(
          title,
          style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 11, fontWeight: FontWeight.w600, color: color),
        ),
      ],
    );
  }

  Widget _buildFinding(dynamic finding) {
    final map = finding is Map<String, dynamic> ? finding : <String, dynamic>{};
    final severity = map['severity'] as String? ?? 'info';
    final message = map['message'] as String? ?? '';
    final type = map['type'] as String? ?? '';

    IconData icon;
    Color color;
    switch (severity) {
      case 'high':
        icon = _typeIcon(type, Icons.error);
        color = CwColors.danger;
        break;
      case 'medium':
        icon = _typeIcon(type, Icons.warning_amber_rounded);
        color = const Color(0xFFE5A100);
        break;
      default:
        icon = _typeIcon(type, Icons.check_circle_outline);
        color = CwColors.success;
    }

    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              message,
              style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 12, color: CwColors.ink2, height: 1.4),
            ),
          ),
        ],
      ),
    );
  }

  IconData _typeIcon(String type, IconData fallback) {
    switch (type) {
      case 'shard_complete':
      case 'shard_incomplete':
      case 'shard_unhealthy':
      case 'shard_stale':
        return Icons.key;
      case 'mpc_protection':
        return Icons.security;
      case 'transport_encryption':
        return Icons.lock_outline;
      case 'biometric_auth':
        return Icons.fingerprint;
      case 'presign_ready':
      case 'no_presignatures':
        return Icons.speed;
      case 'policies_active':
      case 'no_policies':
        return Icons.policy_outlined;
      case 'unlimited_approvals':
        return Icons.token;
      case 'failed_transactions':
      case 'large_transactions':
      case 'tx_frequency_spike':
        return Icons.receipt_long;
      case 'many_recipients':
        return Icons.people_outline;
      default:
        return fallback;
    }
  }

  String _getSeverity(dynamic finding) {
    if (finding is Map<String, dynamic>) {
      return finding['severity'] as String? ?? 'info';
    }
    return 'info';
  }

  String _riskLabel(String level) {
    switch (level) {
      case 'low':
        return '安全';
      case 'medium':
        return '中等风险';
      case 'high':
        return '高风险';
      default:
        return '未知';
    }
  }

  String _formatTime(String isoTime) {
    try {
      final dt = DateTime.parse(isoTime);
      return '${dt.month}/${dt.day} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {
      return '';
    }
  }
}
