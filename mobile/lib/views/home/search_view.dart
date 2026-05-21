import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../l10n/strings.dart';
import '../../main.dart';
import '../../services/locator.dart';
import '../../services/balance_service.dart';
import '../../services/chain_service.dart';
import '../../router/app_router.dart';

class SearchView extends StatefulWidget {
  const SearchView({super.key});

  @override
  State<SearchView> createState() => _SearchViewState();
}

class _SearchViewState extends State<SearchView> {
  final _controller = TextEditingController();
  final _focusNode = FocusNode();
  String _query = '';

  @override
  void initState() {
    super.initState();
    _focusNode.requestFocus();
  }

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _onChanged(String value) {
    setState(() => _query = value.trim().toLowerCase());
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: CwColors.bgPaper,
      body: SafeArea(
        child: Column(
          children: [
            _buildSearchBar(),
            Expanded(
              child: _query.isEmpty ? _buildQuickActions() : _buildResults(),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSearchBar() {
    return Container(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
      decoration: const BoxDecoration(
        border: Border(bottom: BorderSide(color: CwColors.line)),
      ),
      child: Row(
        children: [
          Expanded(
            child: Container(
              height: 40,
              decoration: BoxDecoration(
                color: CwColors.bgSubtle,
                borderRadius: BorderRadius.circular(10),
              ),
              child: TextField(
                controller: _controller,
                focusNode: _focusNode,
                onChanged: _onChanged,
                style: const TextStyle(fontSize: 14, color: CwColors.ink1),
                decoration: InputDecoration(
                  hintText: S.searchHint,
                  hintStyle: const TextStyle(fontSize: 14, color: CwColors.ink3),
                  prefixIcon: const Icon(Icons.search, size: 20, color: CwColors.ink3),
                  border: InputBorder.none,
                  contentPadding: const EdgeInsets.symmetric(vertical: 10),
                ),
              ),
            ),
          ),
          const SizedBox(width: 12),
          GestureDetector(
            onTap: () => Navigator.pop(context),
            child: Text(
              S.cancel,
              style: const TextStyle(fontSize: 14, color: CwColors.accent),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildQuickActions() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        _sectionTitle(S.features),
        const SizedBox(height: 8),
        ..._allFeatures().map((f) => _featureRow(f)),
        const SizedBox(height: 24),
        _sectionTitle(S.myAssets),
        const SizedBox(height: 8),
        ..._buildAssetRows(),
      ],
    );
  }

  Widget _buildResults() {
    final features = _allFeatures().where((f) =>
        f.label.toLowerCase().contains(_query) ||
        f.keywords.any((k) => k.contains(_query))).toList();

    final assets = _getAssets().where((a) =>
        a.symbol.toLowerCase().contains(_query) ||
        (a.contractAddress ?? '').toLowerCase().contains(_query)).toList();

    final txs = _getMatchingTxs();

    if (features.isEmpty && assets.isEmpty && txs.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.search_off, size: 48, color: CwColors.ink3),
            const SizedBox(height: 12),
            Text(S.noResults, style: const TextStyle(color: CwColors.ink3)),
          ],
        ),
      );
    }

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        if (features.isNotEmpty) ...[
          _sectionTitle(S.features),
          const SizedBox(height: 8),
          ...features.map((f) => _featureRow(f)),
          const SizedBox(height: 20),
        ],
        if (assets.isNotEmpty) ...[
          _sectionTitle(S.myAssets),
          const SizedBox(height: 8),
          ...assets.map((a) => _assetRow(a)),
          const SizedBox(height: 20),
        ],
        if (txs.isNotEmpty) ...[
          _sectionTitle(S.recentTx),
          const SizedBox(height: 8),
          ...txs.map((tx) => _txRow(tx)),
        ],
      ],
    );
  }

  // -- Features ---------------------------------------------------------------

  List<_FeatureItem> _allFeatures() => [
        _FeatureItem(
          icon: Icons.arrow_upward,
          color: CwColors.accent,
          label: S.sendTitle,
          keywords: ['转账', '发送', 'send', 'transfer'],
          action: () => _goChat(S.sendTitle),
        ),
        _FeatureItem(
          icon: Icons.arrow_downward,
          color: CwColors.success,
          label: S.receive,
          keywords: ['收款', '收钱', 'receive', '二维码', 'qr'],
          action: () => _goChat(S.receive),
        ),
        _FeatureItem(
          icon: Icons.swap_horiz,
          color: const Color(0xFF7C3AED),
          label: S.swap,
          keywords: ['兑换', '换币', 'swap', 'exchange'],
          action: () => _goChat(S.swap),
        ),
        _FeatureItem(
          icon: Icons.account_balance_wallet,
          color: const Color(0xFF0EA5E9),
          label: S.totalBalance,
          keywords: ['余额', '资产', 'balance', 'portfolio'],
          action: () => _goChat('查看余额'),
        ),
        _FeatureItem(
          icon: Icons.history,
          color: CwColors.ink2,
          label: S.recentTx,
          keywords: ['交易', '历史', 'history', 'transaction', '记录'],
          action: () => _goChat('最近交易记录'),
        ),
        _FeatureItem(
          icon: Icons.security,
          color: const Color(0xFFF59E0B),
          label: S.securityAudit,
          keywords: ['安全', '审计', 'security', 'audit'],
          action: () => _goChat('安全审计'),
        ),
      ];

  Widget _featureRow(_FeatureItem f) {
    return InkWell(
      onTap: f.action,
      borderRadius: BorderRadius.circular(10),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 10, horizontal: 4),
        child: Row(
          children: [
            Container(
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: f.color.withOpacity(0.1),
                borderRadius: BorderRadius.circular(10),
              ),
              child: Icon(f.icon, color: f.color, size: 18),
            ),
            const SizedBox(width: 12),
            Text(
              f.label,
              style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: CwColors.ink1),
            ),
            const Spacer(),
            const Icon(Icons.chevron_right, size: 18, color: CwColors.ink3),
          ],
        ),
      ),
    );
  }

  // -- Assets -----------------------------------------------------------------

  List<TokenBalance> _getAssets() {
    return Services.balance.allTokens;
  }

  List<Widget> _buildAssetRows() {
    final assets = _getAssets();
    if (assets.isEmpty) {
      return [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 12),
          child: Text(S.noAssets, style: const TextStyle(color: CwColors.ink3, fontSize: 13)),
        ),
      ];
    }
    return assets.take(10).map((a) => _assetRow(a)).toList();
  }

  Widget _assetRow(TokenBalance token) {
    final chainName = token.chainId != null ? ChainConfig.byId(token.chainId!).name : '';
    return InkWell(
      onTap: () => _goChat('${token.symbol} $chainName'),
      borderRadius: BorderRadius.circular(10),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 10, horizontal: 4),
        child: Row(
          children: [
            Container(
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: CwColors.bgSubtle,
                borderRadius: BorderRadius.circular(18),
              ),
              child: Center(
                child: Text(
                  token.symbol.substring(0, token.symbol.length > 3 ? 3 : token.symbol.length),
                  style: const TextStyle(fontSize: 11, fontWeight: FontWeight.w700, color: CwColors.ink2),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    token.symbol,
                    style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: CwColors.ink1),
                  ),
                  if (chainName.isNotEmpty)
                    Text(
                      chainName,
                      style: const TextStyle(fontSize: 11, color: CwColors.ink3),
                    ),
                ],
              ),
            ),
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                Text(
                  token.balance,
                  style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: CwColors.ink1),
                ),
                Text(
                  '\$${token.usd}',
                  style: const TextStyle(fontSize: 11, color: CwColors.ink3),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  // -- Transactions -----------------------------------------------------------

  List<Map<String, dynamic>> _getMatchingTxs() {
    // Search from home view's cached transactions via services
    // For now, return empty — txs are loaded in home_view state
    return [];
  }

  Widget _txRow(Map<String, dynamic> tx) {
    final hash = tx['tx_hash'] as String? ?? '';
    final shortHash = hash.length > 14
        ? '${hash.substring(0, 8)}...${hash.substring(hash.length - 4)}'
        : hash;
    final tokenSymbol = tx['token_symbol'] as String? ?? '';
    return InkWell(
      onTap: () {
        Navigator.pop(context);
        AppShell.goToChatAndShowTx(context, tx);
      },
      borderRadius: BorderRadius.circular(10),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 10, horizontal: 4),
        child: Row(
          children: [
            const Icon(Icons.receipt_long, size: 18, color: CwColors.ink3),
            const SizedBox(width: 12),
            Text(shortHash, style: const TextStyle(fontSize: 13, color: CwColors.ink2)),
            if (tokenSymbol.isNotEmpty) ...[
              const SizedBox(width: 8),
              Text(tokenSymbol, style: const TextStyle(fontSize: 12, color: CwColors.ink3)),
            ],
          ],
        ),
      ),
    );
  }

  // -- Helpers ----------------------------------------------------------------

  void _goChat(String message) {
    Navigator.pop(context);
    AppShell.goToChatAndSend(context, message);
  }

  Widget _sectionTitle(String text) {
    return Text(
      text,
      style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: CwColors.ink3),
    );
  }
}

class _FeatureItem {
  final IconData icon;
  final Color color;
  final String label;
  final List<String> keywords;
  final VoidCallback action;

  _FeatureItem({
    required this.icon,
    required this.color,
    required this.label,
    required this.keywords,
    required this.action,
  });
}
