import 'package:cowallet/theme/typography.dart';
import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../l10n/strings.dart';
import '../../widgets/section_label.dart';
import '../../widgets/cw_chip.dart';
import '../../widgets/top_toast.dart';
import '../../main.dart';
import '../../services/locator.dart';
import '../../router/app_router.dart';
import '../../config/api_config.dart';

class WalletView extends StatefulWidget {
  const WalletView({super.key});

  @override
  State<WalletView> createState() => _WalletViewState();
}

class _WalletViewState extends State<WalletView> {
  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    final tt = Theme.of(context).textTheme;

    return SafeArea(
      child: ListenableBuilder(
        listenable: Services.balance,
        builder: (context, _) => RefreshIndicator(
          onRefresh: () => Services.balance.refresh(
              CowalletApp.of(context).walletAddress,
          ),
          child: ListView(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            children: [
              const SizedBox(height: 16),

              // ── Portfolio Total ──
              _portfolioTotalCard(tt),
              const SizedBox(height: 20),

              // ── Action buttons ──
              _actionButtons(context),
              const SizedBox(height: 8),

              // ── Multi-chain assets ──
              SectionLabel(title: S.yourMoney),
              ..._buildChainSections(context),

              // ── Section: 证券代币 · 可选 ──
              SectionLabel(
                title: S.securities,
                trailingWidget: Padding(
                  padding: const EdgeInsets.only(left: 6),
                  child: CwChip(
                    label: S.securitiesNew,
                    variant: ChipVariant.amber,
                    fontSize: 10,
                  ),
                ),
              ),
              _securitiesCard(context, tt),

              // ── Section: 在赚利息的钱 ──
              SectionLabel(title: S.earning),
              _earningCard(context, tt),

              const SizedBox(height: 32),
            ],
          ),
        ),
      ),
    );
  }

  // ── Portfolio Total Card ──────────────────────────────────────────────────────────

  Widget _portfolioTotalCard(TextTheme tt) {
    final bal = Services.balance;
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: CwColors.line),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            S.totalBalance,
            style: TextStyle(
              fontFamily: CwTypography.monoFamily,
              fontSize: 11,
              fontWeight: FontWeight.w600,
              letterSpacing: 0.5,
              color: CwColors.ink3,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            bal.loading ? '...' : '\$${bal.portfolioTotalUsd}',
            style: TextStyle(
              fontFamily: CwTypography.monoFamily,
              fontSize: 34,
              fontWeight: FontWeight.w700,
              color: CwColors.ink1,
              letterSpacing: -0.5,
              height: 1.1,
            ),
          ),
          if (bal.error != null) ...[
            const SizedBox(height: 12),
            Text(
              bal.error!,
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w500,
                color: CwColors.danger,
              ),
            ),
          ] else if (bal.loading)
            Padding(
              padding: const EdgeInsets.only(top: 12),
              child: Text(
                'Loading...',
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                  color: CwColors.ink4,
                ),
              ),
            ),
        ],
      ),
    );
  }

  // ── Multi-chain sections ───────────────────────────────────────────────────────────

  List<Widget> _buildChainSections(BuildContext context) {
    final bal = Services.balance;
    if (bal.loading || bal.error != null) {
      return [
        Container(
          padding: const EdgeInsets.all(20),
          decoration: BoxDecoration(
            color: CwColors.bgCard,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: CwColors.line),
          ),
          child: Center(
            child: Text(
              bal.loading ? 'Loading chains...' : 'Pull to refresh',
              style: const TextStyle(
                fontSize: 13,
                color: CwColors.ink3,
              ),
            ),
          ),
        ),
        const SizedBox(height: 8),
      ];
    }

    final chainTotals = bal.chainTotals;
    if (chainTotals.isEmpty) {
      return [
        Container(
          padding: const EdgeInsets.all(20),
          decoration: BoxDecoration(
            color: CwColors.bgCard,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: CwColors.line),
          ),
          child: const Center(
            child: Text(
              'No assets found',
              style: TextStyle(
                fontSize: 13,
                color: CwColors.ink3,
              ),
            ),
          ),
        ),
        const SizedBox(height: 8),
      ];
    }

    final widgets = <Widget>[];
    for (final entry in chainTotals.entries) {
      final chainId = entry.key;
      final chainTotal = entry.value;
      final tokens = bal.tokensForChain(chainId);

      if (tokens.isEmpty) continue;

      widgets.add(_chainSection(context, chainId, chainTotal, tokens));
      widgets.add(const SizedBox(height: 8));
    }

    return widgets;
  }

  Widget _chainSection(BuildContext context, int chainId, String chainTotal, List tokens) {
    final chain = ChainConfig.byChainId(chainId)!;
    final chainColor = _chainColor(chain);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () => AppShell.goToChatAndSend(
        context,
        '查看${chain.displayName}上的资产',
      ),
      child: Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: CwColors.line),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Chain header: dot + name + total
          Row(
            children: [
              Container(
                width: 10,
                height: 10,
                decoration: BoxDecoration(
                  color: chainColor,
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 8),
              Text(
                chain.displayName,
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                  color: CwColors.ink1,
                ),
              ),
              const Spacer(),
              Text(
                '\$$chainTotal',
                style: TextStyle(
                  fontFamily: CwTypography.monoFamily,
                  fontSize: 15,
                  fontWeight: FontWeight.w700,
                  color: CwColors.ink1,
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          const Divider(height: 1),
          const SizedBox(height: 12),

          // Top tokens for this chain
          ...tokens.map((token) => _tokenRowInChain(context, token)),
        ],
      ),
    ),
    );
  }

  Widget _tokenRowInChain(BuildContext context, token) {
    final symbol = token.symbol as String;
    final balance = token.balance as String;
    final usd = token.usd as String;
    final logoUrl = token.logoUrl as String?;

    String emoji = '🪙';
    Color iconBg = CwColors.ink4.withValues(alpha: 0.1);
    if (symbol == 'ETH') {
      emoji = 'Ⓔ';
      iconBg = const Color(0xFF7B61FF).withValues(alpha: 0.12);
    }
    if (symbol == 'USDC') {
      emoji = 'Ⓤ';
      iconBg = CwColors.info.withValues(alpha: 0.12);
    }
    if (symbol == 'USDT') {
      emoji = 'Ⓣ';
      iconBg = CwColors.success.withValues(alpha: 0.12);
    }

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () => AppShell.goToChatAndSend(
        context,
        S.actionTokenInfo(symbol, balance, usd),
      ),
      child: Padding(
        padding: const EdgeInsets.only(bottom: 12),
        child: Row(
          children: [
            if (logoUrl != null && logoUrl.isNotEmpty)
              ClipRRect(
                borderRadius: BorderRadius.circular(8),
                child: Image.network(
                  logoUrl,
                  width: 32,
                  height: 32,
                  fit: BoxFit.cover,
                  errorBuilder: (_, __, ___) => _tokenFallbackIcon(emoji, iconBg),
                ),
              )
            else
              _tokenFallbackIcon(emoji, iconBg),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    symbol,
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Text(
                    balance,
                    style: TextStyle(
                      fontFamily: CwTypography.monoFamily,
                      fontSize: 11,
                      color: CwColors.ink3,
                    ),
                  ),
                ],
              ),
            ),
            Text(
              '\$$usd',
              style: TextStyle(
                fontFamily: CwTypography.monoFamily,
                fontSize: 14,
                fontWeight: FontWeight.w600,
                color: CwColors.ink1,
              ),
            ),
            const SizedBox(width: 4),
            const Icon(Icons.chevron_right, size: 16, color: CwColors.ink4),
          ],
        ),
      ),
    );
  }

  Widget _tokenFallbackIcon(String emoji, Color iconBg) {
    return Container(
      width: 32,
      height: 32,
      decoration: BoxDecoration(
        color: iconBg,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Center(
        child: Text(
          emoji,
          style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 16),
        ),
      ),
    );
  }

  static Color _chainColor(ChainConfig chain) {
    switch (chain.name) {
      case 'ethereum':
      case 'sepolia':
        return const Color(0xFF627EEA);
      case 'base':
      case 'base-sepolia':
        return const Color(0xFF0052FF);
      case 'arbitrum':
        return const Color(0xFF28A0F0);
      case 'optimism':
        return const Color(0xFFFF0420);
      case 'bsc':
        return const Color(0xFFF3BA2F);
      case 'polygon':
        return const Color(0xFF8247E5);
      default:
        return CwColors.ink3;
    }
  }

  // ── Action buttons ──────────────────────────────────────────────────────

  Widget _actionButtons(BuildContext context) {
    return Row(
      children: [
        _actionBtn(context, Icons.arrow_upward_rounded, S.send,
            () => AppShell.goToChatAndSend(context, S.actionSend)),
        const SizedBox(width: 10),
        _actionBtn(context, Icons.arrow_downward_rounded, S.receive,
            () => AppShell.goToChatAndSend(context, S.actionReceive)),
        const SizedBox(width: 10),
        _actionBtn(context, Icons.swap_horiz_rounded, S.swap,
            () => AppShell.goToChatAndSend(context, S.actionSwap)),
      ],
    );
  }

  Widget _actionBtn(BuildContext context, IconData icon, String label, VoidCallback onTap) {
    return Expanded(
      child: OutlinedButton.icon(
        onPressed: onTap,
        icon: Icon(icon, size: 18),
        label: Text(label, style: TextStyle(fontFamily: CwTypography.serifFamily, fontSize: 13)),
        style: OutlinedButton.styleFrom(
          foregroundColor: CwColors.ink1,
          side: const BorderSide(color: CwColors.lineStrong),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          padding: const EdgeInsets.symmetric(vertical: 10),
          minimumSize: Size.zero,
          tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        ),
      ),
    );
  }

  // ── Securities card ─────────────────────────────────────────────────────

  void _showComingSoonToast() {
    showTopToast(
      context,
      S.lang == Lang.zh ? '功能开发中，敬请期待' : 'Coming soon',
      backgroundColor: CwColors.ink3,
    );
  }

  Widget _securitiesCard(BuildContext context, TextTheme tt) {
    return GestureDetector(
      onTap: _showComingSoonToast,
      child: Container(
      decoration: BoxDecoration(
        color: CwColors.bgCard,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: CwColors.line),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Intro text
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 14, 14, 10),
            child: Text(
              S.securitiesIntro,
              style: tt.bodyMedium,
            ),
          ),

          // 3-column grid
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14),
            child: Row(
              children: [
                _securitiesItem('T-Bills', '5.20% APY', CwColors.info),
                const SizedBox(width: 8),
                _securitiesItem('AAPL', '\$224 +1.4%', CwColors.success),
                const SizedBox(width: 8),
                _securitiesItem('Gold', '\$92.4/g', CwColors.gold),
              ],
            ),
          ),

          // Footer link
          const Divider(height: 24, indent: 14, endIndent: 14),
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 0, 14, 14),
            child: Text(
              '${S.browseAll} →',
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w500,
                color: CwColors.accent,
              ),
            ),
          ),
        ],
      ),
      ),
    );
  }

  Widget _securitiesItem(String name, String detail, Color color) {
    return Expanded(
      child: Container(
        padding: const EdgeInsets.all(10),
        decoration: BoxDecoration(
          color: CwColors.bgSubtle.withValues(alpha: 0.5),
          borderRadius: BorderRadius.circular(10),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              name,
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w600,
                color: CwColors.ink1,
              ),
            ),
            const SizedBox(height: 3),
            Text(
              detail,
              style: TextStyle(
                fontFamily: CwTypography.monoFamily,
                fontSize: 11,
                fontWeight: FontWeight.w500,
                color: color,
              ),
            ),
          ],
        ),
      ),
    );
  }

  // ── Earning card ────────────────────────────────────────────────────────

  Widget _earningCard(BuildContext context, TextTheme tt) {
    return GestureDetector(
      onTap: _showComingSoonToast,
      child: Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: CwColors.successSoft.withValues(alpha: 0.45),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: CwColors.success.withValues(alpha: 0.18)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Protocol label row
          Row(
            children: [
              Text(
                'Aave 上的 USDC',
                style: tt.titleMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: CwColors.ink1,
                ),
              ),
              const Spacer(),
              const CwChip(
                label: '4.82%',
                variant: ChipVariant.green,
                fontSize: 12,
              ),
            ],
          ),
          const SizedBox(height: 4),

          // Chain + audit
          Text(
            'Base 链 · 审计过',
            style: tt.bodySmall?.copyWith(color: CwColors.ink3),
          ),
          const SizedBox(height: 10),

          // APY + earnings
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            decoration: BoxDecoration(
              color: CwColors.bgCard.withValues(alpha: 0.7),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'APY',
                        style: TextStyle(
                          fontFamily: CwTypography.monoFamily,
                          fontSize: 10,
                          fontWeight: FontWeight.w500,
                          color: CwColors.ink3,
                          letterSpacing: 0.5,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '4.82%',
                        style: TextStyle(
                          fontFamily: CwTypography.monoFamily,
                          fontSize: 20,
                          fontWeight: FontWeight.w700,
                          color: CwColors.success,
                        ),
                      ),
                    ],
                  ),
                ),
                Container(
                  width: 1,
                  height: 32,
                  color: CwColors.line,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '放了 \$10,000 · ${S.today}赚了 \$1.32',
                        style: TextStyle(
                          fontSize: 12,
                          fontWeight: FontWeight.w500,
                          color: CwColors.ink2,
                          height: 1.4,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
      ),
    );
  }

}
