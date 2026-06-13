"use client";

import { useLang } from "@/context/LangContext";
import { LangToggle } from "@/components/ui/LangToggle";

export function Footer() {
  const { t } = useLang();

  return (
    <footer className="py-12 px-6 border-t border-line">
      <div className="max-w-5xl mx-auto flex flex-col md:flex-row items-center justify-between gap-6">
        <div className="text-center md:text-left">
          <div className="font-mono text-[11px] tracking-[0.12em] uppercase text-ink-3 mb-1">
            cowallet · 2026
          </div>
          <div className="text-[13px] text-ink-3 font-serif">
            {t("footer.tagline")}
          </div>
        </div>

        <div className="flex items-center gap-6">
          <a href="#" className="text-[12px] text-ink-3 hover:text-ink-1 transition-colors">
            {t("footer.privacy")}
          </a>
          <a href="#" className="text-[12px] text-ink-3 hover:text-ink-1 transition-colors">
            {t("footer.terms")}
          </a>
          <a href="#" className="text-[12px] text-ink-3 hover:text-ink-1 transition-colors">
            GitHub
          </a>
          <LangToggle />
        </div>
      </div>
    </footer>
  );
}
