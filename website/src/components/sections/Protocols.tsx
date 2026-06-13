"use client";

import { useLang } from "@/context/LangContext";

const protocols = [
  { name: "MCP", badge: "accent", desc: { zh: "Claude、Cursor 等 MCP host 直连", en: "Claude, Cursor, any MCP host" }, status: "LIVE" },
  { name: "x402", badge: "info", desc: { zh: "AI 按 API 调用次数付钱", en: "AI pays APIs per call" }, status: "LIVE" },
  { name: "REST", badge: "default", desc: { zh: "HTTP API + HMAC 签名", en: "HTTP API + HMAC signing" }, status: "LIVE" },
  { name: "Webhook", badge: "default", desc: { zh: "每笔交易实时通知你的服务器", en: "Push every tx to your server" }, status: "LIVE" },
  { name: "EIP-6963", badge: "default", desc: { zh: "Web3 网站自动认得", en: "Web3 sites recognize automatically" }, status: "LIVE" },
  { name: "A2A", badge: "default", desc: { zh: "助手对助手（Google 提案）", en: "Agent-to-agent (Google draft)" }, status: "soon" },
];

const badgeColors: Record<string, string> = {
  accent: "bg-accent-soft text-accent-hover border-[#e9c7b4]",
  info: "bg-info-soft text-info border-[#c2d4e2]",
  default: "bg-paper-card text-ink-2 border-line-strong",
};

export function Protocols() {
  const { lang, t } = useLang();

  return (
    <section className="py-24 px-6">
      <div className="max-w-3xl mx-auto">
        <div className="flex items-center gap-3 mb-8">
          <h2 className="font-serif font-medium text-2xl text-ink-1 tracking-tight">
            {t("protocols.title")}
          </h2>
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-[11px] font-mono border border-[#c2d4e2] bg-info-soft text-info">
            for devs
          </span>
        </div>

        <div className="bg-paper-card border border-line rounded-[16px] overflow-hidden">
          {protocols.map((p, i) => (
            <div
              key={p.name}
              className={`flex items-center gap-3 px-4 py-3 ${
                i < protocols.length - 1 ? "border-b border-line" : ""
              } ${p.status === "soon" ? "opacity-60" : ""}`}
            >
              <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-[11px] font-mono border ${badgeColors[p.badge]}`}>
                {p.name}
              </span>
              <span className="flex-1 text-[13px] text-ink-2 min-w-0">
                {p.desc[lang]}
              </span>
              <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-mono ${
                p.status === "LIVE"
                  ? "bg-success-soft text-success border border-[#c9d7bc]"
                  : "bg-warn-soft text-warn border border-[#e4d2a8]"
              }`}>
                {p.status === "LIVE" && <span className="w-[5px] h-[5px] rounded-full bg-success" />}
                {p.status}
              </span>
            </div>
          ))}
        </div>

        <div className="mt-4 text-right">
          <a href="#" className="font-mono text-[10px] tracking-[0.1em] uppercase text-accent hover:text-accent-hover">
            {t("protocols.docs")} →
          </a>
        </div>
      </div>
    </section>
  );
}
