import 'package:flutter/material.dart';
import 'app_localizations.dart';

/// Language enum
enum Lang { zh, en }

/// Localization wrapper with the same API as the old S class
/// Uses Flutter's official localization system under the hood
class S {
  S._();

  static AppLocalizations? _localizations;

  /// Get AppLocalizations from context
  static AppLocalizations of(BuildContext context) {
    _localizations = AppLocalizations.of(context);
    return _localizations!;
  }

  /// Get current language
  static Lang get lang {
    final localeName = _localizations?.localeName ?? 'zh';
    return localeName == 'zh' ? Lang.zh : Lang.en;
  }

  /// Internal helper for backward compatibility (fallback when localizations not available)
  static String _p(String zh, String en) => lang == Lang.zh ? zh : en;

  // App
  static String get appName => _localizations?.appName ?? 'CoWallet';
  static String get tagline => _localizations?.tagline ?? _p('会听懂人话的钱包', 'the wallet that reads you back');

  // Tabs
  static String get tabHome => _localizations?.tabHome ?? _p('首页', 'Home');
  static String get tabWallet => _localizations?.tabWallet ?? _p('钱包', 'Wallet');
  static String get tabAsk => _localizations?.tabAsk ?? _p('问', 'ASK');
  static String get tabAgents => _localizations?.tabAgents ?? _p('助手', 'Agents');
  static String get tabSettings => _localizations?.tabSettings ?? _p('设置', 'Settings');

  // Onboarding — Hero
  static String get heroKicker => _localizations?.heroKicker ?? _p('数字钱包 · 会听懂人话', 'Digital wallet · speaks your language');
  static String get heroH1a => _localizations?.heroH1a ?? _p('会听你说话的', 'A wallet that');
  static String get heroH1b => _localizations?.heroH1b ?? _p('', 'actually');
  static String get heroH1em => _localizations?.heroH1em ?? _p('钱包', 'listens');
  static String get heroExplain => _localizations?.heroExplain ?? _p(
    '就像给你家请了个管家——你说"帮我转 100 块给小明",它就去做;你不会说也没关系,它有按钮。',
    'Like hiring a butler for your money — say "send \$100 to Sarah" and it does it. Don\'t feel like talking? Buttons work too.',
  );
  static String get heroFeat1h => _localizations?.heroFeat1h ?? _p('不用懂区块链', 'No crypto knowledge needed');
  static String get heroFeat1s => _localizations?.heroFeat1s ?? _p('说句话就能转账、收款、理财', 'Send, receive, and earn just by saying so');
  static String get heroFeat2h => _localizations?.heroFeat2h ?? _p('100+ 个金融网络', '100+ financial networks');
  static String get heroFeat2s => _localizations?.heroFeat2s ?? _p('全世界通用', 'Works worldwide');
  static String get heroFeat3h => _localizations?.heroFeat3h ?? _p('AI 帮你跑腿', 'AI does the errands');
  static String get heroFeat3s => _localizations?.heroFeat3s ?? _p('你只需说一句话', 'Just say the word');
  static String get getStarted => _localizations?.getStarted ?? _p('开始使用', 'Get started');
  static String get heroLegal => _localizations?.heroLegal ?? _p('继续即表示同意服务条款与隐私政策', 'By continuing you agree to our Terms and Privacy Policy');

  // Onboarding — Intro
  static String get introH1 => _localizations?.introH1 ?? _p('你的钱包如何保护你', 'How your wallet protects you');
  static String get introSub => _localizations?.introSub ?? _p('CoWallet 用一种叫"门限签名"的技术,把钥匙拆成三份。', 'CoWallet uses threshold signatures to split your key into three pieces.');
  static String get introBullet1h => _localizations?.introBullet1h ?? _p('钥匙拆成三份', 'Key split into three');
  static String get introBullet1s => _localizations?.introBullet1s ?? _p('手机一份、服务器一份、你自己保管一份。完整钥匙从不出现在任何地方。', 'One on your phone, one on server, one kept by you. The full key never exists anywhere.');
  static String get introBullet2h => _localizations?.introBullet2h ?? _p('动钱需要两份', 'Two needed to transact');
  static String get introBullet2s => _localizations?.introBullet2s ?? _p('任何单方(包括 CoWallet)都无法单独动你的钱。', 'No single party — including CoWallet — can move your money alone.');
  static String get introBullet3h => _localizations?.introBullet3h ?? _p('没有助记词', 'No seed phrase');
  static String get introBullet3s => _localizations?.introBullet3s ?? _p('不用抄 12 个单词。丢了手机,用你的备份 + 服务器就能恢复。', 'No 12 words to write down. Lose your phone, your backup + server recovers everything.');
  static String get introStart => _localizations?.introStart ?? _p('开始创建', 'Start creating');

  // Onboarding — Email
  static String get emailH1 => _localizations?.emailH1 ?? _p('绑定恢复邮箱', 'Recovery Email');
  static String get emailSub => _localizations?.emailSub ?? _p('用于账户恢复时验证身份,我们不会发送垃圾邮件。', 'Used to verify your identity during wallet recovery. We won\'t send spam.');
  static String get emailHint => _localizations?.emailHint ?? _p('此邮箱仅用于钱包恢复验证', 'This email is only used for wallet recovery verification');
  static String get invalidEmail => _localizations?.invalidEmail ?? _p('请输入有效的邮箱地址', 'Please enter a valid email address');
  static String get emailSendFailed => _localizations?.emailSendFailed ?? _p('发送验证码失败,请重试', 'Failed to send code, please try again');
  static String get emailAlreadyRegistered => _localizations?.emailAlreadyRegistered ?? _p('该邮箱已注册', 'Email already registered');
  static String get emailAlreadyRegisteredDesc => _localizations?.emailAlreadyRegisteredDesc ?? _p('该邮箱已关联钱包,是否前往恢复流程?', 'This email is linked to an existing wallet. Go to recovery?');
  static String get goRecovery => _localizations?.goRecovery ?? _p('去恢复', 'Recover');
  static String get reRegister => _localizations?.reRegister ?? _p('重新注册', 'Re-register');
  static String get reRegisterDesc => _localizations?.reRegisterDesc ?? _p('将创建新钱包,原钱包资产需通过恢复流程找回', 'This will create a new wallet. Original assets can only be recovered via the recovery flow.');
  static String get reRegisterConfirm => _localizations?.reRegisterConfirm ?? _p('确认重新注册', 'Confirm Re-register');

  // Onboarding — Email OTP
  static String get otpH1 => _localizations?.otpH1 ?? _p('输入验证码', 'Enter Verification Code');
  static String otpSub(String email) => _localizations?.otpSub(email) ?? _p('验证码已发送至 $email', 'Code sent to $email');
  static String get otpResend => _localizations?.otpResend ?? _p('重新发送验证码', 'Resend code');
  static String get otpInvalid => _localizations?.otpInvalid ?? _p('验证码错误或已过期', 'Invalid or expired code');

  // Onboarding — Creating
  static String get creatingH1 => _localizations?.creatingH1 ?? _p('正在帮你把钥匙分成三份', 'Splitting your key into three pieces');
  static String get creatingSub => _localizations?.creatingSub ?? _p('动你的钱需要任意两份钥匙。三份分开存放,丢了一份还能恢复。完整的钥匙从不出现在任何地方。', 'Moving your money requires any 2 of 3 keys. Stored separately — lose one, the other two still work. The full key never exists in one place.');
  static String get cl1 => _localizations?.cl1 ?? _p('第 1 份:存在这台手机里', '1st key: stored on this phone');
  static String get cl2 => _localizations?.cl2 ?? _p('第 2 份:存在服务器保险柜', '2nd key: stored in server vault');
  static String get cl3 => _localizations?.cl3 ?? _p('第 3 份:由你自己保管', '3rd key: kept by you');
  static String get createError => _localizations?.createError ?? _p('钱包创建失败,请重试。', 'Wallet creation failed. Please try again.');
  static String get retry => _localizations?.retry ?? _p('重试', 'Retry');

  // Onboarding — Bio
  static String get bioH1 => _localizations?.bioH1 ?? _p('开启生物识别', 'Enable biometric authentication');
  static String get bioSub => _localizations?.bioSub ?? _p('就像手机解锁一样,用指纹或面容保护你的钱包。生物信息不会离开这台手机。', 'Just like unlocking your phone. Protect your wallet with fingerprint or face. Biometric data never leaves this device.');
  static String get bioActivate => _localizations?.bioActivate ?? _p('开启生物识别', 'Turn on biometrics');
  static String get bioSkip => _localizations?.bioSkip ?? _p('改用密码', 'Use a passcode instead');
  static String get bioVerifying => _localizations?.bioVerifying ?? _p('正在验证...', 'Verifying...');
  static String get bioDone => _localizations?.bioDone ?? _p('生物识别已开启', 'Biometrics ready');

  // Onboarding — PIN
  static String get pinH1 => _localizations?.pinH1 ?? _p('设置钱包密码', 'Set wallet passcode');
  static String get pinSub => _localizations?.pinSub ?? _p('6 位数字密码,每次交易时需要输入。', '6-digit passcode, required for every transaction.');
  static String get pinConfirmH1 => _localizations?.pinConfirmH1 ?? _p('再输入一次', 'Confirm passcode');
  static String get pinConfirmSub => _localizations?.pinConfirmSub ?? _p('请再输入一遍以确认。', 'Enter the same passcode again to confirm.');
  static String get pinMismatch => _localizations?.pinMismatch ?? _p('两次输入不一致,请重新设置', 'Passcodes don\'t match. Try again.');
  static String get pinDone => _localizations?.pinDone ?? _p('密码已设置', 'Passcode set');

  // Onboarding — Name
  static String get nameH1 => _localizations?.nameH1 ?? _p('我该怎么叫你?', 'What should I call you?');
  static String get nameSub => _localizations?.nameSub ?? _p('起个名字就行,不用真名。', 'A nickname works. No real name needed.');
  static String get namePlaceholder => _localizations?.namePlaceholder ?? _p('比如 小明 / 老王 / Alice', 'e.g. Alice, Mike, or a nickname');
  static String get nameTooShort => _localizations?.nameTooShort ?? _p('名字太短', 'Name too short');
  static String get nameTooLong => _localizations?.nameTooLong ?? _p('名字太长', 'Name too long');

  // Settings
  static String get settings => _localizations?.settings ?? _p('设置', 'Settings');
  static String get security => _localizations?.security ?? _p('安全', 'Security');
  static String get keySecurity => _localizations?.keySecurity ?? _p('密钥安全', 'Key Security');
  static String get conversation => _localizations?.conversation ?? _p('对话', 'Conversation');
  static String get general => _localizations?.general ?? _p('一般', 'General');

  static String get biometricAuth => _localizations?.biometricAuth ?? _p('生物识别', 'Biometric Authentication');
  static String get biometricAuthReason => _localizations?.biometricAuthReason ?? _p('请验证身份以继续', 'Authenticate to proceed');
  static String get biometricNotAvailable => _localizations?.biometricNotAvailable ?? _p('此设备不支持', 'Not available on this device');
  static String get biometricEnable => _localizations?.biometricEnable ?? _p('开启', 'Enable');
  static String get biometricDisable => _localizations?.biometricDisable ?? _p('关闭', 'Disable');
  static String get emergencyFreeze => _localizations?.emergencyFreeze ?? _p('紧急冻结', 'Emergency Freeze');
  static String get emergencyFreezeSub => _localizations?.emergencyFreezeSub ?? _p('暂时冻结所有交易', 'Temporarily freeze all transactions');
  static String get emergencyFreezeConfirmTitle => _localizations?.emergencyFreezeConfirmTitle ?? _p('冻结钱包?', 'Freeze Wallet?');
  static String get emergencyFreezeConfirmBody => _localizations?.emergencyFreezeConfirmBody ?? _p('所有交易将被阻止,直到你解冻。', 'All transactions will be blocked until you unfreeze.');
  static String get emergencyFreezeActivated => _localizations?.emergencyFreezeActivated ?? _p('钱包已冻结', 'Wallet frozen');
  static String get emergencyFreezeDeactivated => _localizations?.emergencyFreezeDeactivated ?? _p('钱包已解冻', 'Wallet unfrozen');
  static String get frozenBanner => _localizations?.frozenBanner ?? _p('钱包已冻结,点「安全」可解冻', 'Wallet is frozen. Tap Security to unfreeze.');
  static String get emergencyContact => _localizations?.emergencyContact ?? _p('紧急联系人', 'Emergency Contact');
  static String get emergencyContactSub => _localizations?.emergencyContactSub ?? _p('设置可信联系人用于恢复', 'Set up trusted contacts for recovery');
  static String get riskGuard => _localizations?.riskGuard ?? _p('风险防护', 'Risk Guard');
  static String get riskGuardSub => _localizations?.riskGuardSub ?? _p('自定义安全规则和限额', 'Custom security rules and limits');

  static String get keysCheckup => _localizations?.keysCheckup ?? _p('密钥检查', 'Keys Check-up');
  static String get keysCheckupSub => _localizations?.keysCheckupSub ?? _p('检查密钥分片的健康状态', 'Check the health of your key shards');
  static String get onPhone => _localizations?.onPhone ?? _p('手机上', 'On Phone');
  static String get inCloud => _localizations?.inCloud ?? _p('云端', 'In Cloud');
  static String get recovery => _localizations?.recovery ?? _p('恢复', 'Recovery');
  static String get allSafe => _localizations?.allSafe ?? _p('全部安全', 'All Safe');
  static String get keyStatusError => _localizations?.keyStatusError ?? _p('发现问题', 'Issues Found');
  static String get keyStatusWarning => _localizations?.keyStatusWarning ?? _p('尽快检查', 'Check Soon');
  static String get rotateKeyShares => _localizations?.rotateKeyShares ?? _p('轮换密钥分片', 'Rotate Key Shares');
  static String get presignatures => _localizations?.presignatures ?? _p('预签名', 'Presignatures');
  static String get presignaturesSub => _localizations?.presignaturesSub ?? _p('离线签名实现快速交易', 'Offline signing for faster transactions');
  static String get lastRotation => _localizations?.lastRotation ?? _p('上次轮换', 'Last rotation');
  static String get never => _localizations?.never ?? _p('从未', 'Never');
  static String get today => _localizations?.today ?? _p('今天', 'Today');

  static String get voiceInput => _localizations?.voiceInput ?? _p('语音输入', 'Voice Input');
  static String get voiceInputSub => _localizations?.voiceInputSub ?? _p('使用语音与 AI 助手交互', 'Use voice to interact with AI assistant');
  static String get aiModel => _localizations?.aiModel ?? _p('AI 模型', 'AI Model');
  static String get aiModelSub => _localizations?.aiModelSub ?? _p('选择你喜欢的 AI 助手', 'Choose your preferred AI assistant');
  static String get language => _localizations?.language ?? _p('语言', 'Language');
  static String get weeklyReport => _localizations?.weeklyReport ?? _p('周报', 'Weekly Report');
  static String get weeklyReportSub => _localizations?.weeklyReportSub ?? _p('每周获取资产摘要', 'Get weekly portfolio summary');
  static String get redoOnboarding => _localizations?.redoOnboarding ?? _p('重新引导', 'Redo Onboarding');
  static String get redoOnboardingSub => _localizations?.redoOnboardingSub ?? _p('重新开始钱包创建流程', 'Start over with wallet creation');
  static String get on => _localizations?.on ?? _p('开启', 'ON');
  static String get off => _localizations?.off ?? _p('关闭', 'OFF');

  static String get resetWalletTitle => _localizations?.resetWalletTitle ?? _p('钱包有余额', 'Wallet Has Balance');
  static String get resetWalletHasBalance => _localizations?.resetWalletHasBalance ?? _p('重置前请先将资产转移到其他钱包。', 'Please transfer your assets to another wallet before resetting.');
  static String get resetWalletGoTransfer => _localizations?.resetWalletGoTransfer ?? _p('去转账', 'Go Transfer');
  static String get resetWalletConfirmTitle => _localizations?.resetWalletConfirmTitle ?? _p('重置钱包?', 'Reset Wallet?');
  static String get resetWalletConfirmBody => _localizations?.resetWalletConfirmBody ?? _p('这将删除你的钱包。你需要通过备份来恢复。', 'This will delete your wallet. You\'ll need to recover from your backup.');
  static String get resetWalletConfirm => _localizations?.resetWalletConfirm ?? _p('重置', 'Reset');
  static String get resetWalletChecking => _localizations?.resetWalletChecking ?? _p('检查余额中...', 'Checking balance...');

  static String get signoff1 => _localizations?.signoff1 ?? _p('用 ❤️ 为未来的货币而建', 'Built with ❤️ for the future of money');
  static String signoff2(String version) => _localizations?.signoff2(version) ?? 'v$version';

  static String get cancel => _localizations?.cancel ?? _p('取消', 'Cancel');
  static String get confirm => _localizations?.confirm ?? _p('确认', 'Confirm');
  static String get save => _localizations?.save ?? _p('保存', 'Save');
  static String get delete => _localizations?.delete ?? _p('删除', 'Delete');
  static String get copy => _localizations?.copy ?? _p('复制', 'Copy');
  static String get copied => _localizations?.copied ?? _p('已复制', 'Copied');
  static String get send => _localizations?.send ?? _p('发送', 'Send');
  static String get receive => _localizations?.receive ?? _p('接收', 'Receive');
  static String get swap => _localizations?.swap ?? _p('兑换', 'Swap');
  static String get more => _localizations?.more ?? _p('更多', 'More');

  static String get loading => _localizations?.loading ?? _p('加载中...', 'Loading...');
  static String get error => _localizations?.error ?? _p('错误', 'Error');
  static String get success => _localizations?.success ?? _p('成功', 'Success');
  static String get failed => _localizations?.failed ?? _p('失败', 'Failed');
  static String get retryLater => _localizations?.retryLater ?? _p('请稍后重试', 'Please try again later');
  static String get comingSoon => _localizations?.comingSoon ?? _p('即将推出', 'Coming soon');

  static String get wallet => _localizations?.wallet ?? _p('钱包', 'Wallet');
  static String get home => _localizations?.home ?? _p('首页', 'Home');
  static String get balance => _localizations?.balance ?? _p('余额', 'Balance');
  static String get transactions => _localizations?.transactions ?? _p('交易记录', 'Transactions');
  static String get contacts => _localizations?.contacts ?? _p('联系人', 'Contacts');
  static String get scan => _localizations?.scan ?? _p('扫码', 'Scan QR');
  static String get help => _localizations?.help ?? _p('帮助', 'Help');

  static String get amount => _localizations?.amount ?? _p('金额', 'Amount');
  static String get to => _localizations?.to ?? _p('接收方', 'To');
  static String get from => _localizations?.from ?? _p('发送方', 'From');
  static String get gas => _localizations?.gas ?? _p('Gas 费', 'Gas Fee');
  static String get total => _localizations?.total ?? _p('总计', 'Total');
  static String get max => _localizations?.max ?? _p('最大', 'Max');
  static String get memo => _localizations?.memo ?? _p('备注', 'Memo');
  static String get optional => _localizations?.optional ?? _p('可选', 'Optional');

  static String get sendConfirmTitle => _localizations?.sendConfirmTitle ?? _p('确认发送', 'Confirm Send');
  static String sendConfirmBody(String amount, String symbol, String address) =>
    _localizations?.sendConfirmBody(amount, symbol, address) ??
    _p('发送 $amount $symbol 到 $address?', 'Send $amount $symbol to $address?');

  static String get transactionSent => _localizations?.transactionSent ?? _p('交易已发送', 'Transaction sent');
  static String get transactionFailed => _localizations?.transactionFailed ?? _p('交易失败', 'Transaction failed');
  static String get insufficientBalance => _localizations?.insufficientBalance ?? _p('余额不足', 'Insufficient balance');

  static String get contactName => _localizations?.contactName ?? _p('联系人名称', 'Contact Name');
  static String get contactAddress => _localizations?.contactAddress ?? _p('钱包地址', 'Wallet Address');
  static String get addContact => _localizations?.addContact ?? _p('添加联系人', 'Add Contact');
  static String get editContact => _localizations?.editContact ?? _p('编辑联系人', 'Edit Contact');
  static String get deleteContact => _localizations?.deleteContact ?? _p('删除联系人', 'Delete Contact');
  static String get noContacts => _localizations?.noContacts ?? _p('暂无联系人', 'No contacts yet');
  static String get addFirstContact => _localizations?.addFirstContact ?? _p('添加第一个联系人', 'Add your first contact');

  static String get chatPlaceholder => _localizations?.chatPlaceholder ?? _p('问我任何问题...', 'Ask me anything...');
  static String get newChat => _localizations?.newChat ?? _p('新对话', 'New Chat');
  static String get chatHistory => _localizations?.chatHistory ?? _p('对话历史', 'Chat History');
  static String get clearHistory => _localizations?.clearHistory ?? _p('清空历史', 'Clear History');
  static String get clearHistoryConfirm => _localizations?.clearHistoryConfirm ?? _p('清空所有对话历史?', 'Clear all chat history?');

  static String get yield => _localizations?.yield ?? _p('理财', 'Yield');
  static String get pools => _localizations?.pools ?? _p('资金池', 'Pools');
  static String get apr => _localizations?.apr ?? _p('年化收益', 'APR');
  static String get deposit => _localizations?.deposit ?? _p('存入', 'Deposit');
  static String get withdraw => _localizations?.withdraw ?? _p('取出', 'Withdraw');
  static String get yourPosition => _localizations?.yourPosition ?? _p('你的持仓', 'Your Position');
  static String get totalValue => _localizations?.totalValue ?? _p('总价值', 'Total Value');

  // Chat suggestions
  static String get suggestBalance => _p('我的余额是多少', "What's my balance");
  static String get suggestRecentTx => _p('最近的交易记录', 'Recent transactions');
  static String get suggestSecurityAudit => _p('安全审计', 'Security audit');
  static String get suggestAddress => _p('我的收款地址', 'Show my address');

  // Error messages
  static String get requestFailed => _p('请求失败，请稍后重试', 'Request failed, please try again');
  static String get networkError => _p('网络错误，请稍后重试', 'Network error, please try again');
  static String get insufficientGasWarning => _p('⚠ 余额不足以支付Gas费', '⚠ Insufficient balance for gas');

  // Chat UI
  static String get askCowallet => _p('问问CoWallet', 'Ask CoWallet');
  static String get chatEmpty => _p('还没问过什么问题', 'No questions yet');
  static String get chatEmptySub => _p('试试问"我的余额是多少"', 'Try asking "What\'s my balance?"');
  static String get composerHint => _p('输入问题...', 'Type a question...');
  static String get thinking => _p('正在思考', 'Thinking');
  static String get transferCancelled => _p('已取消转账', 'Transfer cancelled');
  static String get swapCancelled => _p('已取消兑换', 'Swap cancelled');

  // Transfer
  static String get sendAll => _p('全部', 'All');

  // Token warnings
  static String tokenBalanceZeroWarning(String token) =>
    _p('⚠ $token 余额为 0', '⚠ $token balance is 0');
}