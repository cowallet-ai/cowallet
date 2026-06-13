"use client";

import { useLang } from "@/context/LangContext";
import { Orb } from "@/components/ui/Orb";
import { Button } from "@/components/ui/Button";
import { PhoneDemo } from "./PhoneDemo";

export function Hero() {
  const { t } = useLang();

  return (
    <section className="relative min-h-screen flex items-center overflow-hidden pt-16">
      {/* Background gradient */}
      <div
        className="absolute inset-0 -z-10"
        style={{
          background:
            "radial-gradient(ellipse 70% 50% at 50% 0%, #ede4ce 0%, transparent 60%), radial-gradient(ellipse 60% 40% at 50% 100%, #ede4ce 0%, transparent 60%), #faf9f5",
        }}
      />

      <div className="max-w-6xl mx-auto px-6 w-full">
        <div className="flex flex-col lg:flex-row items-center gap-12 lg:gap-16">
          {/* Left: Text */}
          <div className="flex-1 text-center lg:text-left pt-8 lg:pt-0">
            <p className="font-mono text-[10.5px] tracking-[0.22em] uppercase text-accent mb-4">
              {t("hero.kicker")}
            </p>

            <h1 className="font-serif font-medium text-4xl md:text-5xl lg:text-[56px] leading-[1.1] tracking-tight text-ink-1 mb-5">
              {t("hero.headline")}
              <br />
              <em className="font-serif-en italic text-accent font-medium">
                {t("hero.headline.em")}
              </em>
            </h1>

            <p className="text-[15px] leading-relaxed text-ink-2 max-w-md mx-auto lg:mx-0 mb-8">
              {t("hero.sub")}
            </p>

            <div className="flex flex-col sm:flex-row gap-3 justify-center lg:justify-start">
              <a href="https://static.catwallet.ai/app/app-release.apk" download>
                <Button variant="accent" size="lg">
                  {t("hero.cta.primary")}
                </Button>
              </a>
              <Button variant="outline" size="lg">
                {t("hero.cta.secondary")}
              </Button>
            </div>
          </div>

          {/* Right: Phone simulator */}
          <div className="flex-1 flex justify-center lg:justify-end">
            <div className="relative">
              <Orb size={300} className="opacity-20 absolute -top-10 -right-10 -z-10" />
              <PhoneDemo />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
