// Centralized download / store links so every CTA points to the same targets.

export const DOWNLOAD_LINKS = {
  ios: "https://apps.apple.com/sg/app/cowallet-ai/id6769661470",
  android: "https://static.catwallet.ai/app/app-release.apk",
  // Google Play listing is not live yet — reserved for future release.
  googlePlay: null as string | null,
} as const;

export const GOOGLE_PLAY_AVAILABLE = DOWNLOAD_LINKS.googlePlay !== null;
