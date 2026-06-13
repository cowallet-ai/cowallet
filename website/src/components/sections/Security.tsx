"use client";

import { useLang } from "@/context/LangContext";

const keys = [
  {
    id: "phone",
    color: "bg-success-soft border-[#c9d7bc]",
    iconColor: "text-success",
    badge: { text: "OK", color: "bg-success-soft text-success border-[#c9d7bc]" },
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
        <rect x="6" y="3" width="12" height="18" rx="2" />
        <circle cx="12" cy="17" r="1" />
      </svg>
    ),
  },
  {
    id: "cloud",
    color: "bg-success-soft border-[#c9d7bc]",
    iconColor: "text-success",
    badge: { text: "OK", color: "bg-success-soft text-success border-[#c9d7bc]" },
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
        <path d="M18 10a4 4 0 0 0 0-8H6a4 4 0 0 0 0 8M2 14a4 4 0 0 1 4-4h12a4 4 0 0 1 0 8H6a4 4 0 0 1-4-4z" />
      </svg>
    ),
  },
  {
    id: "recovery",
    color: "bg-warn-soft border-[#e4d2a8]",
    iconColor: "text-warn",
    badge: { text: "!", color: "bg-warn-soft text-warn border-[#e4d2a8]" },
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
        <circle cx="12" cy="16" r="1" />
        <path d="M7 11V7a5 5 0 0 1 10 0v4M5 11h14v10H5z" />
      </svg>
    ),
  },
];

export function Security() {
  const { t } = useLang();

  return (
    <section className="py-24 px-6 bg-paper-subtle">
      <div className="max-w-5xl mx-auto">
        <div className="text-center mb-16">
          <h2 className="font-serif font-medium text-3xl md:text-4xl text-ink-1 tracking-tight mb-4">
            {t("security.title")}
          </h2>
          <p className="text-[15px] text-ink-2 max-w-lg mx-auto">
            {t("security.sub")}
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-5 mb-12">
          {keys.map((k) => (
            <div
              key={k.id}
              className={`p-5 rounded-[16px] border ${k.color}`}
            >
              <div className="flex items-start gap-3 mb-3">
                <div className={`w-10 h-10 rounded-xl bg-paper-card flex items-center justify-center ${k.iconColor}`}>
                  {k.icon}
                </div>
                <div className="flex-1">
                  <h3 className="font-serif text-[15px] font-medium text-ink-1 leading-snug">
                    {t(`security.${k.id}.title`)}
                  </h3>
                  <p className="text-[12px] text-ink-3 mt-0.5">
                    {t(`security.${k.id}.where`)}
                  </p>
                </div>
                <span className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-mono border ${k.badge.color}`}>
                  <span className={`w-[5px] h-[5px] rounded-full ${k.id === "recovery" ? "bg-warn" : "bg-success"}`} />
                  {k.badge.text}
                </span>
              </div>
            </div>
          ))}
        </div>

        {/* Explanation */}
        <div className="max-w-2xl mx-auto bg-paper-card border border-line rounded-[16px] p-6 mb-5">
          <p className="text-[14px] text-ink-2 leading-relaxed">
            {t("security.explain")}
          </p>
        </div>

        {/* Tech detail */}
        <div className="max-w-2xl mx-auto bg-paper-card border border-line rounded-[16px] p-5">
          <div className="flex items-center justify-between mb-3">
            <span className="font-mono text-[10px] tracking-[0.12em] uppercase text-ink-3">
              Tech detail
            </span>
            <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-[11px] font-mono border border-line-strong text-ink-2 bg-paper-card">
              MPC 2-of-3
            </span>
          </div>
          <p className="text-[12.5px] text-ink-3 leading-relaxed">
            {t("security.tech")}
          </p>
        </div>
      </div>
    </section>
  );
}
