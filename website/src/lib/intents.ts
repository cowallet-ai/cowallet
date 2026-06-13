export interface IntentRule {
  re_zh: RegExp;
  re_en: RegExp;
  kind: string;
  title: { zh: string; en: string };
  sub: { zh: string; en: string };
  yes: { zh: string; en: string };
  no: { zh: string; en: string };
}

export const INTENT_RULES: IntentRule[] = [
  {
    re_zh: /(闲着|闲放|放哪|存起来|赚利息|生息|理财|没用)/,
    re_en: /(idle|sitting|park|save|earn interest|yield)/i,
    kind: "savings",
    title: { zh: "给你闲着的钱找个赚利息的地方", en: "Park your idle money so it earns" },
    sub: { zh: "推荐 Aave V3 (4.82%) 或 美国国债代币 (5.20%)。", en: "Suggest Aave V3 (4.82%) or US T-Bill token (5.20%)." },
    yes: { zh: "对，看看", en: "Yes, show me" },
    no: { zh: "不是这意思", en: "Not that" },
  },
  {
    re_zh: /(老婆|妻子|生日|Sarah|sarah)/,
    re_en: /(wife|sarah|birthday)/i,
    kind: "transfer",
    title: { zh: "给 Sarah (你老婆) 转 $1000 USDC", en: "Send $1000 USDC to Sarah (your wife)" },
    sub: { zh: "3 周前你给她转过。本次约 7,200 人民币。", en: "You sent her money 3 weeks ago. ~7,200 RMB." },
    yes: { zh: "对，转", en: "Yes, send" },
    no: { zh: "换个人", en: "Different person" },
  },
  {
    re_zh: /(花了多少|这个月.*花|支出|开销|花销)/,
    re_en: /(how much.*spen|this month.*spen|expense|spending)/i,
    kind: "spending",
    title: { zh: "你这个月花了 $2,847", en: "You've spent $2,847 this month" },
    sub: { zh: "订阅 $140 · 餐饮 $623 · 转账给朋友 $1,200 · 其他 $884。比上月少 12%。", en: "Subs $140 · Food $623 · Friends $1,200 · Other $884. 12% less than last month." },
    yes: { zh: "看详情", en: "See details" },
    no: { zh: "不对", en: "Not that" },
  },
  {
    re_zh: /(余额|总共.*多少|放哪|钱都|资产)/,
    re_en: /(balance|total|where.*money|how much.*have)/i,
    kind: "balance",
    title: { zh: "总共 $48,280", en: "Total: $48,280" },
    sub: { zh: "USDC 28,450 · ETH 16,830 · stETH 3,000。今天 +$392 (+0.82%)。", en: "USDC 28,450 · ETH 16,830 · stETH 3,000. Today +$392 (+0.82%)." },
    yes: { zh: "看钱包", en: "Open wallet" },
    no: { zh: "不要", en: "Nope" },
  },
];

export function detectIntent(text: string, lang: "zh" | "en"): IntentRule | null {
  const t = text.trim();
  if (!t) return null;
  for (const r of INTENT_RULES) {
    const re = lang === "en" ? r.re_en : r.re_zh;
    if (re.test(t)) return r;
  }
  return null;
}
