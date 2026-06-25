// ---------------------------------------------------------------------------
// Threat detection
// ---------------------------------------------------------------------------

pub(super) fn detect_threat(message: &str) -> Option<&'static str> {
    let lower = message.to_lowercase();

    // Prompt injection
    if lower.contains("ignore previous instructions")
        || lower.contains("ignore all instructions")
        || lower.contains("你现在是")
        || lower.contains("from now on you are")
        || lower.contains("disregard your system prompt")
    {
        return Some("检测到 prompt injection 尝试。我不会执行此类请求。");
    }

    // Seed phrase / private key extraction
    if lower.contains("show seed")
        || lower.contains("export private key")
        || lower.contains("显示助记词")
        || lower.contains("导出私钥")
        || lower.contains("reveal mnemonic")
    {
        return Some("⚠️ 安全警告：私钥和助记词永远不会通过聊天暴露。CoWallet 使用 MPC 分片保护，没有任何单点可以导出完整密钥。");
    }

    // Phishing URLs
    let phishing_patterns = [
        "uniswap-claim", "airdrop-claim", "metamask-verify",
        "walletconnect-verify", "pancakeswap-airdrop",
    ];
    for pattern in phishing_patterns {
        if lower.contains(pattern) {
            return Some("⚠️ 安全警告：检测到疑似钓鱼链接。请勿点击不明链接或授权未知合约。正规协议不会通过聊天发送领取链接。");
        }
    }

    // Airdrop scams
    if (lower.contains("claim") || lower.contains("领取")) && (lower.contains("airdrop") || lower.contains("空投") || lower.contains("free token")) {
        return Some("⚠️ 注意：疑似空投骗局。正规空投不会要求你先发送代币或授权未知合约。请通过官方渠道验证。");
    }

    None
}

/// Detect if user message contains transfer/send intent keywords.
/// Used as safety net when AI fails to trigger send_transaction tool.
pub(super) fn has_transfer_intent(message: &str) -> bool {
    let lower = message.to_lowercase();
    let transfer_keywords = [
        "转", "发送", "打钱", "汇款", "付款", "send", "transfer",
        "打给", "转给", "转到", "转出", "发给", "付给",
        "全部转", "send all", "swap", "兑换", "换成", "换点",
    ];
    // Must also have some amount or address-like context, or be very explicit
    let explicit_intents = [
        "转账", "transfer", "send", "打钱", "汇款", "付款",
        "全部转出", "send all", "swap", "兑换",
    ];
    for kw in &explicit_intents {
        if lower.contains(kw) { return true; }
    }
    // "转/发送" + (amount or 0x address)
    let has_action = transfer_keywords.iter().any(|kw| lower.contains(kw));
    let has_target = lower.contains("0x")
        || lower.chars().any(|c| c.is_ascii_digit())
        || lower.contains("eth")
        || lower.contains("usdc")
        || lower.contains("usdt")
        || lower.contains("bnb")
        || lower.contains("pol");
    has_action && has_target
}
