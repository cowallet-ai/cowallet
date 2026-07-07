// Bilingual legal content (zh / en) for the Privacy Policy and Terms of Service.
// Kept out of the UI i18n map because these are long-form documents, not UI strings.

export type Bi = { zh: string; en: string };

export interface LegalSection {
  heading: Bi;
  // Each entry is a paragraph. A leading "• " marks a bullet item.
  paragraphs: Bi[];
}

export interface LegalContent {
  title: Bi;
  updated: Bi;
  intro: Bi;
  sections: LegalSection[];
}

const EFFECTIVE_DATE = "2026-07-07";

export const privacyContent: LegalContent = {
  title: { zh: "隐私政策", en: "Privacy Policy" },
  updated: {
    zh: `最后更新：${EFFECTIVE_DATE}`,
    en: `Last updated: ${EFFECTIVE_DATE}`,
  },
  intro: {
    zh: "cowallet 是一款 AI 原生的 MPC（多方计算）加密钱包。我们从产品设计之初就把隐私放在核心：你的私钥从不以完整形态存在，我们也无法单独动用你的资产。本政策说明我们收集哪些信息、为什么收集、如何使用与保护，以及你拥有哪些权利。",
    en: "cowallet is an AI-native MPC (Multi-Party Computation) crypto wallet. Privacy is built into the product from the ground up: your private key never exists in complete form, and we cannot move your assets on our own. This policy explains what information we collect, why we collect it, how we use and protect it, and the rights you have.",
  },
  sections: [
    {
      heading: { zh: "1. 我们收集的信息", en: "1. Information We Collect" },
      paragraphs: [
        {
          zh: "• 账户与身份：你的注册标识（如邮箱或设备标识）、公钥地址，以及设备身份公钥（iOS 为 P-256，Android 为 RSA）。",
          en: "• Account & identity: your registration identifier (e.g. email or device identifier), public-key addresses, and your device identity public key (P-256 on iOS, RSA on Android).",
        },
        {
          zh: "• 密钥分片元数据：我们保管三份密钥分片之一（服务器分片）的加密数据及其元信息。我们从不持有、也无法重建你的完整私钥。",
          en: "• Key-shard metadata: we hold one of three key shards (the server shard) as encrypted data along with its metadata. We never hold, and cannot reconstruct, your full private key.",
        },
        {
          zh: "• 交易与链上数据：为展示余额与交易历史，我们会查询公开区块链数据（通过 OKX 钱包 API 等服务）。链上数据本身是公开的。",
          en: "• Transaction & on-chain data: to display balances and history we query public blockchain data (via services such as the OKX Wallet API). On-chain data is public by nature.",
        },
        {
          zh: "• AI 对话内容：当你使用 AI 助手（转账、余额查询、交易解读等）时，你的指令会被发送至 AI 服务商（AWS Bedrock 上的 Claude，或 DeepSeek 作为回退）以生成回应。",
          en: "• AI conversation content: when you use the AI assistant (transfers, balance lookups, transaction explanations), your instructions are sent to our AI providers (Claude on AWS Bedrock, with DeepSeek as a fallback) to generate responses.",
        },
        {
          zh: "• 推送令牌：若你启用通知，我们会保存 Firebase Cloud Messaging（FCM）推送令牌。",
          en: "• Push tokens: if you enable notifications, we store your Firebase Cloud Messaging (FCM) push token.",
        },
        {
          zh: "• 技术日志：为安全与故障排查，我们会记录有限的技术信息（如请求时间戳、错误码、粗略的设备/网络信息）。",
          en: "• Technical logs: for security and troubleshooting we record limited technical information (e.g. request timestamps, error codes, coarse device/network info).",
        },
      ],
    },
    {
      heading: { zh: "2. 我们不收集什么", en: "2. What We Do Not Collect" },
      paragraphs: [
        {
          zh: "• 完整私钥或助记词：MPC 架构下不存在可被任何单方还原的完整私钥。你的设备分片保存在设备安全区（Secure Enclave / Keystore），我们无从获取。",
          en: "• Full private keys or seed phrases: under the MPC architecture there is no full private key that any single party can reconstruct. Your device shard stays in your device's secure enclave / keystore, out of our reach.",
        },
        {
          zh: "• 我们不会为广告目的出售或出租你的个人信息。",
          en: "• We do not sell or rent your personal information for advertising purposes.",
        },
      ],
    },
    {
      heading: { zh: "3. 我们如何使用信息", en: "3. How We Use Information" },
      paragraphs: [
        {
          zh: "• 提供核心功能：参与 MPC 签名协议、展示余额与交易、执行你授权的交易。",
          en: "• Deliver core features: participate in the MPC signing protocol, display balances and transactions, and execute the transactions you authorize.",
        },
        {
          zh: "• 支持 AI 能力：解析你的自然语言意图并生成回应；AI 只能在你设定的策略边界内行动，越界即停止。",
          en: "• Power AI capabilities: parse your natural-language intent and generate responses. The AI can only act within the policy boundaries you set, and stops when a boundary is crossed.",
        },
        {
          zh: "• 保障安全：风险检测、速率限制、防欺诈，以及满足合规义务。",
          en: "• Protect security: risk detection, rate limiting, fraud prevention, and meeting compliance obligations.",
        },
        {
          zh: "• 发送通知：交易状态、审批请求与安全提醒。",
          en: "• Send notifications: transaction status, approval requests, and security alerts.",
        },
      ],
    },
    {
      heading: { zh: "4. 第三方服务", en: "4. Third-Party Services" },
      paragraphs: [
        {
          zh: "我们依赖以下服务商，仅共享实现其功能所必需的最少数据：AWS Bedrock / DeepSeek（AI 推理）、OKX 钱包 API（余额与交易历史）、Anchorage Digital（受美国监管的云端分片托管，通过 SOC2 / ISO27001）、Firebase Cloud Messaging（推送通知），以及各区块链网络的 RPC 节点。",
          en: "We rely on the following providers and share only the minimum data needed for each to function: AWS Bedrock / DeepSeek (AI inference), OKX Wallet API (balances and transaction history), Anchorage Digital (US-regulated custody of the cloud shard, SOC2 / ISO27001), Firebase Cloud Messaging (push notifications), and blockchain RPC nodes for each network.",
        },
        {
          zh: "这些服务商各自的隐私实践受其自身政策约束。",
          en: "Each provider's privacy practices are governed by their own respective policies.",
        },
      ],
    },
    {
      heading: { zh: "5. 数据存储与安全", en: "5. Data Storage & Security" },
      paragraphs: [
        {
          zh: "服务器分片以 AES-GCM 加密存储。传输采用 TLS，MPC 消息通道采用 Noise_XX 加密握手。门限签名（TSS）确保任一分片都不产生完整私钥——即使 cowallet 遭到入侵，攻击者也只拿到一份分片，无法动用你的资产。",
          en: "The server shard is stored encrypted with AES-GCM. Data in transit uses TLS, and the MPC message channel uses a Noise_XX encrypted handshake. Threshold signatures (TSS) ensure no single shard reconstructs the full private key — even if cowallet were breached, an attacker would hold only one shard and could not move your assets.",
        },
      ],
    },
    {
      heading: { zh: "6. 你的权利", en: "6. Your Rights" },
      paragraphs: [
        {
          zh: "你有权访问、更正或删除你的个人信息，撤回同意，以及导出数据。由于加密钱包的自主性，删除账户不会影响已上链的公开交易记录。如需行使权利，请通过下方邮箱联系我们。",
          en: "You may access, correct, or delete your personal information, withdraw consent, and export your data. Because of the self-sovereign nature of a crypto wallet, deleting your account does not affect public transaction records already on-chain. To exercise these rights, contact us at the email below.",
        },
      ],
    },
    {
      heading: { zh: "7. 儿童隐私", en: "7. Children's Privacy" },
      paragraphs: [
        {
          zh: "cowallet 不面向 18 岁以下的用户，我们不会有意收集未成年人的个人信息。",
          en: "cowallet is not directed to anyone under 18, and we do not knowingly collect personal information from minors.",
        },
      ],
    },
    {
      heading: { zh: "8. 政策变更", en: "8. Changes to This Policy" },
      paragraphs: [
        {
          zh: "我们可能不时更新本政策。重大变更将通过 App 内通知或本页顶部的更新日期告知。",
          en: "We may update this policy from time to time. Material changes will be communicated via in-app notice or the updated date at the top of this page.",
        },
      ],
    },
    {
      heading: { zh: "9. 联系我们", en: "9. Contact Us" },
      paragraphs: [
        {
          zh: "如对本隐私政策有任何疑问，请联系：privacy@cowallet.app",
          en: "For any questions about this Privacy Policy, contact: privacy@cowallet.app",
        },
      ],
    },
  ],
};

export const termsContent: LegalContent = {
  title: { zh: "服务条款", en: "Terms of Service" },
  updated: {
    zh: `最后更新：${EFFECTIVE_DATE}`,
    en: `Last updated: ${EFFECTIVE_DATE}`,
  },
  intro: {
    zh: "欢迎使用 cowallet。使用本 App 及相关服务即表示你同意以下条款。请仔细阅读，尤其是关于风险、责任限制与仲裁的部分。",
    en: "Welcome to cowallet. By using this app and related services you agree to the following terms. Please read them carefully, especially the sections on risk, limitation of liability, and dispute resolution.",
  },
  sections: [
    {
      heading: { zh: "1. 服务说明", en: "1. Description of Service" },
      paragraphs: [
        {
          zh: "cowallet 是一款基于多方计算（MPC）门限签名的加密货币钱包，支持 AI 辅助的资产管理。你通过设备分片、云端分片与找回码分片共同控制资产，至少两份分片同意方可完成签名。",
          en: "cowallet is a cryptocurrency wallet built on Multi-Party Computation (MPC) threshold signatures, with AI-assisted asset management. You control your assets through a device shard, a cloud shard, and a recovery shard; signing requires at least two shards to agree.",
        },
      ],
    },
    {
      heading: { zh: "2. 你的责任", en: "2. Your Responsibilities" },
      paragraphs: [
        {
          zh: "• 你须妥善保管你的设备、生物识别凭据以及找回码。丢失访问凭据可能导致资产无法找回。",
          en: "• You are responsible for safeguarding your device, biometric credentials, and recovery information. Losing access credentials may result in permanently unrecoverable assets.",
        },
        {
          zh: "• 你须对通过本 App 发起的所有交易负责，包括通过 AI 助手授权的交易。请在确认前核对每笔交易。",
          en: "• You are responsible for all transactions initiated through the app, including those authorized via the AI assistant. Review every transaction before confirming.",
        },
        {
          zh: "• 你须遵守所在司法辖区适用的法律法规，不得将本服务用于洗钱、欺诈或其他非法目的。",
          en: "• You must comply with the laws and regulations of your jurisdiction and must not use the service for money laundering, fraud, or any other unlawful purpose.",
        },
      ],
    },
    {
      heading: { zh: "3. 非托管性质", en: "3. Non-Custodial Nature" },
      paragraphs: [
        {
          zh: "cowallet 无法单独动用你的资产，也无法为你重建完整私钥。我们不是银行或托管方，不持有你资产的所有权。链上交易一经广播即不可逆转。",
          en: "cowallet cannot move your assets alone and cannot reconstruct your full private key for you. We are not a bank or a custodian and do not take ownership of your assets. On-chain transactions are irreversible once broadcast.",
        },
      ],
    },
    {
      heading: { zh: "4. AI 功能", en: "4. AI Features" },
      paragraphs: [
        {
          zh: "AI 助手旨在辅助你理解与操作，但可能产生不准确的结果。AI 仅能在你设定的策略边界（如限额、收款方）内行动。最终决定权始终在你——请在签名前独立核实。",
          en: "The AI assistant is designed to help you understand and act, but it may produce inaccurate results. The AI can only act within the policy boundaries you set (such as limits and recipients). The final decision is always yours — verify independently before signing.",
        },
      ],
    },
    {
      heading: { zh: "5. 风险提示", en: "5. Risk Disclosure" },
      paragraphs: [
        {
          zh: "加密资产波动剧烈，可能大幅贬值甚至归零。区块链交易不可撤销；智能合约可能存在漏洞；网络拥堵可能导致延迟或手续费上涨。你应仅投入你能承受损失的资金。",
          en: "Crypto assets are highly volatile and may lose substantial value or become worthless. Blockchain transactions are irreversible; smart contracts may contain bugs; network congestion may cause delays or higher fees. Only commit funds you can afford to lose.",
        },
      ],
    },
    {
      heading: { zh: "6. 责任限制", en: "6. Limitation of Liability" },
      paragraphs: [
        {
          zh: "在适用法律允许的最大范围内，cowallet 及其关联方不对因使用或无法使用本服务、市场波动、第三方服务故障或你自身操作而产生的任何间接、附带或后果性损失承担责任。本服务按“现状”提供，不作任何明示或暗示的保证。",
          en: "To the maximum extent permitted by law, cowallet and its affiliates are not liable for any indirect, incidental, or consequential losses arising from your use of or inability to use the service, market volatility, third-party service failures, or your own actions. The service is provided \"as is,\" without warranties of any kind, express or implied.",
        },
      ],
    },
    {
      heading: { zh: "7. 费用", en: "7. Fees" },
      paragraphs: [
        {
          zh: "使用本服务可能产生区块链网络手续费（Gas）及第三方服务费用。任何由 cowallet 收取的费用将在相关操作前明确告知。",
          en: "Using the service may incur blockchain network fees (gas) and third-party service costs. Any fees charged by cowallet will be disclosed before the relevant action.",
        },
      ],
    },
    {
      heading: { zh: "8. 服务变更与终止", en: "8. Changes & Termination" },
      paragraphs: [
        {
          zh: "我们可能随时修改、暂停或终止服务的部分或全部功能。因本服务的非托管性质，即使服务终止，你仍可凭借你的分片与找回机制访问资产。",
          en: "We may modify, suspend, or discontinue any part of the service at any time. Because of the non-custodial design, you can still access your assets through your shards and recovery mechanism even if the service is discontinued.",
        },
      ],
    },
    {
      heading: { zh: "9. 争议解决", en: "9. Dispute Resolution" },
      paragraphs: [
        {
          zh: "本条款适用相关司法辖区的法律。因本条款产生的争议应首先通过友好协商解决；协商不成的，依适用法律通过约定的仲裁或法院程序解决。",
          en: "These terms are governed by the laws of the applicable jurisdiction. Disputes arising from these terms should first be resolved through good-faith negotiation; failing that, through the agreed arbitration or court process under applicable law.",
        },
      ],
    },
    {
      heading: { zh: "10. 联系我们", en: "10. Contact Us" },
      paragraphs: [
        {
          zh: "如对本服务条款有任何疑问，请联系：legal@cowallet.app",
          en: "For any questions about these Terms of Service, contact: legal@cowallet.app",
        },
      ],
    },
  ],
};
