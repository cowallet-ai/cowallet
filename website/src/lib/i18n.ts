export const i18n = {
  // Navbar
  "nav.download": { zh: "下载 App", en: "Download" },

  // Hero
  "hero.kicker": { zh: "数字钱包 · 会听懂人话", en: "Digital wallet · speaks your language" },
  "hero.headline": { zh: "会听你说话的", en: "A wallet that actually" },
  "hero.headline.em": { zh: "钱包。", en: "listens." },
  "hero.sub": {
    zh: '就像给你家请了个管家——你说"帮我转 100 块给小明"，它就去做；你不会说也没关系，它有按钮。',
    en: "Like hiring a butler for your money — say \"send $100 to Sarah\" and it does it. Don’t feel like talking? Buttons work too.",
  },
  "hero.cta.primary": { zh: "开始使用", en: "Get started" },
  "hero.cta.secondary": { zh: "了解更多", en: "Learn more" },

  // Features
  "features.title": { zh: "为什么选 cowallet", en: "Why cowallet" },
  "features.security.title": { zh: "只有你能动你的钱", en: "Only you can move your money" },
  "features.security.desc": {
    zh: "我们也进不来。三份钥匙分开保管，谁也拿不到完整的。",
    en: "Not even us. Three key pieces stored separately — nobody holds the whole thing.",
  },
  "features.global.title": { zh: "100+ 个金融网络", en: "100+ financial networks" },
  "features.global.desc": {
    zh: "全世界通用。以太坊、Base、Arbitrum、Polygon……一个钱包全搞定。",
    en: "Works worldwide. Ethereum, Base, Arbitrum, Polygon… one wallet for all.",
  },
  "features.ai.title": { zh: "AI 帮你跑腿", en: "AI does the errands" },
  "features.ai.desc": {
    zh: "你只需说一句话。转账、查余额、理财建议——说人话就行。",
    en: "Just say the word. Transfers, balances, yield advice — in plain language.",
  },

  // AI Showcase
  "ai.title": { zh: "AI 原生体验", en: "AI-native experience" },
  "ai.reads.title": { zh: "AI 看懂交易", en: "AI reads the contract" },
  "ai.reads.desc": {
    zh: "签字前用人话告诉你这笔交易到底在干什么。再也不会稀里糊涂授权。",
    en: "Explains what a transaction actually does before you sign. No more blind approvals.",
  },
  "ai.transfer.title": { zh: "说话就能转账", en: "Transfer by speaking" },
  "ai.transfer.desc": {
    zh: '"给老婆转 1000 块"——它会先复述确认，你点头才动。',
    en: '"Send $1000 to my wife" — it confirms what it heard before moving anything.',
  },
  "ai.autopay.title": { zh: "让 AI 替你付款", en: "Let AI pay for you" },
  "ai.autopay.desc": {
    zh: "你定规矩（花多少、给谁），AI 在边界内自动办。越界就停。",
    en: "You set the rules (how much, to whom). AI acts within boundaries. Crosses a line, it stops.",
  },

  // Security
  "security.title": { zh: "三份钥匙都点头才能动钱", en: "All three keys must agree to move money" },
  "security.sub": {
    zh: "没人能单独动你的钱——连 cowallet 公司都进不来。",
    en: "Nobody can move your money alone — not even cowallet.",
  },
  "security.phone.title": { zh: "手机里那份", en: "On this phone" },
  "security.phone.where": { zh: "在你手机的安全区里", en: "In your phone's secure enclave" },
  "security.cloud.title": { zh: "云端那份", en: "In the cloud" },
  "security.cloud.where": { zh: "Anchorage 托管 · 美国合规机构", en: "Anchorage · US-regulated custodian" },
  "security.recovery.title": { zh: "找回码那份", en: "Recovery piece" },
  "security.recovery.where": { zh: "在你那儿（或没设）", en: "In your hands (or unset)" },
  "security.explain": {
    zh: "想象家门有三把锁——你身上一把，信任的帮手（云端）一把，保险柜里一把（找回码）。开门必须凑齐两把以上。就算 cowallet 公司被黑，他们也只拿到一把，开不了你的门。",
    en: "Imagine three locks on your door. You hold one. A trusted helper (cloud) holds one. A safe holds one (recovery). Opening needs at least two. Even if cowallet gets hacked, attackers have only one — your door stays shut.",
  },
  "security.tech": {
    zh: "门限签名 (TSS) · 设备端用 Secure Enclave，云端由 Anchorage Digital 托管（SOC2/ISO27001）。任一分片不产生完整私钥。",
    en: "Threshold signatures (TSS). Device share in Secure Enclave; cloud share at Anchorage Digital (SOC2/ISO27001). No share reconstructs the full private key.",
  },

  // Protocols
  "protocols.title": { zh: "开发者接口", en: "Developer protocols" },
  "protocols.docs": { zh: "查看文档", en: "View docs" },

  // CTA
  "cta.title": { zh: "准备好了吗？", en: "Ready?" },
  "cta.sub": { zh: "下载 cowallet，让钱包听懂你说话。", en: "Download cowallet. A wallet that speaks your language." },
  "cta.appstore": { zh: "App Store", en: "App Store" },
  "cta.playstore": { zh: "下载 Android", en: "Download Android" },

  // Footer
  "footer.tagline": { zh: "会听懂人话的钱包", en: "A wallet that listens" },
  "footer.privacy": { zh: "隐私政策", en: "Privacy" },
  "footer.terms": { zh: "服务条款", en: "Terms" },

  // Phone simulator
  "phone.demo.start": { zh: "自动演示", en: "Auto demo" },
  "phone.demo.exit": { zh: "退出演示", en: "Exit demo" },
} as const;
