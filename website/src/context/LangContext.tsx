"use client";

import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from "react";
import { i18n } from "@/lib/i18n";

type Lang = "zh" | "en";

interface LangContextValue {
  lang: Lang;
  setLang: (lang: Lang) => void;
  t: (key: string) => string;
}

const LangContext = createContext<LangContextValue | null>(null);

export function LangProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>("zh");

  useEffect(() => {
    const saved = localStorage.getItem("cowallet-lang") as Lang | null;
    if (saved === "en" || saved === "zh") {
      setLangState(saved);
    }
  }, []);

  const setLang = useCallback((newLang: Lang) => {
    setLangState(newLang);
    localStorage.setItem("cowallet-lang", newLang);
    document.documentElement.lang = newLang === "zh" ? "zh" : "en";
  }, []);

  const t = useCallback(
    (key: string): string => {
      const entry = (i18n as Record<string, Record<string, string>>)[key];
      if (!entry) return key;
      return entry[lang] || entry.zh || key;
    },
    [lang]
  );

  return (
    <LangContext.Provider value={{ lang, setLang, t }}>
      {children}
    </LangContext.Provider>
  );
}

export function useLang(): LangContextValue {
  const ctx = useContext(LangContext);
  if (!ctx) throw new Error("useLang must be used within LangProvider");
  return ctx;
}
