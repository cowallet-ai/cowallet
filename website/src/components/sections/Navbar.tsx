"use client";

import { useState, useEffect } from "react";
import { useLang } from "@/context/LangContext";
import { LangToggle } from "@/components/ui/LangToggle";
import { Button } from "@/components/ui/Button";

export function Navbar() {
  const { t } = useLang();
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  return (
    <nav
      className={`fixed top-0 left-0 right-0 z-50 transition-all duration-200 ${
        scrolled
          ? "bg-paper/80 backdrop-blur-xl border-b border-line shadow-sm"
          : "bg-transparent"
      }`}
    >
      <div className="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between">
        {/* Logo */}
        <div className="flex items-center gap-2.5">
          <span className="w-2 h-2 rounded-full bg-accent shadow-[0_0_0_3px_var(--color-accent-soft)] animate-[pulse-dot_2.4s_ease-in-out_infinite]" />
          <span className="font-serif-en font-semibold text-[17px] text-ink-2 tracking-tight">
            cowallet<span className="text-accent">.ai</span>
          </span>
        </div>

        {/* Right side */}
        <div className="flex items-center gap-3">
          <LangToggle />
          <a href="https://static.catwallet.ai/app/app-release.apk" download>
            <Button variant="accent" size="default" pill className="hidden sm:inline-flex">
              {t("nav.download")}
            </Button>
          </a>
        </div>
      </div>
    </nav>
  );
}
