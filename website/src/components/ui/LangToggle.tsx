"use client";

import { useLang } from "@/context/LangContext";

export function LangToggle({ className = "" }: { className?: string }) {
  const { lang, setLang } = useLang();

  return (
    <div className={`flex border border-line-strong rounded-[999px] p-0.5 bg-paper ${className}`}>
      <button
        onClick={() => setLang("en")}
        className={`px-3 py-1 text-[11px] font-mono tracking-wider rounded-[999px] cursor-pointer transition-colors ${
          lang === "en" ? "bg-ink-1 text-paper" : "text-ink-3"
        }`}
      >
        EN
      </button>
      <button
        onClick={() => setLang("zh")}
        className={`px-3 py-1 text-[11px] font-mono tracking-wider rounded-[999px] cursor-pointer transition-colors ${
          lang === "zh" ? "bg-ink-1 text-paper" : "text-ink-3"
        }`}
      >
        中
      </button>
    </div>
  );
}
