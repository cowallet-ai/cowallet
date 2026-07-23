"use client";

import { useLang } from "@/context/LangContext";
import { Orb } from "@/components/ui/Orb";
import { DownloadButtons } from "@/components/ui/DownloadButtons";

export function Cta() {
  const { t } = useLang();

  return (
    <section id="download" className="py-24 px-6 text-center relative overflow-hidden">
      <div className="absolute inset-0 -z-10" style={{
        background: "radial-gradient(ellipse 60% 50% at 50% 50%, #ede4ce 0%, transparent 70%)",
      }} />

      <div className="max-w-xl mx-auto">
        <Orb size={100} className="mx-auto mb-8" />

        <h2 className="font-serif font-medium text-4xl md:text-5xl text-ink-1 tracking-tight mb-4">
          {t("cta.title")}
        </h2>

        <p className="text-[15px] text-ink-2 mb-10 max-w-md mx-auto leading-relaxed">
          {t("cta.sub")}
        </p>

        <DownloadButtons iosVariant="primary" androidVariant="outline" />
      </div>
    </section>
  );
}
