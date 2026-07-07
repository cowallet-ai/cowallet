"use client";

import Link from "next/link";
import { useLang } from "@/context/LangContext";
import { LangToggle } from "@/components/ui/LangToggle";
import { Footer } from "@/components/sections/Footer";
import type { LegalContent } from "@/lib/legal";

export function LegalArticle({ content }: { content: LegalContent }) {
  const { lang } = useLang();

  return (
    <>
      {/* Minimal top bar — brand returns home */}
      <nav className="fixed top-0 left-0 right-0 z-50 bg-paper/80 backdrop-blur-xl border-b border-line">
        <div className="max-w-3xl mx-auto px-6 h-16 flex items-center justify-between">
          <Link href="/" className="flex items-center gap-2.5">
            <span className="w-2 h-2 rounded-full bg-accent shadow-[0_0_0_3px_var(--color-accent-soft)]" />
            <span className="font-serif-en font-semibold text-[17px] text-ink-2 tracking-tight">
              cowallet<span className="text-accent">.ai</span>
            </span>
          </Link>
          <LangToggle />
        </div>
      </nav>

      <main className="max-w-3xl mx-auto px-6 pt-32 pb-24">
        <header className="mb-12">
          <h1 className="font-serif font-medium text-4xl md:text-5xl text-ink-1 tracking-tight mb-3">
            {content.title[lang]}
          </h1>
          <p className="font-mono text-[11px] tracking-[0.12em] uppercase text-ink-3">
            {content.updated[lang]}
          </p>
        </header>

        <p className="text-[15px] text-ink-2 leading-relaxed mb-12">
          {content.intro[lang]}
        </p>

        <div className="space-y-10">
          {content.sections.map((section) => (
            <section key={section.heading.en}>
              <h2 className="font-serif font-medium text-xl text-ink-1 tracking-tight mb-4">
                {section.heading[lang]}
              </h2>
              <div className="space-y-3">
                {section.paragraphs.map((p, i) => {
                  const text = p[lang];
                  const isBullet = text.startsWith("• ");
                  return (
                    <p
                      key={i}
                      className={`text-[14.5px] text-ink-2 leading-relaxed ${
                        isBullet ? "pl-4" : ""
                      }`}
                    >
                      {isBullet ? text.slice(2) : text}
                    </p>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      </main>

      <Footer />
    </>
  );
}
