import type { Metadata } from "next";
import { LegalArticle } from "@/components/legal/LegalArticle";
import { termsContent } from "@/lib/legal";

export const metadata: Metadata = {
  title: "服务条款 · cowallet",
  description: "cowallet 服务条款 — Terms of Service for the AI-native MPC crypto wallet.",
};

export default function TermsPage() {
  return <LegalArticle content={termsContent} />;
}
