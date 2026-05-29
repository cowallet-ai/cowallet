import 'package:cowallet/theme/typography.dart';
import 'package:flutter/material.dart';
import '../../../theme/colors.dart';
import '../../../l10n/strings.dart';

class ChatSwapConfirmWidget extends StatelessWidget {
  final String fromToken;
  final String toToken;
  final String amount;
  final String estimatedOutput;
  final double slippage;
  final bool loading;
  final bool resolved;
  final VoidCallback? onConfirm;
  final VoidCallback? onDeny;
  final int? chainId;
  final int? toChainId;
  final int? estimatedTime;

  const ChatSwapConfirmWidget({
    super.key,
    required this.fromToken,
    required this.toToken,
    required this.amount,
    required this.estimatedOutput,
    this.slippage = 0.5,
    this.loading = false,
    this.resolved = false,
    this.onConfirm,
    this.onDeny,
    this.chainId,
    this.toChainId,
    this.estimatedTime,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: resolved ? CwColors.line : CwColors.accent.withValues(alpha: 0.4),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                resolved ? Icons.check_circle : Icons.swap_horiz,
                size: 16,
                color: resolved ? CwColors.success : CwColors.accent,
              ),
              const SizedBox(width: 6),
              Text(
                resolved ? S.swapSubmitted : S.swapConfirm,
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  color: resolved ? CwColors.success : CwColors.accent,
                  letterSpacing: 0.5,
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          // Swap visualization
          Row(
            children: [
              Expanded(
                child: _tokenBox(fromToken, amount, S.pay),
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8),
                child: Icon(
                  Icons.arrow_forward_rounded,
                  size: 20,
                  color: CwColors.ink3,
                ),
              ),
              Expanded(
                child: _tokenBox(toToken, estimatedOutput, S.estimatedReceive),
              ),
            ],
          ),
          const SizedBox(height: 12),
          _infoRow(S.slippageTolerance, '${slippage}%'),
          if (chainId != null && toChainId != null && toChainId != chainId)
            _infoRow(S.network, '${_chainName(chainId!)} → ${_chainName(toChainId!)}')
          else if (chainId != null)
            _infoRow(S.network, _chainName(chainId!)),
          if (estimatedTime != null && estimatedTime! > 0)
            _infoRow(S.estimatedArrival, _formatDuration(estimatedTime!)),
          _infoRow(S.route, '$fromToken → $toToken'),
          if (!resolved) ...[
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: SizedBox(
                    height: 48,
                    child: OutlinedButton(
                      onPressed: loading ? null : onDeny,
                      style: OutlinedButton.styleFrom(
                        foregroundColor: CwColors.ink3,
                        side: const BorderSide(color: CwColors.line),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(10),
                        ),
                      ),
                      child: Text(S.cancel),
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: SizedBox(
                    height: 48,
                    child: ElevatedButton(
                      onPressed: loading ? null : onConfirm,
                      style: ElevatedButton.styleFrom(
                        backgroundColor: CwColors.accent,
                        foregroundColor: Colors.white,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(10),
                        ),
                      ),
                      child: loading
                          ? const SizedBox(
                              width: 16,
                              height: 16,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: Colors.white,
                              ),
                            )
                          : Text(S.confirmSwap),
                    ),
                  ),
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }

  Widget _tokenBox(String token, String amount, String label) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: CwColors.bgPaper,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: CwColors.line),
      ),
      child: Column(
        children: [
          Text(
            label,
            style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 10, color: CwColors.ink4),
          ),
          const SizedBox(height: 4),
          Text(
            amount,
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              fontFamily: CwTypography.monoFamily,
              color: CwColors.ink1,
            ),
          ),
          Text(
            token,
            style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 12, color: CwColors.ink3),
          ),
        ],
      ),
    );
  }

  Widget _infoRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        children: [
          Text(label, style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 12, color: CwColors.ink4)),
          const Spacer(),
          Text(
            value,
            style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 12, color: CwColors.ink2),
          ),
        ],
      ),
    );
  }

  String _chainName(int chainId) {
    switch (chainId) {
      case 1: return 'Ethereum';
      case 8453: return 'Base';
      case 42161: return 'Arbitrum';
      case 10: return 'Optimism';
      case 56: return 'BNB Chain';
      case 137: return 'Polygon';
      default: return 'Chain $chainId';
    }
  }

  String _formatDuration(int minutes) {
    if (minutes < 60) return S.estArrivalMinutes(minutes);
    final h = minutes ~/ 60;
    final m = minutes % 60;
    return m == 0 ? S.estArrivalHours(h) : S.estArrivalHoursMinutes(h, m);
  }
}
