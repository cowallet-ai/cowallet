// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get appName => 'CoWallet';

  @override
  String get tagline => '会听懂人话的钱包';

  @override
  String get tabHome => '首页';

  @override
  String get tabWallet => '钱包';

  @override
  String get tabAsk => '问';

  @override
  String get tabAgents => '助手';

  @override
  String get tabSettings => '设置';

  @override
  String get tabDefi => 'DeFi';

  @override
  String get heroKicker => '数字钱包 · 会听懂人话';

  @override
  String get heroH1a => '会听你说话的';

  @override
  String get heroH1b => '';

  @override
  String get heroH1em => '钱包';

  @override
  String get heroExplain => '就像给你家请了个管家——你说"帮我转 100 块给小明",它就去做;你不会说也没关系,它有按钮。';

  @override
  String get heroFeat1h => '不用懂区块链';

  @override
  String get heroFeat1s => '说句话就能转账、收款、理财';

  @override
  String get heroFeat2h => '100+ 个金融网络';

  @override
  String get heroFeat2s => '全世界通用';

  @override
  String get heroFeat3h => 'AI 帮你跑腿';

  @override
  String get heroFeat3s => '你只需说一句话';

  @override
  String get getStarted => '开始使用';

  @override
  String get heroLegal => '继续即表示同意服务条款与隐私政策';

  @override
  String get introH1 => '你的钱包如何保护你';

  @override
  String get introSub => 'CoWallet 用一种叫"门限签名"的技术,把钥匙拆成三份。';

  @override
  String get introBullet1h => '钥匙拆成三份';

  @override
  String get introBullet1s => '手机一份、服务器一份、你自己保管一份。完整钥匙从不出现在任何地方。';

  @override
  String get introBullet2h => '动钱需要两份';

  @override
  String get introBullet2s => '任何单方(包括 CoWallet)都无法单独动你的钱。';

  @override
  String get introBullet3h => '没有助记词';

  @override
  String get introBullet3s => '不用抄 12 个单词。丢了手机,用你的备份 + 服务器就能恢复。';

  @override
  String get introStart => '开始创建';

  @override
  String get emailH1 => '绑定恢复邮箱';

  @override
  String get emailSub => '用于账户恢复时验证身份,我们不会发送垃圾邮件。';

  @override
  String get emailHint => '此邮箱仅用于钱包恢复验证';

  @override
  String get invalidEmail => '请输入有效的邮箱地址';

  @override
  String get emailSendFailed => '发送验证码失败,请重试';

  @override
  String get emailAlreadyRegistered => '该邮箱已注册';

  @override
  String get emailAlreadyRegisteredDesc => '该邮箱已关联钱包,是否前往恢复流程?';

  @override
  String get goRecovery => '去恢复';

  @override
  String get reRegister => '重新注册';

  @override
  String get reRegisterDesc => '将创建新钱包,原钱包资产需通过恢复流程找回';

  @override
  String get reRegisterConfirm => '确认重新注册';

  @override
  String get otpH1 => '输入验证码';

  @override
  String otpSub(String email) {
    return '验证码已发送至 $email';
  }

  @override
  String get otpResend => '重新发送验证码';

  @override
  String get otpInvalid => '验证码错误或已过期';

  @override
  String get creatingH1 => '正在帮你把钥匙分成三份';

  @override
  String get creatingSub => '动你的钱需要任意两份钥匙。三份分开存放,丢了一份还能恢复。完整的钥匙从不出现在任何地方。';

  @override
  String get cl1 => '第 1 份:存在这台手机里';

  @override
  String get cl2 => '第 2 份:存在服务器保险柜';

  @override
  String get cl3 => '第 3 份:由你自己保管';

  @override
  String get createError => '钱包创建失败,请重试。';

  @override
  String get retry => '重试';

  @override
  String get bioH1 => '开启生物识别';

  @override
  String get bioSub => '就像手机解锁一样,用指纹或面容保护你的钱包。生物信息不会离开这台手机。';

  @override
  String get bioActivate => '开启生物识别';

  @override
  String get bioSkip => '改用密码';

  @override
  String get bioVerifying => '正在验证...';

  @override
  String get bioDone => '生物识别已开启';

  @override
  String get pinH1 => '设置钱包密码';

  @override
  String get pinSub => '6 位数字密码,每次交易时需要输入。';

  @override
  String get pinConfirmH1 => '再输入一次';

  @override
  String get pinConfirmSub => '请再输入一遍以确认。';

  @override
  String get pinMismatch => '两次输入不一致,请重新设置';

  @override
  String get pinDone => '密码已设置';

  @override
  String get nameH1 => '我该怎么叫你?';

  @override
  String get nameSub => '起个名字就行,不用真名。';

  @override
  String get namePlaceholder => '比如 小明 / 老王 / Alice';

  @override
  String get nameTooShort => '名字太短';

  @override
  String get nameTooLong => '名字太长';

  @override
  String get settings => '设置';

  @override
  String get security => '安全';

  @override
  String get keySecurity => '密钥安全';

  @override
  String get conversation => '对话';

  @override
  String get general => '一般';

  @override
  String get biometricAuth => '生物识别';

  @override
  String get biometricAuthReason => '请验证身份以继续';

  @override
  String get biometricNotAvailable => '此设备不支持';

  @override
  String get biometricEnable => '开启';

  @override
  String get biometricDisable => '关闭';

  @override
  String get emergencyFreeze => '紧急冻结';

  @override
  String get emergencyFreezeSub => '暂时冻结所有交易';

  @override
  String get emergencyFreezeConfirmTitle => '冻结钱包?';

  @override
  String get emergencyFreezeConfirmBody => '所有交易将被阻止,直到你解冻。';

  @override
  String get emergencyFreezeActivated => '钱包已冻结';

  @override
  String get emergencyFreezeDeactivated => '钱包已解冻';

  @override
  String get frozenBanner => '钱包已冻结,点「安全」可解冻';

  @override
  String get emergencyContact => '紧急联系人';

  @override
  String get emergencyContactSub => '设置可信联系人用于恢复';

  @override
  String get riskGuard => '风险防护';

  @override
  String get riskGuardSub => '自定义安全规则和限额';

  @override
  String get keysCheckup => '密钥检查';

  @override
  String get keysCheckupSub => '检查密钥分片的健康状态';

  @override
  String get onPhone => '手机上';

  @override
  String get inCloud => '云端';

  @override
  String get recovery => '恢复';

  @override
  String get allSafe => '全部安全';

  @override
  String get keyStatusError => '发现问题';

  @override
  String get keyStatusWarning => '尽快检查';

  @override
  String get rotateKeyShares => '轮换密钥分片';

  @override
  String get presignatures => '预签名';

  @override
  String get presignaturesSub => '离线签名实现快速交易';

  @override
  String get lastRotation => '上次轮换';

  @override
  String get never => '从未';

  @override
  String get today => '今天';

  @override
  String get voiceInput => '语音输入';

  @override
  String get voiceInputSub => '使用语音与 AI 助手交互';

  @override
  String get aiModel => 'AI 模型';

  @override
  String get aiModelSub => '选择你喜欢的 AI 助手';

  @override
  String get language => '语言';

  @override
  String get weeklyReport => '周报';

  @override
  String get weeklyReportSub => '每周获取资产摘要';

  @override
  String get redoOnboarding => '重新引导';

  @override
  String get redoOnboardingSub => '重新开始钱包创建流程';

  @override
  String get on => '开启';

  @override
  String get off => '关闭';

  @override
  String get resetWalletTitle => '钱包有余额';

  @override
  String get resetWalletHasBalance => '重置前请先将资产转移到其他钱包。';

  @override
  String get resetWalletGoTransfer => '去转账';

  @override
  String get resetWalletConfirmTitle => '重置钱包?';

  @override
  String get resetWalletConfirmBody => '这将删除你的钱包。你需要通过备份来恢复。';

  @override
  String get resetWalletConfirm => '重置';

  @override
  String get resetWalletChecking => '检查余额中...';

  @override
  String get deleteAccount => '删除账户';

  @override
  String get deleteAccountSub => '永久删除账户及全部数据';

  @override
  String get deleteAccountHasBalance => '你的钱包仍有余额。删除账户将永久销毁密钥，资产将无法找回。建议先将资产转出。仍要继续吗？';

  @override
  String get deleteAccountConfirmTitle => '永久删除账户？';

  @override
  String get deleteAccountConfirmBody => '此操作不可撤销。你的钱包、密钥分片、交易记录和所有数据将被永久删除，且无法恢复。';

  @override
  String get deleteAccountConfirm => '永久删除';

  @override
  String get deleteAccountDeleting => '正在删除账户...';

  @override
  String get deleteAccountSuccess => '账户已删除';

  @override
  String get deleteAccountFailed => '删除失败，请稍后重试';

  @override
  String get signoff1 => 'CoWallet · 2026';

  @override
  String signoff2(String version) {
    return 'v$version';
  }

  @override
  String get cancel => '取消';

  @override
  String get confirm => '确认';

  @override
  String get save => '保存';

  @override
  String get delete => '删除';

  @override
  String get copy => '复制';

  @override
  String get copied => '已复制';

  @override
  String get send => '发送';

  @override
  String get receive => '接收';

  @override
  String get swap => '兑换';

  @override
  String get more => '更多';

  @override
  String get loading => '加载中...';

  @override
  String get error => '错误';

  @override
  String get success => '成功';

  @override
  String get failed => '失败';

  @override
  String get retryLater => '请稍后重试';

  @override
  String get wallet => '钱包';

  @override
  String get home => '首页';

  @override
  String get balance => '余额';

  @override
  String get transactions => '交易记录';

  @override
  String get contacts => '联系人';

  @override
  String get scan => '扫码';

  @override
  String get help => '帮助';

  @override
  String get amount => '金额';

  @override
  String get to => '接收方';

  @override
  String get from => '发送方';

  @override
  String get gas => 'Gas 费';

  @override
  String get total => '总计';

  @override
  String get max => '最大';

  @override
  String get memo => '备注';

  @override
  String get optional => '可选';

  @override
  String get sendConfirmTitle => '确认发送';

  @override
  String sendConfirmBody(String amount, String symbol, String address) {
    return '发送 $amount $symbol 到 $address?';
  }

  @override
  String get transactionSent => '交易已发送';

  @override
  String get transactionFailed => '交易失败';

  @override
  String get insufficientBalance => '余额不足';

  @override
  String get contactName => '联系人名称';

  @override
  String get contactAddress => '钱包地址';

  @override
  String get addContact => '添加联系人';

  @override
  String get editContact => '编辑联系人';

  @override
  String get deleteContact => '删除联系人';

  @override
  String get noContacts => '暂无联系人';

  @override
  String get addFirstContact => '添加第一个联系人';

  @override
  String get chatPlaceholder => '问我任何问题...';

  @override
  String get newChat => '新对话';

  @override
  String get chatHistory => '对话历史';

  @override
  String get clearHistory => '清空历史';

  @override
  String get clearHistoryConfirm => '清空所有对话历史?';

  @override
  String get yield => '理财';

  @override
  String get pools => '资金池';

  @override
  String get apr => '年化收益';

  @override
  String get deposit => '存入';

  @override
  String get withdraw => '取出';

  @override
  String get yourPosition => '你的持仓';

  @override
  String get totalValue => '总价值';

  @override
  String get comingSoon => '即将推出';

  @override
  String get quickSetup => '快速设置';

  @override
  String get dailyLimit => '每日限额';

  @override
  String dailyLimitDesc(String amount) {
    return '每日转账限额最高 $amount';
  }

  @override
  String get largeTransferConfirm => '大额转账确认';

  @override
  String largeTransferDesc(String amount) {
    return '单笔转账超过 $amount 需确认';
  }

  @override
  String get activePolicies => '已激活策略';

  @override
  String get noPolicies => '暂无策略,可使用上方快速设置';

  @override
  String deletePolicyConfirm(String name) {
    return '删除「$name」?';
  }

  @override
  String get yesterday => '昨天';

  @override
  String daysAgo(int days) {
    return '$days 天前';
  }

  @override
  String monthsAgo(int months) {
    return '$months 个月前';
  }

  @override
  String get languageLabel => '中文';

  @override
  String contactSaved(String name) {
    return '✅ 已保存联系人「$name」';
  }

  @override
  String get releaseToCancel => '松手取消';

  @override
  String get slideUpToSend => '上滑取消,松手发送';

  @override
  String get voiceListening => '正在聆听…';

  @override
  String get voiceTapToFinish => '点击任意处结束';

  @override
  String get voiceDone => '完成';

  @override
  String get voiceUnavailable => '语音识别不可用,请检查麦克风权限';

  @override
  String get voiceErrorHint => '语音识别出错,请重试';

  @override
  String get saveContact => '保存联系人';

  @override
  String get cautions => '注意事项';

  @override
  String get passed => '通过';

  @override
  String get safetyAdvice => '安全建议';

  @override
  String get riskLevelSafe => '安全';

  @override
  String get riskLevelMedium => '中等风险';

  @override
  String get riskLevelHigh => '高风险';

  @override
  String get riskLevelUnknown => '未知';

  @override
  String get securityAudit => '安全审计';

  @override
  String get riskItems => '风险项';

  @override
  String get allEvmChains => '支持所有 EVM 链';

  @override
  String get addressCopied => '地址已复制';

  @override
  String get copyAddress => '复制地址';

  @override
  String get transferRejected => '转账被拒';

  @override
  String get transferExceedsLimit => '超过转账限额';

  @override
  String limitLabel(String limit) {
    return '限额: $limit';
  }

  @override
  String get adjustLimitHint => '请调整金额或在「设置 > 转账限额」中修改限额';

  @override
  String get featureAiIntent => '说说 AI 意图识别';

  @override
  String get featureProxyPay => '说说代理支付';

  @override
  String get featureFamily => '说说家庭共享钱包';

  @override
  String get featureSkills => '说说技能扩展';

  @override
  String get checkBalance => '查询余额';

  @override
  String chainBalance(String chain, String token) {
    return '查询 $chain 上的 $token 余额';
  }

  @override
  String get recentTransactions => '最近的交易记录';

  @override
  String get backupVerifyFailed => '备份验证失败,请确认导入了正确的备份文件';

  @override
  String get backupFormatInvalid => '备份格式无效,请检查文件是否完整';

  @override
  String get noCloudBackup => '未找到云端备份,请尝试从文件导入';

  @override
  String viewChainAssets(String chain) {
    return '查看 $chain 上的资产';
  }

  @override
  String get aaveUsdc => 'Aave 上的 USDC';

  @override
  String get baseAudited => 'Base 链 · 已审计';

  @override
  String get agentRule1 => '只读余额,每日限额 \$500,不可质押';

  @override
  String agentSigned(int signed, int total) {
    return '已签名 $signed/$total';
  }

  @override
  String get agentRule2 => '团队报销,审批后自动支付';

  @override
  String pinError(Object attempts) {
    return 'PIN 错误,剩余 $attempts 次尝试';
  }
}
