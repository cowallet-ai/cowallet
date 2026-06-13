"use client";

import { useLang } from "@/context/LangContext";

const scenarios = [
  {
    key: "reads" as const,
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" className="w-6 h-6">
        <circle cx="12" cy="12" r="3" />
        <path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7z" />
      </svg>
    ),
    color: "from-accent to-accent-hover",
  },
  {
    key: "transfer" as const,
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" className="w-6 h-6">
        <rect x="9" y="2" width="6" height="12" rx="3" />
        <path d="M5 10v2a7 7 0 0 0 14 0v-2M12 19v3" />
      </svg>
    ),
    color: "from-info to-[#2d5a7a]",
  },
  {
    key: "autopay" as const,
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className="w-6 h-6">
        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
      </svg>
    ),
    color: "from-success to-[#4a6640]",
  },
];

export function AiShowcase() {
  const { t } = useLang();

  return (
    <section className="py-24 px-6">
      <div className="max-w-5xl mx-auto">
        <h2 className="font-serif font-medium text-3xl md:text-4xl text-center text-ink-1 tracking-tight mb-16">
          {t("ai.title")}
        </h2>

        <div className="flex flex-col gap-20">
          {scenarios.map((s, i) => (
            <div
              key={s.key}
              className={`flex flex-col ${i % 2 === 0 ? "md:flex-row" : "md:flex-row-reverse"} items-center gap-10 md:gap-16`}
            >
              {/* Illustration */}
              <div className="flex-1 flex justify-center">
                <div className="w-[280px] h-[200px] bg-paper-card border border-line rounded-[20px] flex items-center justify-center relative overflow-hidden">
                  <div className={`absolute inset-0 bg-gradient-to-br ${s.color} opacity-5`} />
                  <div className={`w-16 h-16 rounded-2xl bg-gradient-to-br ${s.color} text-white flex items-center justify-center shadow-lg`}>
                    {s.icon}
                  </div>
                </div>
              </div>

              {/* Text */}
              <div className="flex-1 text-center md:text-left">
                <h3 className="font-serif font-medium text-xl md:text-2xl text-ink-1 mb-3 leading-snug">
                  {t(`ai.${s.key}.title`)}
                </h3>
                <p className="text-[14.5px] text-ink-2 leading-relaxed max-w-md mx-auto md:mx-0">
                  {t(`ai.${s.key}.desc`)}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
