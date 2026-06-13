"use client";

import { useLang } from "@/context/LangContext";

const features = [
  {
    key: "security" as const,
    color: "bg-success",
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" className="w-5 h-5">
        <path d="M12 2l8 4v6c0 5-3.5 9-8 10-4.5-1-8-5-8-10V6l8-4z" />
        <path d="M9 12l2 2 4-4" />
      </svg>
    ),
  },
  {
    key: "global" as const,
    color: "bg-info",
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" className="w-5 h-5">
        <circle cx="12" cy="12" r="10" />
        <path d="M2 12h20M12 2a15 15 0 0 1 0 20 15 15 0 0 1 0-20z" />
      </svg>
    ),
  },
  {
    key: "ai" as const,
    color: "bg-accent",
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" className="w-5 h-5">
        <path d="M12 2v4M12 18v4M4 12H2M22 12h-2M6 6l-2-2M20 20l-2-2M6 18l-2 2M20 4l-2 2" />
        <circle cx="12" cy="12" r="4" />
      </svg>
    ),
  },
];

export function Features() {
  const { t } = useLang();

  return (
    <section className="py-24 px-6">
      <div className="max-w-5xl mx-auto">
        <h2 className="font-serif font-medium text-3xl md:text-4xl text-center text-ink-1 tracking-tight mb-16">
          {t("features.title")}
        </h2>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {features.map((f) => (
            <div
              key={f.key}
              className="flex flex-col items-start p-6 bg-paper-card border border-line rounded-[16px] hover:border-line-strong hover:shadow-sm transition-all"
            >
              <div
                className={`w-11 h-11 rounded-xl ${f.color} text-white flex items-center justify-center shadow-[0_4px_10px_-2px_rgba(20,16,8,0.18)] mb-4`}
              >
                {f.icon}
              </div>
              <h3 className="font-serif font-medium text-[17px] text-ink-1 mb-2 leading-snug">
                {t(`features.${f.key}.title`)}
              </h3>
              <p className="text-[13.5px] text-ink-2 leading-relaxed">
                {t(`features.${f.key}.desc`)}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
