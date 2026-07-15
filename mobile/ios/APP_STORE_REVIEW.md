# CoWallet — iOS App Store 审核物料清单

> 生成日期：2026-07-07 · 版本 `1.0.1 (build 3)` · Bundle ID `com.cowallet.app`
> 类目：加密货币钱包(MPC / 非托管)。属高敏感类目,审核从严。

---

## 0. 🔴 提交前必须解决的拦路石

| # | 问题 | 指南 | 状态 |
|---|------|------|------|
| 1 | 有 Google 登录但缺 **Sign in with Apple** | 4.8 | ☐ 待修复 |
| 2 | 加密钱包须由 **企业(Organization)开发者账号**提交,个人账号会被拒 | 3.1.5(b) | ☐ 待确认 |
| 3 | 非托管钱包:确认 App 内**不涉及** App 外数字商品销售/法币出入金抽成 | 3.1.1 / 3.1.5 | ☐ 待确认 |

---

## 1. App 基本信息(App Store Connect 填写)

| 字段 | 值 |
|------|-----|
| App 名称 | CoWallet AI(显示名) / CoWallet |
| 副标题(30字符内) | 待定，例:AI-Native MPC Wallet |
| Bundle ID | com.cowallet.app |
| 版本号 | 1.0.1 |
| Build | 3 |
| 主类别 | Finance(财务) |
| 次类别 | Utilities(可选) |
| 内容分级 | 需在问卷中如实回答;金融类通常 17+ |
| 加密合规 | Info.plist 已设 `ITSAppUsesNonExemptEncryption = false`(仅用标准/豁免加密时正确;MPC 若含自研密码学需复核此项) |

## 2. 文案物料(需人工定稿/翻译)

- ☐ **App 描述**(中/英,主要市场语言均需)
- ☐ **关键词**(100 字符,逗号分隔)
- ☐ **推广文本**(170 字符,可后续更新不需审核)
- ☐ **What's New / 更新说明**
- ☑ **技术支持 URL**(必填):`https://cowallet.ai/support` — 已新建(FAQ + support@cowallet.ai)
- ☑ **营销 URL**(可选):`https://cowallet.ai`
- ☑ **隐私政策 URL**(必填):`https://cowallet.ai/privacy`
- ☑ **服务条款 / EULA URL**:`https://cowallet.ai/terms`
- ⚠️ 上述 URL 均需 **`cowallet.ai` 正式部署上线**后才可用;审核期间必须公网可访问
- ☑ 邮箱域名已统一为 `@cowallet.ai`(privacy/terms 正文与 support 页一致:privacy@cowallet.ai、legal@cowallet.ai、support@cowallet.ai)

## 3. 视觉物料

- ☑ App 图标 1024×1024:`Assets.xcassets/AppIcon.appiconset/Icon-App-1024x1024@1x.png` 已存在(确认无 alpha 通道、无圆角)
- ☐ **截图(必需)**:
  - 6.9" (iPhone 16 Pro Max, 1320×2868) — 必需一组
  - 6.5" (iPhone 14 Plus 等) — 建议
  - iPad 13"(若声明支持 iPad,当前 Info.plist 含 iPad 方向配置 → **需提供或改为仅 iPhone**)
  - 每语言 3–10 张
- ☐ **App 预览视频**(可选)

## 4. 隐私(App Privacy "营养标签")

Info.plist 已声明的权限用途字符串(审核会逐条核对是否与实际功能一致):

| 权限 | 用途文案 | 实际用到? |
|------|----------|-----------|
| 相机 `NSCameraUsageDescription` | 扫描收款地址/交易二维码 | ☐ 核对 |
| 麦克风 `NSMicrophoneUsageDescription` | AI 对话语音输入 | ☐ 核对 |
| 语音识别 `NSSpeechRecognitionUsageDescription` | 语音转文字命令 | ☐ 核对 |
| Face ID `NSFaceIDUsageDescription` | Secure Enclave 保护密钥 | ☐ 核对 |
| 相册 `NSPhotoLibraryUsageDescription` | 保存/导入二维码、备份 | ☐ 核对 |

- `PrivacyInfo.xcprivacy`:已声明 3 类 API 使用原因(FileTimestamp/SystemBootTime/UserDefaults),`NSPrivacyTracking=false`。
- ☑ **已补齐 `NSPrivacyCollectedDataTypes`**(与 legal.ts §1 一致,均为 App 功能用途、非追踪):
  邮箱(EmailAddress)、用户ID/钱包地址(UserID)、设备标识/FCM+设备身份公钥(DeviceID)、财务信息/余额持仓交易(OtherFinancialInfo)、用户内容/AI对话+应用内联系人(OtherUserContent)、技术日志(OtherDiagnosticData,未关联身份)、加密分片元数据(OtherDataTypes)。
- ⚠️ **仍需手动操作**:App Store 隐私"营养标签"不是由 `.xcprivacy` 生成的,必须在 **App Store Connect › App 隐私** 问卷里逐项勾选。请按下表填写,并对每一项在"是否与第三方共享"处如实勾选(AI 数据共享给 Anthropic/AWS Bedrock、DeepSeek;推送经 Firebase;链上查询经 OKX)。填写口径必须与本清单、legal.ts、App 内同意弹窗三者一致。

  | App Store Connect 数据类型 | 是否收集 | 关联身份 | 用于追踪 | 用途 | 共享第三方 |
  |------|------|------|------|------|------|
  | Email Address | 是 | 是 | 否 | App 功能 | 否 |
  | User ID(钱包地址/用户ID) | 是 | 是 | 否 | App 功能 | 是(AI 请求上下文) |
  | Device ID(FCM token/设备身份公钥) | 是 | 是 | 否 | App 功能 | 是(Firebase 推送) |
  | Other Financial Info(余额/持仓/交易) | 是 | 是 | 否 | App 功能 | 是(AI 上下文;OKX 查询) |
  | Other User Content(AI 对话/应用内联系人) | 是 | 是 | 否 | App 功能 | 是(AI 服务商) |
  | Other Data Types(加密分片元数据) | 是 | 是 | 否 | App 功能 | 否 |
  | Crash / Performance / Other Diagnostic | 是(有限技术日志) | 否 | 否 | App 功能 | 否 |

  说明:App 未集成广告/分析 SDK(Firebase 仅 Core+Messaging),无 Crashlytics/Analytics;语音走 Apple 系统识别;联系人为应用内自建、不读系统通讯录。故"用于追踪"全部为否,`NSPrivacyTracking=false`。

## 5. 审核演示账号 / App Review Information

加密钱包必须让审核员能完整跑通,否则以"无法审核"拒绝:

- ☐ **演示账号**:提供可登录的测试账号(邮箱+验证方式),或说明如何注册
- ☐ **测试网说明**:如资产在测试网,备注中说明如何领取测试币
- ☐ **MPC 2-of-3 流程说明**:向审核员解释设备/服务器/备份三分片,避免被误判为"无法使用"
- ☐ **联系人**:姓名、电话、邮箱
- ☐ **备注(Notes)**:说明这是非托管钱包、私钥永不完整存在、无 App 内购买法币等

## 6. 构建与签名

- ☐ 生产证书 + App Store Provisioning Profile(Team 主体 = 企业)
- ☐ `Runner.entitlements`:keychain-access-group、iCloud KVStore 已配 → 确认对应 Capability 已在 Apple Developer 后台 App ID 开启
- ☐ Archive(Release)→ 上传 TestFlight → 内部测试跑通 → 提交审核
- ☐ 确认无调试符号/日志泄露密钥

## 7. 常见拒因预防(加密钱包专项)

- 3.1.1:App 内不得引导购买 App 外数字内容或规避 IAP
- 5.2.1:确认拥有 "CoWallet" 名称与品牌使用权
- 2.1:功能完整,无占位页/死链;后端 API 在审核期间必须在线可用
- 4.2:非托管钱包本身有实用价值(转账/签名/AI 助手),满足最低功能门槛
