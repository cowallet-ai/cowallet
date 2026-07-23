import type { Metadata } from "next";
import { LegalArticle } from "@/components/legal/LegalArticle";
import { privacyContent } from "@/lib/legal";

export const metadata: Metadata = {
  title: "隐私政策 · cowallet",
  description: "cowallet 隐私政策 — Privacy Policy for the AI-native MPC crypto wallet.",
};

export default function PrivacyPage() {
  return <LegalArticle content={privacyContent} />;
}
