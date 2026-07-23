"use client";

import { useLang } from "@/context/LangContext";
import { Button } from "@/components/ui/Button";
import { DOWNLOAD_LINKS, GOOGLE_PLAY_AVAILABLE } from "@/lib/download";

type Variant = "primary" | "accent" | "outline" | "ghost";
type Size = "default" | "lg";

const ICON_CLASS = "w-[1.05em] h-[1.05em] shrink-0";

function AppleIcon() {
  return (
    <svg className={ICON_CLASS} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M17.05 12.54c-.03-2.62 2.14-3.88 2.24-3.94-1.22-1.79-3.12-2.03-3.8-2.06-1.62-.16-3.16.95-3.98.95-.82 0-2.09-.93-3.44-.9-1.77.03-3.4 1.03-4.31 2.61-1.84 3.19-.47 7.9 1.32 10.49.87 1.27 1.91 2.69 3.28 2.64 1.32-.05 1.81-.85 3.41-.85 1.59 0 2.04.85 3.43.82 1.42-.02 2.31-1.29 3.17-2.57 1-1.47 1.41-2.9 1.43-2.97-.03-.01-2.74-1.05-2.77-4.17M14.44 4.73c.73-.88 1.22-2.11 1.08-3.33-1.05.04-2.32.7-3.07 1.58-.67.78-1.26 2.03-1.1 3.22 1.17.09 2.36-.6 3.09-1.47" />
    </svg>
  );
}

function AndroidIcon() {
  return (
    <svg className={ICON_CLASS} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M17.6 9.48l1.84-3.18a.38.38 0 0 0-.14-.52.38.38 0 0 0-.52.14l-1.86 3.22a11.4 11.4 0 0 0-8.84 0L6.22 5.92a.38.38 0 0 0-.52-.14.38.38 0 0 0-.14.52L7.4 9.48A10.8 10.8 0 0 0 2 18h20a10.8 10.8 0 0 0-5.4-8.52M7 15.25a1.06 1.06 0 1 1 0-2.12 1.06 1.06 0 0 1 0 2.12m10 0a1.06 1.06 0 1 1 0-2.12 1.06 1.06 0 0 1 0 2.12" />
    </svg>
  );
}

function GooglePlayIcon() {
  return (
    <svg className={ICON_CLASS} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M3.6 2.3a1 1 0 0 0-.6.92v17.56a1 1 0 0 0 .6.92l10.2-9.7zM15 11.03l3.02-2.87 3.5 1.97a1.06 1.06 0 0 1 0 1.85l-3.5 1.97L15 12.97l1.03-.97zM4.5 2.06l10.5 5.9-2.9 2.76zm0 19.88l7.6-8.66 2.9 2.76z" />
    </svg>
  );
}

interface DownloadButtonsProps {
  /** Variant for the primary (iOS) button. */
  iosVariant?: Variant;
  /** Variant for the Android button. */
  androidVariant?: Variant;
  size?: Size;
  className?: string;
}

export function DownloadButtons({
  iosVariant = "primary",
  androidVariant = "outline",
  size = "lg",
  className = "",
}: DownloadButtonsProps) {
  const { t } = useLang();

  return (
    <div className={`flex flex-col sm:flex-row gap-3 justify-center ${className}`}>
      <a
        href={DOWNLOAD_LINKS.ios}
        target="_blank"
        rel="noopener noreferrer"
      >
        <Button variant={iosVariant} size={size} className="whitespace-nowrap">
          <AppleIcon />
          {t("store.ios")}
        </Button>
      </a>

      <a href={DOWNLOAD_LINKS.android} download>
        <Button variant={androidVariant} size={size} className="whitespace-nowrap">
          <AndroidIcon />
          {t("store.android")}
        </Button>
      </a>

      {GOOGLE_PLAY_AVAILABLE ? (
        <a
          href={DOWNLOAD_LINKS.googlePlay ?? undefined}
          target="_blank"
          rel="noopener noreferrer"
        >
          <Button variant={androidVariant} size={size} className="whitespace-nowrap">
            <GooglePlayIcon />
            {t("store.googleplay")}
          </Button>
        </a>
      ) : (
        <Button
          variant="ghost"
          size={size}
          disabled
          className="whitespace-nowrap opacity-50 cursor-not-allowed"
        >
          <GooglePlayIcon />
          {t("store.googleplay")}
        </Button>
      )}
    </div>
  );
}
