// ---------------------------------------------------------------------------
// Threat detection
// ---------------------------------------------------------------------------

/// Normalize a message for prompt-injection matching. Lowercases, folds a set of
/// common unicode confusables / homoglyphs back to ASCII, strips zero-width and
/// combining characters used to break up keywords, and collapses runs of
/// whitespace/punctuation so "i g n o r e" or "ig-nore" still match "ignore".
fn normalize_for_detection(message: &str) -> String {
    let lower = message.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    for c in lower.chars() {
        let mapped = match c {
            // Zero-width / invisible separators frequently used to split keywords.
            '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}' | '\u{00ad}' => continue,
            // Cyrillic / Greek homoglyphs -> ASCII look-alikes.
            'а' => 'a', 'е' => 'e', 'о' => 'o', 'р' => 'p', 'с' => 'c',
            'х' => 'x', 'у' => 'y', 'к' => 'k', 'і' => 'i', 'ѕ' => 's',
            'ο' => 'o', 'α' => 'a', 'ρ' => 'p', 'ν' => 'v', 'ѐ' => 'e',
            // Fullwidth latin -> ASCII.
            'ａ'..='ｚ' => ((c as u32 - 'ａ' as u32) as u8 + b'a') as char,
            other => other,
        };
        out.push(mapped);
    }
    // Collapse separators (spaces, hyphens, dots, underscores, asterisks) so
    // keywords broken up by punctuation/whitespace are matched. This intentionally
    // removes them entirely, producing a compact form for substring checks.
    out.chars()
        .filter(|c| !matches!(c, ' ' | '\t' | '-' | '_' | '.' | '*' | '/' | '\\' | '|' | '=' | '+'))
        .collect()
}

pub(super) fn detect_threat(message: &str) -> Option<&'static str> {
    let lower = message.to_lowercase();
    // Compact, confusable-folded form for robust substring matching.
    let norm = normalize_for_detection(message);

    // Prompt injection — check both the readable lowercase form and the
    // separator-stripped/confusable-folded compact form.
    let injection_patterns_spaced = [
        "ignore previous instructions",
        "ignore all instructions",
        "ignore the above",
        "ignore your instructions",
        "disregard your system prompt",
        "disregard previous instructions",
        "disregard all previous",
        "from now on you are",
        "you are now",
        "pretend to be",
        "new instructions",
        "override your",
        "developer mode",
        "jailbreak",
        "你现在是",
        "忽略之前的指令",
        "忽略以上",
        "忽略所有指令",
        "忽略系统提示",
        "从现在开始你是",
        "扮演",
    ];
    let injection_patterns_compact = [
        "ignorepreviousinstructions",
        "ignoreallinstructions",
        "ignoretheabove",
        "ignoreyourinstructions",
        "disregardyoursystemprompt",
        "disregardpreviousinstructions",
        "fromnowonyouare",
        "youarenow",
        "pretendtobe",
        "newinstructions",
        "overrideyour",
        "developermode",
        "jailbreak",
        "你现在是",
        "忽略之前的指令",
        "忽略所有指令",
        "忽略系统提示",
        "从现在开始你是",
    ];
    if injection_patterns_spaced.iter().any(|p| lower.contains(p))
        || injection_patterns_compact.iter().any(|p| norm.contains(p))
    {
        return Some("检测到 prompt injection 尝试。我不会执行此类请求。");
    }

    // Seed phrase / private key extraction
    let secret_patterns_compact = [
        "showseed",
        "showseedphrase",
        "exportprivatekey",
        "revealmnemonic",
        "revealseed",
        "whatismyprivatekey",
        "dumpprivatekey",
        "显示助记词",
        "导出私钥",
        "泄露私钥",
        "私钥是什么",
    ];
    if secret_patterns_compact.iter().any(|p| norm.contains(p)) {
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
