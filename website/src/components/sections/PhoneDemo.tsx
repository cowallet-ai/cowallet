"use client";

import { useLang } from "@/context/LangContext";
import { PhoneSimulator } from "@/components/phone/PhoneSimulator";

export function PhoneDemo() {
  const { lang } = useLang();

  return (
    <div className="w-[300px] md:w-[320px] lg:w-[340px] flex flex-col items-center gap-4">
      <PhoneSimulator />
      <p className="text-ink-3 text-[11px] font-mono tracking-wider text-center">
        {lang === "en" ? "Interactive — try tapping around" : "可交互 — 试试点点看"}
      </p>
    </div>
  );
}
